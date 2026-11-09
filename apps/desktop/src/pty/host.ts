import { execFile, execFileSync } from 'node:child_process'
import { randomUUID, timingSafeEqual } from 'node:crypto'
import http from 'node:http'
import type { IPty } from '@lydell/node-pty'
import { WebSocketServer, type WebSocket } from 'ws'
import { PTY_PROTOCOL_PREFIX, type PtyClientMessage, type PtyPreset, type PtyServerMessage, type PtySessionInfo } from '../../../../packages/shared/src/pty'
import {
  encodeControl,
  NdjsonReader,
  parseMainToCore,
  PTY_RESOLVE_TIMEOUT_MS,
  PtyFrameQueue,
  PTY_QUEUE_MAX,
  type PtyCoreToMain,
  type PtyMainToCore,
  type PtyResolved,
  type PtySpawnRequest,
} from '../../../../packages/shared/src/ptyControl'
import { ownersOf, readPidTable } from './owners'
import { clampDim, earlyExitNote, taskkillPath } from './presets'

// Native module ships outside the bundle (resources/pty in a packaged app), so it is required by path.
const pty = require(process.env.CCM_NODE_PTY || '@lydell/node-pty') as typeof import('@lydell/node-pty')

const STDIO = process.env.CCM_PTY_CONTROL === 'stdio'
const IS_WIN = process.platform === 'win32'
const SCROLLBACK_CHARS = 256 * 1024
const FLUSH_MS = 8
const MAX_LIVE = 16
const EXITED_TTL_MS = 30 * 60_000
// Deck's numbers: past ~512 KB unsent the renderer is behind; pausing the PTY lets the agent block instead of us buffering.
const PAUSE_AT = 512 * 1024
const RESUME_AT = 128 * 1024
const EARLY_EXIT_MS = 15_000
let TOKEN = ''
let ORIGINS = new Set<string>()
let initialized = false
let closing = false

interface Session {
  id: string
  preset: PtyPreset
  cwd: string
  /** Folder the shell last reported after a `cd` (validated); preview roots and the branch follow it. */
  liveCwd: string | null
  proc: IPty
  pid: number
  ring: string
  pending: string[]
  timer: NodeJS.Timeout | null
  ws: WebSocket | null
  exitCode: number | null
  exitedAt: number
  paused: boolean
  branch: string | null
  siNonce: string | null
}

interface Frame {
  data: Buffer
  binary: boolean
}

interface Conn {
  ws: WebSocket
  s: Session | null
  queue: PtyFrameQueue<Frame> | null
}

interface Resolving {
  c: Conn
  msg: PtySpawnRequest
  timer: NodeJS.Timeout
}

const sessions = new Map<string, Session>()
const resolving = new Map<string, Resolving>()
const cwdWaits = new Map<string, { ws: WebSocket; id: string }>()
const wtWaits = new Map<string, WebSocket>()

// One bad frame or a dying child must never take every other terminal down with it.
process.on('uncaughtException', (err) => console.error('pty host error:', err))

function control(msg: PtyCoreToMain): void {
  if (STDIO) process.stdout.write(encodeControl(msg))
  else process.parentPort?.postMessage(msg)
}

function tokenOk(candidate: string): boolean {
  if (!TOKEN || !candidate.startsWith(PTY_PROTOCOL_PREFIX)) return false
  const a = Buffer.from(candidate.slice(PTY_PROTOCOL_PREFIX.length))
  const b = Buffer.from(TOKEN)
  return a.length === b.length && timingSafeEqual(a, b)
}

function send(ws: WebSocket | null, msg: PtyServerMessage): void {
  if (ws?.readyState === 1) ws.send(JSON.stringify(msg))
}

const liveCount = (): number => [...sessions.values()].filter((s) => s.exitCode === null).length

function flush(s: Session): void {
  if (s.timer) clearTimeout(s.timer)
  s.timer = null
  if (!s.pending.length) return
  const data = s.pending.join('')
  s.pending = []
  const ws = s.ws
  if (ws?.readyState !== 1) return
  ws.send(Buffer.from(data, 'utf8'), { binary: true })
  if (!s.paused && ws.bufferedAmount > PAUSE_AT) {
    s.paused = true
    s.proc.pause()
    const poll = setInterval(() => {
      if (s.ws === ws && ws.readyState === 1 && ws.bufferedAmount > RESUME_AT) return
      clearInterval(poll)
      s.paused = false
      if (s.exitCode === null) s.proc.resume()
    }, 50)
  }
}

