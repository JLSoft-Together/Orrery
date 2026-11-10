import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createWriteStream, existsSync, type WriteStream } from 'node:fs'
import { CollectorHost, FatalError, isHealthy, type CollectorHostOptions, type FatalReason } from './collector-host'
import { HOST_REQUEST, HostRpc } from './host-rpc'

export interface MonitorHost {
  readonly ownsPort: boolean
  readonly tail: string[]
  start(): Promise<void>
  stop(): Promise<void>
  post(message: object): void
}

export type CoreServeMode = 'rust' | 'node'

export type ServeSpawner = (bin: string, args: string[], env: NodeJS.ProcessEnv) => ChildProcessWithoutNullStreams

export interface CoreServeHostOptions {
  bin: string
  port: number
  env: NodeJS.ProcessEnv
  logFile: string
  onFatal: (reason: FatalReason) => void
  onMessage?: (data: unknown) => void
  onExit?: () => void
  rpc?: HostRpc
  spawner?: ServeSpawner
  exists?: (file: string) => boolean
  bootTimeoutMs?: number
}

const PORT_IN_USE_EXIT = 3
const MAX_RESTARTS = 3
const BOOT_TIMEOUT_MS = 30_000
const STOP_TIMEOUT_MS = 4_000
const STABLE_MS = 60_000
const LOG_TAIL = 40
const LINE_MAX = 1024 * 1024

export function coreServeMode(env: NodeJS.ProcessEnv): CoreServeMode {
  return env.CCM_CORE?.trim().toLowerCase() === 'node' ? 'node' : 'rust'
}

const defaultSpawner: ServeSpawner = (bin, args, env) => spawn(bin, args, { env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })

export class CoreServeHost implements MonitorHost {
  private child: ChildProcessWithoutNullStreams | null = null
  private restarts = 0
  private stopping = false
  private booting = false
  private log: WriteStream | null = null
  private lastExit: number | null | undefined
  private readonly rpc: HostRpc
  readonly tail: string[] = []

  constructor(private readonly opts: CoreServeHostOptions) {
    this.rpc = opts.rpc ?? new HostRpc()
  }

  get ownsPort(): boolean {
    return this.child !== null
  }

  async start(): Promise<void> {
    if (this.child) await this.stop()
    this.stopping = false
    this.restarts = 0
    if (await isHealthy(this.opts.port)) throw new FatalError({ kind: 'port-in-use' })
    this.log ??= createWriteStream(this.opts.logFile, { flags: 'w' })
    if (!(this.opts.exists ?? existsSync)(this.opts.bin)) {
      this.write(`orrery-core not found at ${this.opts.bin}`)
      throw new FatalError({ kind: 'exited', code: null })
    }
    this.booting = true
    try {
      this.spawn()
      await this.waitHealthy()
    } finally {
      this.booting = false
    }
  }

  post(message: object): void {
    this.send(message)
  }

