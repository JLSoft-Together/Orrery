import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import net from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import WebSocket from 'ws'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const DEMO = path.join(REPO, 'scripts', 'demo-fixture.mjs')
const ENTRY = path.join(REPO, 'apps', 'collector', 'src', 'index.ts')
const CORE_KEYS = ['CCM_CORE_PROCS', 'CCM_CORE_USAGE', 'CCM_CORE_TRANSCRIPT', 'CCM_CORE_SIDEFILES', 'CCM_CORE_CODEX']
const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, '').split('=')
    return [k, v ?? 'true']
  }),
)
const SESSIONS = Number(args.sessions ?? 30)
const SUBAGENTS = Number(args.subagents ?? 6)
const TICK_MS = Number(args.tick ?? 400)
const SECONDS = Number(args.seconds ?? 60)
const SEED = args.seed === undefined ? null : Number(args.seed)
const SERVER = args.server ?? 'rust'
const NODE_CORE = args['node-core'] ?? 'rust'
const KINDS = SERVER === 'both' ? ['node', 'rust'] : [SERVER]
const LATE = Number(args.late ?? 0)
const LATENCY_KINDS =new Set(['tool.started', 'tool.failed'])
const BIN = args['rust-bin'] ?? process.env.CCM_CORE_BIN ?? ['release', 'debug'].map((d) => path.join(REPO, 'apps', 'core', 'target', d, process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core')).find((p) => existsSync(p))
if (!['node', 'rust', 'both'].includes(SERVER)) throw new Error('--server must be node, rust or both')
if (!['node', 'rust'].includes(NODE_CORE)) throw new Error('--node-core must be node or rust')
if (!BIN && (SERVER !== 'node' || NODE_CORE === 'rust')) throw new Error('orrery-core binary not found: cargo build --release --locked -j 4 -p core-bin, or --rust-bin=')

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

function freePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer()
    srv.once('error', reject)
    srv.listen(0, '127.0.0.1', () => {
      const { port } = srv.address()
      srv.close(() => resolve(port))
    })
  })
}

function quantiles(list) {
  const v = [...list].sort((a, b) => a - b)
  const q = (p) => (v.length ? Math.round(v[Math.min(v.length - 1, Math.round((v.length - 1) * p))] * 10) / 10 : null)
  return { n: v.length, p50: q(0.5), p90: q(0.9), p99: q(0.99), max: q(1) }
}

function baseEnv(tmp, root) {
  const env = { ...process.env }
  for (const k of ['CCM_APP_KEY', 'CCM_TOAST_LINK', 'CCM_HOST_RPC', 'CCM_DATA_DIR', 'CCM_PORT', 'CCM_CORE', 'CCM_TRACE_OPS']) delete env[k]
  return Object.assign(env, {
    CLAUDE_CONFIG_DIR: root,
    CODEX_HOME: path.join(tmp, 'codex'),
    CODEWHALE_HOME: path.join(tmp, 'codewhale'),
    CCM_CODEX: '0',
    CCM_DEEPSEEK: '0',
    CCM_TOAST: '0',
    CCM_VERIFY_PROCESSES: '0',
    CCM_RES_FIXTURE: path.join(tmp, 'res-fixture.json'),
    CCM_WEB_DIST: path.join(tmp, 'web'),
  })
}