function spawn(r: Extract<PtyResolved, { ok: true }>, preset: PtyPreset, cols: number, rows: number): Session {
  const proc = pty.spawn(r.file, r.args, { name: 'xterm-256color', cols, rows, cwd: r.cwd, env: r.env, useConptyDll: false })
  const startedAt = Date.now()
  const s: Session = { id: r.id, preset, cwd: r.cwd, liveCwd: null, proc, pid: proc.pid, ring: '', pending: [], timer: null, ws: null, exitCode: null, exitedAt: 0, paused: false, branch: null, siNonce: r.si ?? null }
  proc.onData((d) => {
    s.ring += d
    // Amortised trim; a replay may start mid escape sequence, which xterm shrugs off.
    if (s.ring.length > SCROLLBACK_CHARS * 1.5) s.ring = s.ring.slice(-SCROLLBACK_CHARS)
    if (!s.ws) return
    s.pending.push(d)
    s.timer ??= setTimeout(() => flush(s), FLUSH_MS)
  })
  // Flush before announcing exit so the last lines are not lost behind the exit banner.
  proc.onExit(({ exitCode, signal }) => {
    const ms = Date.now() - startedAt
    if (ms < EARLY_EXIT_MS) console.error(earlyExitNote(preset, exitCode, signal, ms, !sessions.has(s.id)))
    flush(s)
    s.exitCode = exitCode
    s.exitedAt = Date.now()
    send(s.ws, { t: 'exit', code: exitCode })
    if (sessions.get(s.id) === s) control({ t: 'exited', id: s.id, code: exitCode })
  })
  // ConPTY hands out the pid asynchronously (0 right after spawn).
  if (!s.pid) {
    const started = Date.now()
    const poll = setInterval(() => {
      if (!proc.pid && Date.now() - started < 5000) return
      clearInterval(poll)
      s.pid = proc.pid
      send(s.ws, { t: 'pid', pid: s.pid })
      if (sessions.get(s.id) === s) control({ t: 'pid', id: s.id, pid: s.pid })
    }, 25)
  }
  sessions.set(s.id, s)
  return s
}

const treePid = (s: Session): number | null => (IS_WIN && s.exitCode === null ? s.proc.pid || s.pid || null : null)

// node-pty's kill only ends the shell; agents and their tool processes would survive it on Windows.
function kill(s: Session, treeKilled = false): void {
  const known = sessions.get(s.id) === s
  sessions.delete(s.id)
  const pid = treeKilled ? null : treePid(s)
  if (pid) execFile(taskkillPath(process.env), ['/T', '/F', '/PID', String(pid)], { windowsHide: true }, () => undefined)
  try {
    s.proc.kill()
  } catch {
    // already gone
  }
  if (known) control({ t: 'removed', id: s.id })
}

function attachTo(ws: WebSocket, s: Session, replay: boolean): void {
  if (s.ws && s.ws !== ws) s.ws.close(4000, 'replaced')
  s.ws = ws
  s.pending = []
  send(ws, { t: 'ready', id: s.id, preset: s.preset, pid: s.pid, cwd: s.cwd, replay, ...(s.siNonce ? { si: s.siNonce } : {}) })
  if (s.liveCwd) send(ws, { t: 'cwd', cwd: s.liveCwd })
  if (replay && s.ring) ws.send(Buffer.from(s.ring, 'utf8'), { binary: true })
  if (s.branch !== null) send(ws, { t: 'branch', branch: s.branch })
  if (s.exitCode !== null) send(ws, { t: 'exit', code: s.exitCode })
}

let pidTable: { at: number; table: Promise<Map<number, number> | null> } | null = null

// One scan serves every pane asking in the same burst; a fresh agent shows up in the next one.
function pidTableNow(): Promise<Map<number, number> | null> {
  if (!pidTable || Date.now() - pidTable.at > 1500) pidTable = { at: Date.now(), table: readPidTable() }
  return pidTable.table
}

async function answerOwners(ws: WebSocket, pids: unknown): Promise<void> {
  const wanted = Array.isArray(pids) ? pids.filter((p): p is number => Number.isInteger(p) && p > 0).slice(0, 64) : []
  const roots = new Map<number, string>()
  for (const s of sessions.values()) if (s.exitCode === null && s.pid) roots.set(s.pid, s.id)
  if (!wanted.length || !roots.size) return send(ws, { t: 'owners', owners: {} })
  const table = await pidTableNow()
  if (!table) return send(ws, { t: 'owners', owners: null })
  send(ws, { t: 'owners', owners: Object.fromEntries([...ownersOf(wanted, roots, table)].map(([pid, id]) => [String(pid), id])) })
}

