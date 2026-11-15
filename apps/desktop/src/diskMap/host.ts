import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'

export type DiskMapSpawner = (bin: string, args: string[], env: NodeJS.ProcessEnv) => ChildProcessWithoutNullStreams

export interface DiskMapHostOptions {
  bin: string
  env: NodeJS.ProcessEnv
  onEvent?: (event: string, payload: unknown) => void
  onExit?: (code: number | null, busy: boolean) => void
  spawner?: DiskMapSpawner
  exists?: (file: string) => boolean
  bootTimeoutMs?: number
  callTimeoutMs?: number
}

export type CoreLine =
  | { kind: 'ready'; pid: number | null; channels: string[] }
  | { kind: 'event'; event: string; payload: unknown }
  | { kind: 'reply'; id: number; ok: boolean; value: unknown; error: string }
  | { kind: 'text'; text: string }

interface Pending {
  resolve: (v: unknown) => void
  reject: (e: Error) => void
  timer: NodeJS.Timeout | null
}

const BOOT_TIMEOUT_MS = 10_000
const CALL_TIMEOUT_MS = 120_000
const STOP_TIMEOUT_MS = 2_000
const LINE_MAX = 64 * 1024 * 1024
const LOG_TAIL = 40

const defaultSpawner: DiskMapSpawner = (bin, args, env) => spawn(bin, args, { env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })

export function parseCoreLine(line: string): CoreLine | null {
  const text = line.replace(/\r$/, '')
  if (!text.trim()) return null
  let m: unknown
  try {
    m = JSON.parse(text)
  } catch {
    return { kind: 'text', text }
  }
  if (!m || typeof m !== 'object' || Array.isArray(m)) return { kind: 'text', text }
  const o = m as Record<string, unknown>
  if (o.ready === true) {
    return { kind: 'ready', pid: typeof o.pid === 'number' ? o.pid : null, channels: Array.isArray(o.channels) ? o.channels.filter((c): c is string => typeof c === 'string') : [] }
  }
  if (typeof o.event === 'string') return { kind: 'event', event: o.event, payload: o.payload ?? null }
  if (typeof o.id === 'number' && Number.isInteger(o.id)) {
    return { kind: 'reply', id: o.id, ok: o.ok === true, value: o.value ?? null, error: typeof o.error === 'string' ? o.error : 'failed' }
  }
  return { kind: 'text', text }
}

export class LineSplitter {
  private buf = ''

  push(chunk: string): string[] {
    this.buf += chunk
    const lines = this.buf.split('\n')
    this.buf = lines.pop() ?? ''
    if (this.buf.length > LINE_MAX) this.buf = ''
    return lines
  }
}

export class DiskMapHost {
  private child: ChildProcessWithoutNullStreams | null = null
  private booting: Promise<void> | null = null
  private bootDone: ((err?: Error) => void) | null = null
  private nextId = 1
  private readonly pending = new Map<number, Pending>()
  private stopping = false
  private scanning = false
  channels: string[] = []
  lastError: string | null = null
  readonly tail: string[] = []

  constructor(private readonly opts: DiskMapHostOptions) {}

  get running(): boolean {
    return this.child !== null
  }

  get pid(): number | null {
    return this.child?.pid ?? null
  }