  async stop(): Promise<void> {
    this.stopping = true
    const child = this.child
    if (!child) return
    const exited = new Promise<'exited'>((resolve) => child.once('exit', () => resolve('exited')))
    const within = (ms: number): Promise<'exited' | 'timeout'> => {
      let timer: NodeJS.Timeout | undefined
      const timeout = new Promise<'timeout'>((resolve) => (timer = setTimeout(() => resolve('timeout'), ms)))
      return Promise.race([exited, timeout]).finally(() => clearTimeout(timer))
    }
    this.send({ type: 'shutdown' })
    child.stdin.end()
    if ((await within(STOP_TIMEOUT_MS)) === 'timeout') {
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
    this.lastExit = undefined
    const spawnedAt = Date.now()
    const child = (this.opts.spawner ?? defaultSpawner)(this.opts.bin, ['serve'], { ...this.opts.env, CCM_HOST_RPC: '1' })
    let pending = ''
    child.stdout.on('data', (buf: Buffer) => {
      pending += String(buf)
      const lines = pending.split('\n')
      pending = lines.pop() ?? ''
      if (pending.length > LINE_MAX) pending = ''
      for (const line of lines) this.onLine(line.replace(/\r$/, ''))
    })
    child.stderr.on('data', (buf: Buffer) => {
      for (const line of String(buf).split(/\r?\n/)) if (line.trim()) this.write(line)
    })
    child.stdin.on('error', () => undefined)
    let done = false
    const finish = (code: number | null): void => {
      if (done) return
      done = true
      this.onExit(child, code, spawnedAt)
    }
    child.on('error', (err) => {
      this.write(`orrery-core serve: ${err.message}`)
      finish(null)
    })
    child.on('exit', (code) => finish(code))
    this.child = child
  }

  private onLine(line: string): void {
    if (!line.trim()) return
    let message: unknown
    try {
      message = JSON.parse(line)
    } catch {
      this.write(line)
      return
    }
    const type = (message as { type?: unknown } | null)?.type
    if (type === HOST_REQUEST) {
      void this.rpc.handleLine(line).then((reply) => {
        if (reply) this.child?.stdin.write(reply)
      })
      return
    }
    if (type === 'listening') {
      this.write(line)
      return
    }
    this.opts.onMessage?.(message)
  }

  private onExit(child: ChildProcessWithoutNullStreams, code: number | null, spawnedAt: number): void {
    if (this.child === child) {
      this.child = null
      this.opts.onExit?.()
    }
    this.lastExit = code
    this.write(`orrery-core serve exited (code ${code})`)
    if (this.stopping) return
    if (code === PORT_IN_USE_EXIT) {
      if (!this.booting) this.opts.onFatal({ kind: 'port-in-use' })
      return
    }
    if (Date.now() - spawnedAt > STABLE_MS) this.restarts = 0
    if (this.restarts >= MAX_RESTARTS) {
      if (!this.booting) this.opts.onFatal({ kind: 'exited', code })
      return
    }
    this.restarts++
    this.spawn()
    if (this.booting) return
    this.waitHealthy().catch((err: unknown) => {
      if (err instanceof FatalError && !this.stopping) this.opts.onFatal(err.reason)
    })
  }

  private async waitHealthy(): Promise<void> {
    const deadline = Date.now() + (this.opts.bootTimeoutMs ?? BOOT_TIMEOUT_MS)
    while (Date.now() < deadline) {
      if (this.lastExit === PORT_IN_USE_EXIT) throw new FatalError({ kind: 'port-in-use' })
      if (this.lastExit !== undefined && this.restarts >= MAX_RESTARTS) throw new FatalError({ kind: 'exited', code: this.lastExit })
      if (this.child && (await isHealthy(this.opts.port))) return
      await new Promise((r) => setTimeout(r, 300))
    }
    throw new FatalError({ kind: 'timeout' })
  }

  private write(line: string): void {
    this.log?.write(`${line}\n`)
    this.tail.push(line)
    if (this.tail.length > LOG_TAIL) this.tail.shift()
  }
}

export function fallbackReason(err: unknown): string {
  if (!(err instanceof FatalError)) return err instanceof Error ? err.message.slice(0, 200) : 'unknown'
  return err.reason.kind === 'exited' ? `exited${err.reason.code === null ? '' : ` with code ${err.reason.code}`}` : err.reason.kind
}

export class SwitchingHost implements MonitorHost {
  private node: MonitorHost | null = null
  private fell: string | null = null

  constructor(
    private readonly rust: MonitorHost | null,
    private readonly makeNode: () => MonitorHost,
    private readonly onFallback: (reason: string) => void,
  ) {}

  get mode(): CoreServeMode {
    return this.rust && this.fell === null ? 'rust' : 'node'
  }

  get fellBack(): string | null {
    return this.fell
  }

  private get active(): MonitorHost | null {
    return this.mode === 'rust' ? this.rust : this.node
  }

  get ownsPort(): boolean {
    return this.active?.ownsPort ?? false
  }

  get tail(): string[] {
    return this.active?.tail ?? []
  }

  describe(): string {
    if (this.fell !== null) return `core serve: node (rust serve unusable, fell back: ${this.fell})`
    return `core serve: ${this.mode}`
  }

  async start(): Promise<void> {
    if (this.rust && this.fell === null) {
      try {
        await this.rust.start()
        return
      } catch (err) {
        if (err instanceof FatalError && err.reason.kind === 'port-in-use') throw err
        await this.rust.stop().catch(() => undefined)
        this.fell = fallbackReason(err)
        this.onFallback(this.fell)
      }
    }
    this.node ??= this.makeNode()
    await this.node.start()
  }

  async stop(): Promise<void> {
    await this.active?.stop()
  }

  post(message: object): void {
    this.active?.post(message)
  }
}

export interface MonitorHostOptions extends Omit<CollectorHostOptions, 'script'> {
  mode: CoreServeMode
  script: string
  bin: string
  onFallback: (reason: string) => void
}

export function createMonitorHost(o: MonitorHostOptions): SwitchingHost {
  const common = { port: o.port, env: o.env, logFile: o.logFile, onFatal: o.onFatal, onMessage: o.onMessage, onExit: o.onExit }
  const rust = o.mode === 'rust' ? new CoreServeHost({ ...common, bin: o.bin }) : null
  return new SwitchingHost(rust, () => new CollectorHost({ ...common, script: o.script }), o.onFallback)
}
