import { spawn, type ChildProcess } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import os from 'node:os'
import path from 'node:path'
import WebSocket from 'ws'
import type { PtyServerMessage } from '../../../../packages/shared/src/pty'
import { encodeControl, NdjsonReader, parseCoreToMain, type PtyCoreToMain, type PtyMainToCore } from '../../../../packages/shared/src/ptyControl'
import { cleanOutput, decodeStream, maskValue, type Names } from './normalize'
import { CHILDREN, type ChildName, type Scenario, type Step } from './script'

export type TargetKind = 'node' | 'rust'

export interface Snapshot {
  name: string
  clients: Record<string, { texts: unknown[]; out: string }>
  ctl: unknown[]
}

export interface Transcript {
  scenario: string
  snaps: Snapshot[]
}

const ORIGIN = 'http://127.0.0.1:4317'
const PRESETS = new Set(['claude', 'codex', 'deepseek', 'shell'])
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms))

class Client {
  readonly ws: WebSocket
  readonly texts: PtyServerMessage[] = []
  readonly chunks: Buffer[] = []
  closeCode: number | null = null
  private waiters: (() => void)[] = []

  constructor(port: number, token: string) {
    this.ws = new WebSocket(`ws://127.0.0.1:${port}/pty`, [`ccm.pty.${token}`], { origin: ORIGIN })
    this.ws.on('message', (data: Buffer, binary: boolean) => {
      if (binary) this.chunks.push(Buffer.from(data))
      else this.texts.push(JSON.parse(data.toString('utf8')) as PtyServerMessage)
      this.wake()
    })
    this.ws.on('close', (code) => {
      this.closeCode = code
      this.wake()
    })
    this.ws.on('error', () => undefined)
  }

  private wake(): void {
    for (const w of this.waiters.splice(0)) w()
  }

  opened(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws.once('open', () => resolve())
      this.ws.once('error', reject)
    })
  }

  async until(check: () => boolean, ms: number, what: string): Promise<void> {
    const end = Date.now() + ms
    while (!check()) {
      if (Date.now() > end) throw new Error(`timeout waiting for ${what}; texts=${JSON.stringify(this.texts)} out=${JSON.stringify(this.output().slice(-300))}`)
      await new Promise<void>((r) => {
        this.waiters.push(r)
        setTimeout(r, 50)
      })
    }
  }

  output(): string {
    return decodeStream(this.chunks)
  }
}

export class Target {
  readonly token = randomBytes(32).toString('hex')
  readonly tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-parity-')))
  port = 0
  stderr = ''
  private proc!: ChildProcess
  private fromCore: PtyCoreToMain[] = []
  private listeners = new Set<(m: PtyCoreToMain) => void>()
  onMessage: ((m: PtyCoreToMain) => void) | null = null

  constructor(
    private readonly kind: TargetKind,
    private readonly nodeBundle: string,
    private readonly coreBin: string,
  ) {}

  send(msg: PtyMainToCore): void {
    if (this.proc.stdin?.writable) this.proc.stdin.write(encodeControl(msg))
  }