async function startServer(kind, tmp, root) {
  const data = path.join(tmp, `data-${kind}`)
  await mkdir(data, { recursive: true })
  const port = await freePort()
  if (port === 4317) throw new Error('refusing port 4317')
  const env = { ...baseEnv(tmp, root), CCM_DATA_DIR: data, CCM_PORT: String(port) }
  let proc
  if (kind === 'rust') {
    env.CCM_SERVE_STATS = '1'
    proc = spawn(BIN, ['serve', '--dev'], { cwd: REPO, env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
  } else {
    if (NODE_CORE === 'node') {
      delete env.CCM_CORE_BIN
      for (const k of CORE_KEYS) env[k] = 'node'
    } else {
      env.CCM_CORE_BIN = BIN
      for (const k of CORE_KEYS) delete env[k]
    }
    proc = spawn(process.execPath, ['--import', 'tsx', ENTRY, '--dev'], { cwd: REPO, env, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true })
  }
  const err = []
  proc.stderr.on('data', (d) => err.push(String(d)))
  proc.stdout.on('data', () => undefined)
  const exited = new Promise((resolve) => proc.once('exit', resolve))
  return { kind, port, proc, err, exited, ws: null, frames: 0, latency: [], late: [], health: [] }
}

async function waitHealth(s, deadline) {
  for (;;) {
    if (s.proc.exitCode !== null) throw new Error(`${s.kind} exited ${s.proc.exitCode}\n${s.err.join('')}`)
    if (await fetch(`http://127.0.0.1:${s.port}/health`).then((r) => r.ok).catch(() => false)) return
    if (Date.now() > deadline) throw new Error(`${s.kind} did not answer /health`)
    await sleep(200)
  }
}

function listen(s, from) {
  s.ws = new WebSocket(`ws://127.0.0.1:${s.port}/ws`)
  s.ws.on('message', (raw) => {
    const got = Date.now()
    s.frames++
    if (got < from) return
    const msg = JSON.parse(String(raw))
    for (const e of msg.events ?? []) {
      if (e.type !== 'activity' || !LATENCY_KINDS.has(e.payload?.kind)) continue
      const at = Date.parse(e.payload.at ?? '')
      if (!Number.isFinite(at) || at < from) continue
      s.latency.push(got - at)
      if (LATE && got - at >= LATE) s.late.push({ ms: got - at, at: at - from, agent: String(e.payload.agentId ?? '').endsWith(':main') ? 'main' : 'sub', terminal: e.payload.terminalId })
    }
  })
  return new Promise((resolve, reject) => {
    s.ws.once('open', resolve)
    s.ws.once('error', reject)
  })
}

async function stop(s) {
  s.ws?.close()
  if (s.kind === 'rust') s.proc.stdin.end(`${JSON.stringify({ type: 'shutdown' })}\n`)
  else s.proc.kill()
  await Promise.race([s.exited, sleep(8000)])
  if (s.proc.exitCode === null && s.proc.signalCode === null) s.proc.kill()
}

function report(s) {
  const line = s.err.join('').split(/\r?\n/).find((l) => l.includes('"serve.stats"'))
  const late = LATE ? { late: s.late.slice(0, 40).map((l) => `${l.agent}@${(l.at / 1000).toFixed(1)}s+${l.ms}ms ${l.terminal}`), lateCount: s.late.length } : {}
  return { server: s.kind, ...(s.kind === 'node' ? { nodeCore: NODE_CORE } : {}), frames: s.frames, writeToWsMs: quantiles(s.latency), health: quantiles(s.health), stats: line ? JSON.parse(line) : null, ...late }
}

async function main() {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'ccm-lockstats-'))
  const root = path.join(tmp, 'root')
  await writeFile(path.join(tmp, 'res-fixture.json'), JSON.stringify({ procs: [], ports: {} }))
  const seed = SEED === null ? [] : [`--seed=${SEED}`]
  const demo = spawn(process.execPath, [DEMO, `--root=${root}`, `--sessions=${SESSIONS}`, `--subagents=${SUBAGENTS}`, ...seed, `--tick=${TICK_MS}`, `--duration=${SECONDS + 25}`, '--peers=10'], { stdio: 'ignore', windowsHide: true })
  const servers = []
  try {
    await sleep(1500)
    for (const kind of KINDS) servers.push(await startServer(kind, tmp, root))
    for (const s of servers) await waitHealth(s, Date.now() + 60_000)
    await sleep(Number(args.warmup ?? 3000))
    const from = Date.now() + Number(args.settle ?? 2000)
    for (const s of servers) await listen(s, from)
    await sleep(Math.max(0, from - Date.now()))
    const until = from + SECONDS * 1000
    while (Date.now() < until) {
      for (const s of servers) {
        const t = performance.now()
        await fetch(`http://127.0.0.1:${s.port}/health`).catch(() => undefined)
        s.health.push(performance.now() - t)
      }
      await sleep(500)
    }
    for (const s of servers) await stop(s)
    if (args.log) await writeFile(path.resolve(args.log), servers.map((s) => `== ${s.kind}\n${s.err.join('')}`).join('\n'))
    const out = { sessions: SESSIONS, subagents: SUBAGENTS, tickMs: TICK_MS, seconds: SECONDS, seeded: SEED !== null, servers: servers.map(report) }
    console.log(JSON.stringify(out, null, args.pretty ? 2 : 0))
  } finally {
    for (const s of servers) {
      s.ws?.terminate()
      if (s.proc.exitCode === null && s.proc.signalCode === null) s.proc.kill()
    }
    if (demo.exitCode === null) await Promise.race([new Promise((resolve) => demo.once('exit', resolve)), sleep(30_000)])
    if (demo.exitCode === null) demo.kill()
    await sleep(500)
    await rm(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => undefined)
  }
}

main().catch((e) => {
  console.error(`[trace-lockstats] ${e instanceof Error ? e.message : e}`)
  process.exit(1)
})