  start(): Promise<void> {
    if (this.booting) return this.booting
    this.stopping = false
    if (!(this.opts.exists ?? existsSync)(this.opts.bin)) {
      this.lastError = `orrery-core not found at ${this.opts.bin}`
      this.note(this.lastError)
      return Promise.reject(new Error(this.lastError))
    }
    const boot = new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.bootDone = null
        this.lastError = 'boot timeout'
        const child = this.child
        if (child && child.exitCode === null) child.kill()
        reject(new Error('boot timeout'))
      }, this.opts.bootTimeoutMs ?? BOOT_TIMEOUT_MS)
      this.bootDone = (err) => {
        clearTimeout(timer)
        this.bootDone = null
        if (err) reject(err)
        else resolve()
      }
      try {
        this.spawn()
      } catch (err) {
        this.bootDone(err instanceof Error ? err : new Error(String(err)))
      }
    })
    this.booting = boot
    boot.catch(() => {
      if (this.booting === boot) this.booting = null
    })
    return boot
  }

  async call(channel: string, args: unknown = {}, timeoutMs = this.opts.callTimeoutMs ?? CALL_TIMEOUT_MS): Promise<unknown> {
    if (this.stopping) throw new Error('stopping')
    await this.start()
    const child = this.child
    if (!child || child.stdin.destroyed || !child.stdin.writable) throw new Error('not running')
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      const timer = timeoutMs > 0
        ? setTimeout(() => {
            if (this.pending.delete(id)) reject(new Error(`timeout ${channel}`))
          }, timeoutMs)
        : null
      this.pending.set(id, { resolve, reject, timer })
      child.stdin.write(`${JSON.stringify({ id, channel, args })}\n`)
    })
  }

  markScanning(on: boolean): void {
    this.scanning = on
  }

  async stop(): Promise<void> {
    this.stopping = true
    const child = this.child
    if (!child) return
    const exited = new Promise<void>((resolve) => child.once('exit', () => resolve()))
    const within = (ms: number): Promise<boolean> => {
      let timer: NodeJS.Timeout | undefined
      const t = new Promise<boolean>((resolve) => (timer = setTimeout(() => resolve(false), ms)))
      return Promise.race([exited.then(() => true), t]).finally(() => clearTimeout(timer))
    }
    if (!child.stdin.destroyed && child.stdin.writable) {
      child.stdin.write(`${JSON.stringify({ type: 'shutdown' })}\n`)
      child.stdin.end()
    }
    if (!(await within(STOP_TIMEOUT_MS))) {
      child.kill()
      await within(1_000)
    }
  }

  kill(): void {
    this.stopping = true
    const child = this.child
    if (child && child.exitCode === null) child.kill()
  }

  private spawn(): void {
    const child = (this.opts.spawner ?? defaultSpawner)(this.opts.bin, ['disk'], this.opts.env)
    this.child = child
    const out = new LineSplitter()
    child.stdout.on('data', (chunk: Buffer) => {
      for (const line of out.push(String(chunk))) this.onLine(child, line)
    })
    child.stderr.on('data', (chunk: Buffer) => {
      for (const line of String(chunk).split(/\r?\n/)) if (line.trim()) this.note(line)
    })
    child.stdin.on('error', () => undefined)
    let done = false
    const finish = (code: number | null): void => {
      if (done) return
      done = true
      this.onExit(child, code)
    }
    child.on('error', (err) => {
      this.note(`orrery-core disk: ${err.message}`)
      finish(null)
    })
    child.on('exit', (code) => finish(code))
  }

  private onLine(child: ChildProcessWithoutNullStreams, raw: string): void {
    const line = parseCoreLine(raw)
    if (!line) return
    if (line.kind === 'text') return this.note(line.text)
    if (line.kind === 'ready') {
      this.channels = line.channels
      this.lastError = null
      this.note(`orrery-core disk ready (pid ${line.pid ?? child.pid})`)
      this.bootDone?.()
      return
    }
    if (line.kind === 'event') {
      if (line.event === 'disk:done') this.scanning = false
      this.opts.onEvent?.(line.event, line.payload)
      return
    }
    const p = this.pending.get(line.id)
    if (!p) return
    this.pending.delete(line.id)
    if (p.timer) clearTimeout(p.timer)
    if (line.ok) p.resolve(line.value)
    else p.reject(new Error(line.error))
  }

  private onExit(child: ChildProcessWithoutNullStreams, code: number | null): void {
    if (this.child !== child) return
    this.child = null
    this.booting = null
    const reason = `orrery-core disk exited (code ${code})`
    this.note(reason)
    if (!this.stopping) this.lastError = reason
    this.bootDone?.(new Error(reason))
    for (const [id, p] of this.pending) {
      this.pending.delete(id)
      if (p.timer) clearTimeout(p.timer)
      p.reject(new Error(reason))
    }
    const busy = this.scanning
    this.scanning = false
    if (!this.stopping) this.opts.onExit?.(code, busy)
  }

  private note(line: string): void {
    this.tail.push(line)
    if (this.tail.length > LOG_TAIL) this.tail.shift()
  }
}