function settle(c: Conn, s: Session | null): void {
  const queue = c.queue
  const queued = queue?.take() ?? []
  if (queue?.dropped) console.error(`pty input dropped while spawning: ${queue.dropped} frames over ${PTY_QUEUE_MAX} bytes`)
  c.queue = null
  c.s = s
  for (const f of queued) onFrame(c, f)
}

function requestSpawn(c: Conn, msg: PtySpawnRequest): void {
  if (closing) return send(c.ws, { t: 'error', message: 'spawn' })
  c.queue = new PtyFrameQueue<Frame>()
  const rid = randomUUID()
  const timer = setTimeout(() => {
    resolving.delete(rid)
    send(c.ws, { t: 'error', message: 'spawn' })
    settle(c, null)
  }, PTY_RESOLVE_TIMEOUT_MS)
  resolving.set(rid, { c, msg, timer })
  control({ t: 'resolve', rid, msg })
}

function onResolved(msg: PtyResolved): void {
  const r = resolving.get(msg.rid)
  if (!r || closing) {
    if (msg.ok) control({ t: 'removed', id: msg.id })
    if (r) {
      resolving.delete(msg.rid)
      clearTimeout(r.timer)
      send(r.c.ws, { t: 'error', message: 'spawn' })
      settle(r.c, null)
    }
    return
  }
  resolving.delete(msg.rid)
  clearTimeout(r.timer)
  const c = r.c
  if (!msg.ok) {
    send(c.ws, { t: 'error', message: msg.message })
    return settle(c, null)
  }
  const refuse = (message: string): void => {
    send(c.ws, { t: 'error', message })
    control({ t: 'removed', id: msg.id })
    settle(c, null)
  }
  if (c.ws.readyState !== 1) return refuse('spawn')
  if (liveCount() >= MAX_LIVE) return refuse('limit')
  let s: Session
  try {
    s = spawn(msg, r.msg.preset, clampDim(r.msg.cols, 80), clampDim(r.msg.rows, 24))
  } catch (err) {
    console.error('spawn failed:', err)
    return refuse('spawn')
  }
  attachTo(c.ws, s, false)
  control({ t: 'spawned', id: s.id, pid: s.pid })
  settle(c, s)
}

/**
 * The page relays what the shell printed, so it is untrusted: real existing folder only, never a filesystem
 * root (it widens preview roots, like a terminal opened there would).
 */
function reportCwd(ws: WebSocket, s: Session, raw: unknown): void {
  if (typeof raw !== 'string' || raw === '') return send(ws, { t: 'cwd', cwd: null })
  const rid = randomUUID()
  cwdWaits.set(rid, { ws, id: s.id })
  control({ t: 'cwd', rid, id: s.id, cwd: raw })
}

function onCwdOk(rid: string, cwd: string | null): void {
  const w = cwdWaits.get(rid)
  if (!w) return
  cwdWaits.delete(rid)
  send(w.ws, { t: 'cwd', cwd })
  const s = sessions.get(w.id)
  if (cwd && s && cwd !== (s.liveCwd ?? s.cwd)) s.liveCwd = cwd
}

function onText(c: Conn, raw: string): void {
  let msg: PtyClientMessage
  try {
    msg = JSON.parse(raw) as PtyClientMessage
  } catch {
    return
  }
  if (!msg || typeof msg !== 'object') return
  const ws = c.ws
  const current = c.s
  if (msg.t === 'wt.list' || msg.t === 'wt.add' || msg.t === 'wt.remove') {
    const rid = randomUUID()
    wtWaits.set(rid, ws)
    control({ t: 'wt', rid, msg })
    return
  }
  if (msg.t === 'owners') {
    void answerOwners(ws, msg.pids)
    return
  }
  if (msg.t === 'kill' && typeof msg.id === 'string') {
    const target = sessions.get(msg.id)
    if (target) kill(target)
    if (target === current) c.s = null
    return
  }
  if (msg.t === 'spawn' && !current) return requestSpawn(c, msg)
  if (msg.t === 'attach' && !current) {
    const s = sessions.get(String(msg.id))
    if (!s) return send(ws, { t: 'gone' })
    attachTo(ws, s, true)
    c.s = s
    return
  }
  if (!current) return
  if (msg.t === 'resize' && current.exitCode === null) {
    try {
      current.proc.resize(clampDim(msg.cols, 80), clampDim(msg.rows, 24))
    } catch {
      // process exited between the check and the call
    }
  } else if (msg.t === 'kill') {
    kill(current)
    c.s = null
  } else if (msg.t === 'cwd') {
    reportCwd(ws, current, msg.cwd)
  }
}