  async start(): Promise<void> {
    const env = { ...process.env }
    if (this.kind === 'node') {
      this.proc = spawn(process.execPath, [this.nodeBundle], {
        cwd: path.join(__dirname, '..', '..'),
        env: { ...env, CCM_PTY_CONTROL: 'stdio', CCM_NODE_PTY: createRequire(__filename).resolve('@lydell/node-pty') },
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      })
    } else {
      this.proc = spawn(this.coreBin, [], { cwd: this.tmp, env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    }
    this.proc.stderr!.setEncoding('utf8')
    this.proc.stderr!.on('data', (d: string) => (this.stderr += d))
    this.proc.stdin!.on('error', () => undefined)
    const reader = new NdjsonReader((raw) => {
      const m = parseCoreToMain(raw)
      if (!m) return
      this.fromCore.push(m)
      this.onMessage?.(m)
      for (const l of [...this.listeners]) l(m)
    })
    this.proc.stdout!.setEncoding('utf8')
    this.proc.stdout!.on('data', (d: string) => reader.push(d))
    this.send({ t: 'init', token: this.token, origins: [ORIGIN], appVersion: '9.9.9' })
    const listening = await this.next((m) => m.t === 'listening', 15_000)
    if (listening.t === 'listening') this.port = listening.port
  }

  next(pred: (m: PtyCoreToMain) => boolean, ms: number): Promise<PtyCoreToMain> {
    const seen = this.fromCore.find(pred)
    if (seen) return Promise.resolve(seen)
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.listeners.delete(on)
        reject(new Error(`no control message in ${ms} ms; stderr: ${this.stderr.slice(-400)}`))
      }, ms)
      const on = (m: PtyCoreToMain): void => {
        if (!pred(m)) return
        clearTimeout(timer)
        this.listeners.delete(on)
        resolve(m)
      }
      this.listeners.add(on)
    })
  }

  async stop(): Promise<void> {
    const proc = this.proc
    if (proc && proc.exitCode === null) {
      this.send({ t: 'shutdown' })
      const exited = new Promise((r) => proc.once('exit', r))
      try {
        proc.stdin!.end()
      } catch {
        proc.kill()
      }
      await Promise.race([exited, sleep(6_000)])
      if (proc.exitCode === null) proc.kill()
    }
    try {
      rmSync(this.tmp, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
    } catch {
      return
    }
  }
}

const idFor = (alias: string): string => {
  const hex = Buffer.from(alias).toString('hex').padEnd(12, '0').slice(0, 12)
  return `00000000-0000-4000-8000-${hex}`
}

