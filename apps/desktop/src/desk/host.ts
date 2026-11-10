import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createWriteStream, existsSync, type WriteStream } from 'node:fs'

export class DeskUnavailable extends Error {}

export type HostHandler = (params: unknown) => unknown
export type HostHandlers = Record<string, HostHandler>

export type DeskSpawner = (bin: string, args: string[], env: NodeJS.ProcessEnv) => ChildProcessWithoutNullStreams

export interface DeskHostOptions {
  bin: string
  env: NodeJS.ProcessEnv
  logFile?: string
  onEvent?: (event: string, payload: unknown) => void
  spawner?: DeskSpawner
  exists?: (file: string) => boolean
  bootTimeoutMs?: number
  hosts?: HostHandlers
}

interface Pending {
  resolve: (v: unknown) => void
  reject: (e: Error) => void
  hosts: HostHandlers
}

const MAX_RESTARTS = 3
const BOOT_TIMEOUT_MS = 10_000
const STOP_TIMEOUT_MS = 2_000
const STABLE_MS = 60_000
const LOG_TAIL = 40
const LINE_MAX = 64 * 1024 * 1024

const defaultSpawner: DeskSpawner = (bin, args, env) => spawn(bin, args, { env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })

export class DeskHost {
  private child: ChildProcessWithoutNullStreams | null = null
  private booting: Promise<void> | null = null
  private bootDone: ((err?: Error) => void) | null = null
  private restarts = 0
  private stopping = false
  private failed: string | null = null
  private nextId = 1
  private readonly pending = new Map<number, Pending>()
  private log: WriteStream | null = null
  channels: string[] = []
  readonly tail: string[] = []

  constructor(private readonly opts: DeskHostOptions) {}

  get dead(): string | null {
    return this.failed
  }

  get pid(): number | null {
    return this.child?.pid ?? null
  }

  start(): Promise<void> {
    if (this.failed) return Promise.reject(new DeskUnavailable(this.failed))
    if (this.booting) return this.booting
    this.stopping = false
    if (this.opts.logFile) this.log ??= createWriteStream(this.opts.logFile, { flags: 'w' })
    if (!(this.opts.exists ?? existsSync)(this.opts.bin)) {
      this.failed = `orrery-core not found at ${this.opts.bin}`
      this.write(this.failed)
      return Promise.reject(new DeskUnavailable(this.failed))
    }
    this.booting = this.boot()
    this.booting.catch(() => undefined)
    return this.booting
  }

  private boot(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.bootDone = null
        this.giveUp('boot timeout')
        reject(new DeskUnavailable('boot timeout'))
      }, this.opts.bootTimeoutMs ?? BOOT_TIMEOUT_MS)
      this.bootDone = (err) => {
        clearTimeout(timer)
        this.bootDone = null
        if (err) reject(err)
        else resolve()
      }
      this.spawn()
    })
  }

  async call(channel: string, args: unknown[], hosts: HostHandlers = {}): Promise<unknown> {
    if (this.failed) throw new DeskUnavailable(this.failed)
    if (this.stopping) throw new DeskUnavailable('stopping')
    await this.start()
    const child = this.child
    if (!child || child.stdin.destroyed) throw new DeskUnavailable('not running')
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject, hosts })
      this.send({ id, channel, args })
    })
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
    this.send({ type: 'shutdown' })
    child.stdin.end()
    if (!(await within(STOP_TIMEOUT_MS))) {
      child.kill()
      await within(1_000)
    }
    this.log?.end()
    this.log = null
  }

  private send(message: object): void {
    const stdin = this.child?.stdin
    if (!stdin || stdin.destroyed || !stdin.writable) return
    stdin.write(`${JSON.stringify(message)}\n`)
  }

  private spawn(): void {
    const spawnedAt = Date.now()
    const child = (this.opts.spawner ?? defaultSpawner)(this.opts.bin, ['desk'], this.opts.env)
    this.child = child
    let buf = ''
    child.stdout.on('data', (chunk: Buffer) => {
      buf += String(chunk)
      const lines = buf.split('\n')
      buf = lines.pop() ?? ''
      if (buf.length > LINE_MAX) buf = ''
      for (const line of lines) this.onLine(child, line.replace(/\r$/, ''))
    })
    child.stderr.on('data', (chunk: Buffer) => {
      for (const line of String(chunk).split(/\r?\n/)) if (line.trim()) this.write(line)
    })
    child.stdin.on('error', () => undefined)
    let done = false
    const finish = (code: number | null): void => {
      if (done) return
      done = true
      this.onExit(child, code, spawnedAt)
    }
    child.on('error', (err) => {
      this.write(`orrery-core desk: ${err.message}`)
      finish(null)
    })
    child.on('exit', (code) => finish(code))
  }

  private onLine(child: ChildProcessWithoutNullStreams, line: string): void {
    if (!line.trim()) return
    let m: Record<string, unknown>
    try {
      m = JSON.parse(line) as Record<string, unknown>
    } catch {
      this.write(line)
      return
    }
    if (!m || typeof m !== 'object') return
    if (m.ready === true) {
      this.channels = Array.isArray(m.channels) ? m.channels.filter((c): c is string => typeof c === 'string') : []
      this.write(`orrery-core desk ready (pid ${child.pid})`)
      this.bootDone?.()
      return
    }
    if (typeof m.event === 'string') {
      this.opts.onEvent?.(m.event, m.payload)
      return
    }
    if (typeof m.call === 'number') {
      void this.answer(m.call, m.for, m.method, m.params)
      return
    }
    if (typeof m.id === 'number') {
      const p = this.pending.get(m.id)
      if (!p) return
      this.pending.delete(m.id)
      if (m.ok === true) p.resolve(m.value ?? null)
      else p.reject(new Error(typeof m.error === 'string' ? m.error : 'failed'))
    }
  }

  private async answer(call: number, forId: unknown, method: unknown, params: unknown): Promise<void> {
    const own = typeof forId === 'number' && typeof method === 'string' ? this.pending.get(forId)?.hosts[method] : undefined
    const handler = own ?? (typeof method === 'string' ? this.opts.hosts?.[method] : undefined)
    if (!handler) {
      this.send({ reply: call, ok: false, error: `no host ${String(method)}` })
      return
    }
    try {
      const value = await handler(params)
      this.send({ reply: call, ok: true, value: value ?? null })
    } catch (err) {
      this.send({ reply: call, ok: false, error: err instanceof Error ? err.message.slice(0, 200) : 'failed' })
    }
  }

  private onExit(child: ChildProcessWithoutNullStreams, code: number | null, spawnedAt: number): void {
    if (this.child === child) this.child = null
    this.write(`orrery-core desk exited (code ${code})`)
    for (const [id, p] of this.pending) {
      this.pending.delete(id)
      p.reject(new DeskUnavailable(`exited with code ${code}`))
    }
    if (this.stopping) return
    if (Date.now() - spawnedAt > STABLE_MS) this.restarts = 0
    if (this.restarts >= MAX_RESTARTS) {
      this.giveUp(`exited with code ${code}`)
      return
    }
    this.restarts++
    if (this.bootDone) {
      this.spawn()
      return
    }
    this.booting = null
    void this.start().catch(() => undefined)
  }

  private giveUp(reason: string): void {
    this.failed ??= reason
    this.bootDone?.(new DeskUnavailable(reason))
    const child = this.child
    this.child = null
    if (child && child.exitCode === null) child.kill()
  }

  private write(line: string): void {
    this.log?.write(`${line}\n`)
    this.tail.push(line)
    if (this.tail.length > LOG_TAIL) this.tail.shift()
  }
}