function onFrame(c: Conn, f: Frame): void {
  if (c.queue) return void c.queue.push(f, f.data.length, f.binary)
  if (f.binary) {
    const s = c.s
    if (s && s.exitCode === null) {
      try {
        s.proc.write(f.data)
      } catch {
        // process exited between the check and the call
      }
    }
    return
  }
  onText(c, f.data.toString('utf8'))
}

const server = http.createServer((_req, res) => void res.writeHead(404).end())
const wss = new WebSocketServer({
  server,
  path: '/pty',
  maxPayload: 1024 * 1024,
  handleProtocols: (protocols) => [...protocols].find(tokenOk) ?? false,
  verifyClient: ({ req }: { req: http.IncomingMessage }) => {
    if (!/^127\.0\.0\.1:\d+$/.test(req.headers.host ?? '')) return false
    if (!ORIGINS.has(req.headers.origin ?? '')) return false
    const offered = String(req.headers['sec-websocket-protocol'] ?? '').split(',').map((p) => p.trim())
    return offered.some(tokenOk)
  },
})
wss.on('error', (err) => console.error('pty ws server error:', err))

wss.on('connection', (ws) => {
  const c: Conn = { ws, s: null, queue: null }
  ws.on('error', () => ws.terminate())
  ws.on('message', (raw: Buffer, isBinary: boolean) => onFrame(c, { data: raw, binary: isBinary }))
  ws.on('close', () => {
    for (const session of sessions.values()) if (session.ws === ws) session.ws = null
  })
})

const census = (): PtySessionInfo[] =>
  [...sessions.values()].map((s) => ({ id: s.id, preset: s.preset, cwd: s.cwd, liveCwd: s.liveCwd ?? undefined, pid: s.pid, alive: s.exitCode === null }))

// Sync: the process exits right after, and an async taskkill still pending then would never run (orphaned agents).
function shutdown(): void {
  closing = true
  const all = [...sessions.values()]
  const pids = all.map(treePid).filter((p): p is number => p !== null)
  if (pids.length) {
    try {
      execFileSync(taskkillPath(process.env), ['/T', '/F', ...pids.flatMap((p) => ['/PID', String(p)])], { windowsHide: true, timeout: 2_500, stdio: 'ignore' })
    } catch {
      // some trees already gone (exit 128) or timeout; node-pty kill below still ends the shells
    }
  }
  for (const s of all) kill(s, true)
  // node-pty's ConPTY worker keeps the event loop alive after kill.
  setTimeout(() => process.exit(0), 300)
}

// Exited sessions linger only so a reloading page can still show their last output.
setInterval(() => {
  const now = Date.now()
  for (const s of [...sessions.values()]) if (s.exitCode !== null && !s.ws && now - s.exitedAt > EXITED_TTL_MS) {
      sessions.delete(s.id)
      control({ t: 'removed', id: s.id })
    }
}, 60_000).unref()

// If Electron dies without a clean shutdown, don't leave shells (and agents) running headless.
const parentPid = process.ppid
setInterval(() => {
  try {
    process.kill(parentPid, 0)
  } catch {
    shutdown()
  }
}, 5_000).unref()

function onMain(msg: PtyMainToCore): void {
  switch (msg.t) {
    case 'init':
      if (initialized) return
      initialized = true
      TOKEN = msg.token
      ORIGINS = new Set(msg.origins.filter(Boolean))
      server.listen(0, '127.0.0.1', () => {
        const addr = server.address()
        control({ t: 'listening', port: typeof addr === 'object' && addr ? addr.port : 0 })
      })
      return
    case 'resolved':
      return onResolved(msg)
    case 'cwd-ok':
      return onCwdOk(msg.rid, msg.cwd)
    case 'wt-reply': {
      const ws = wtWaits.get(msg.rid)
      wtWaits.delete(msg.rid)
      if (ws) send(ws, msg.reply)
      return
    }
    case 'branch': {
      const s = sessions.get(msg.id)
      if (!s) return
      s.branch = msg.branch
      send(s.ws, { t: 'branch', branch: msg.branch })
      return
    }
    case 'census':
      return control({ t: 'census-reply', sessions: census() })
    case 'kill-all':
      for (const s of [...sessions.values()]) kill(s)
      return
    case 'shutdown':
      return shutdown()
  }
}

function onControl(data: unknown): void {
  const msg = parseMainToCore(data)
  if (msg) onMain(msg)
}

if (STDIO) {
  const reader = new NdjsonReader(onControl)
  process.stdin.setEncoding('utf8')
  process.stdin.on('data', (chunk: string) => reader.push(chunk))
  process.stdin.on('end', shutdown)
} else {
  process.parentPort.on('message', ({ data }: { data: unknown }) => onControl(data))
}