export async function runScenario(kind: TargetKind, scenario: Scenario, nodeBundle: string, coreBin: string): Promise<Transcript> {
  const target = new Target(kind, nodeBundle, coreBin)
  const names: Names = { tmp: target.tmp, ids: new Map(), pids: new Map([[process.pid, 'pid:self']]) }
  const vars = new Map<string, number>([['self', process.pid]])
  const clients = new Map<string, Client>()
  const ctlLog: PtyCoreToMain[] = []
  const pending: { alias: string; child: ChildName } = { alias: '', child: 'echo' }
  let ctlCursor = 0
  let autoCount = 0

  const alias = (a: string): string => {
    const id = idFor(a)
    names.ids.set(id, a)
    return id
  }

  const subst = (v: unknown): unknown => {
    if (typeof v === 'string' && v.startsWith('$TMP')) return target.tmp + v.slice(4)
    if (Array.isArray(v)) return v.map(subst)
    if (v && typeof v === 'object' && ((v as { t?: string }).t === 'attach' || (v as { t?: string }).t === 'kill') && typeof (v as { id?: unknown }).id === 'string') return { ...(v as object), id: alias((v as { id: string }).id) }
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v as Record<string, unknown>).map(([k, x]) => [k, subst(x)]))
    return v
  }

  const answer = (m: PtyCoreToMain): void => {
    if (m.t === 'resolve') {
      const msg = m.msg
      const refuse = (message: 'preset' | 'cwd'): void => target.send({ t: 'resolved', rid: m.rid, ok: false, message })
      if (!PRESETS.has(msg.preset)) return refuse('preset')
      if (msg.resume !== undefined && (msg.preset === 'shell' || !UUID.test(String(msg.resume)))) return refuse('preset')
      if (typeof msg.cwd === 'string' && msg.cwd.startsWith('/nope')) return refuse('cwd')
      const id = pending.alias ? alias(pending.alias) : alias(`auto${autoCount++}`)
      const childEnv: Record<string, string> = {}
      for (const [k, v] of Object.entries(process.env)) if (typeof v === 'string' && !k.startsWith('CCM_') && !k.startsWith('VITEST')) childEnv[k] = v
      target.send({ t: 'resolved', rid: m.rid, ok: true, id, file: process.execPath, args: ['-e', CHILDREN[pending.child]], env: { ...childEnv, ORRERY_TOKEN: 'c'.repeat(64) }, cwd: target.tmp })
    } else if (m.t === 'cwd') {
      target.send({ t: 'cwd-ok', rid: m.rid, cwd: m.cwd.startsWith(target.tmp) ? m.cwd : null })
    } else if (m.t === 'wt') {
      target.send({ t: 'wt-reply', rid: m.rid, reply: { t: 'wt.list', repo: null, worktrees: [] } })
    } else if (m.t === 'spawned' || m.t === 'exited' || m.t === 'removed' || m.t === 'census-reply') {
      ctlLog.push(m)
    }
  }

  const client = (c: string): Client => {
    const found = clients.get(c)
    if (!found) throw new Error(`no client ${c}`)
    return found
  }

  const noteReady = (c: Client): void => {
    let current = ''
    for (const t of c.texts) {
      if (t.t === 'ready') current = names.ids.get(t.id) ?? ''
      const pid = t.t === 'ready' || t.t === 'pid' ? t.pid : 0
      if (pid > 0 && current) {
        vars.set(current, pid)
        names.pids.set(pid, `pid:${current}`)
      }
    }
  }

  const transcript: Transcript = { scenario: scenario.name, snaps: [] }
  target.onMessage = answer
  const ms = (s: { ms?: number }, d = 10_000): number => s.ms ?? d

  const runStep = async (s: Step): Promise<void> => {
    switch (s.op) {
      case 'connect': {
        const c = new Client(target.port, target.token)
        await c.opened()
        clients.set(s.c, c)
        return
      }
      case 'badAuth': {
        const proto = s.kind === 'token' ? 'ccm.pty.nope' : `ccm.pty.${target.token}`
        const ws = new WebSocket(`ws://127.0.0.1:${target.port}/pty`, [proto], { origin: s.kind === 'origin' ? 'http://evil.test' : ORIGIN })
        const result = await new Promise<string>((resolve) => {
          ws.once('open', () => resolve('opened'))
          ws.once('unexpected-response', (_req, res) => resolve(res.statusCode === 401 || res.statusCode === 403 ? 'rejected' : `status ${res.statusCode}`))
          ws.once('error', () => resolve('error'))
        })
        ws.terminate()
        if (result !== 'rejected') throw new Error(`bad auth ${s.kind}: ${result}`)
        return
      }
      case 'spawn': {
        pending.alias = s.id
        pending.child = s.child ?? 'echo'
        const msg = { t: 'spawn', preset: 'shell', cols: 100, rows: 30, cwd: '$TMP', ...(s.msg ?? {}) }
        client(s.c).ws.send(JSON.stringify(subst(msg)))
        const c = client(s.c)
        await c.until(() => c.texts.some((t) => t.t === 'ready' || t.t === 'error'), 15_000, `spawn ${s.id}`)
        noteReady(c)
        return
      }
      case 'send':
        client(s.c).ws.send(JSON.stringify(subst(s.msg)))
        return
      case 'bin':
        client(s.c).ws.send(Buffer.from(s.text, 'utf8'))
        return
      case 'waitText': {
        const c = client(s.c)
        await c.until(() => c.texts.filter((t) => t.t === s.t).length >= (s.n ?? 1), 10_000, `text ${s.t}`)
        noteReady(c)
        return
      }
      case 'waitOut': {
        const c = client(s.c)
        await c.until(() => c.output().includes(s.has), ms(s), `output ${JSON.stringify(s.has)}`)
        noteReady(c)
        return
      }
      case 'waitClose': {
        const c = client(s.c)
        await c.until(() => c.closeCode !== null, 10_000, 'close')
        if (s.code !== undefined && c.closeCode !== s.code) throw new Error(`close code ${c.closeCode}, wanted ${s.code}`)
        return
      }
      case 'closeWs':
        client(s.c).ws.close()
        await client(s.c).until(() => client(s.c).closeCode !== null, 5_000, 'close')
        return
      case 'sleep':
        await sleep(s.ms)
        return
      case 'ctl':
        const m = s.msg as { t: string; id?: string }
        target.send((m.t === 'branch' && m.id ? { ...m, id: alias(m.id) } : m) as PtyMainToCore)
        return
      case 'waitCtl': {
        const want = s.n ?? 1
        const has = (): boolean => ctlLog.filter((m) => m.t === s.t && (s.id === undefined || ('id' in m && m.id === idFor(s.id)))).length >= want
        const end = Date.now() + 10_000
        while (!has()) {
          if (Date.now() > end) throw new Error(`no control ${s.t} ${s.id ?? ''}; log=${JSON.stringify(ctlLog)} stderr=${target.stderr.slice(-300)}`)
          await sleep(25)
        }
        return
      }
      case 'capture': {
        const m = new RegExp(s.re).exec(cleanOutput(client(s.c).output()))
        if (!m) throw new Error(`capture ${s.re} found nothing`)
        const pid = Number(m[1])
        vars.set(s.as, pid)
        names.pids.set(pid, `pid:${s.as}`)
        return
      }
      case 'owners': {
        const pids = s.pids.map((p) => (typeof p === 'number' ? p : (vars.get(p.slice(1)) ?? -1)))
        client(s.c).ws.send(JSON.stringify({ t: 'owners', pids }))
        return
      }
      case 'fill': {
        for (let i = 0; i < s.n; i++) {
          const name = `f${i}`
          const c = new Client(target.port, target.token)
          await c.opened()
          clients.set(name, c)
          pending.alias = `F${i}`
          pending.child = 'sleep'
          c.ws.send(JSON.stringify({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: target.tmp }))
          await c.until(() => c.texts.some((t) => t.t === 'ready' || t.t === 'error'), 15_000, `fill ${i}`)
          const first = c.texts[0]
          if (first?.t !== 'ready') throw new Error(`fill ${i} refused: ${JSON.stringify(first)}`)
        }
        return
      }
      case 'snap': {
        if (s.ctl) await sleep(250)
        const out: Snapshot['clients'] = {}
        for (const name of s.clients ?? []) {
          const c = client(name)
          noteReady(c)
          let text = cleanOutput(c.output())
          if (s.upTo && !s.skipOut?.includes(name)) {
            const at = text.indexOf(s.upTo)
            if (at < 0) throw new Error(`snap ${s.name}: marker ${JSON.stringify(s.upTo)} missing in ${JSON.stringify(text.slice(-200))}`)
            text = text.slice(0, at + s.upTo.length)
          }
          if (s.tail) text = text.slice(-s.tail)
          const texts = c.texts.filter((t) => t.t !== 'pid').map((t) => maskValue(t, names))
          out[name] = { texts, out: s.skipOut?.includes(name) ? '' : text }
          const want = s.texts?.[name]
          if (want && JSON.stringify(texts) !== JSON.stringify(want)) throw new Error(`snap ${s.name}/${name}: texts ${JSON.stringify(texts)} != ${JSON.stringify(want)}`)
          for (const needle of s.has?.[name] ?? []) if (!text.includes(needle)) throw new Error(`snap ${s.name}/${name}: output lacks ${JSON.stringify(needle)}; got ${JSON.stringify(text.slice(-300))}`)
        }
        const ctl = s.ctl
          ? ctlLog
              .slice(ctlCursor)
              .map((m) => JSON.stringify(maskValue(m, names)))
              .sort()
              .map((j) => JSON.parse(j) as unknown)
          : []
        ctlCursor = ctlLog.length
        transcript.snaps.push({ name: s.name, clients: out, ctl })
        return
      }
    }
  }

  try {
    await target.start()
    for (const step of scenario.steps) await runStep(step)
  } finally {
    for (const c of clients.values()) c.ws.terminate()
    await target.stop()
  }
  return transcript
}
