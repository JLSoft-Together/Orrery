import { randomBytes } from 'node:crypto'
import { spawn } from 'node:child_process'
import { createWriteStream, existsSync, type WriteStream } from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { app, utilityProcess } from 'electron'
import { MCP_TOOL_ACTION, type OrreryCaller } from '../../../../packages/shared/src/orreryApi'
import { PTY_PROTOCOL_PREFIX, type PaneOpenRequest, type PaneOpenResult, type PtyEndpoint, type PtySessionInfo } from '../../../../packages/shared/src/pty'
import { encodeControl, invalidCensusRows, NdjsonReader, parseCoreToMain, type PtyMainToCore } from '../../../../packages/shared/src/ptyControl'
import type { McpDeskCall } from '../../../../packages/shared/src/mcpDesk'
import type { McpPageCall } from '../../../../packages/shared/src/mcpCall'
import { paneOpenResult } from '../openLink'
import { PtyBroker } from './broker'
import { mcpHttpHandler } from './mcp'
import { mcpToolsFor } from './mcpTools'

export interface PtySupervisorOptions {
  script: string
  nodePty: string
  origins: string[]
  logFile: string
  /** zsh startup wrappers that report the current folder (posix only). */
  shellInitDir?: string
  /** Orrery dispatcher: `open_session` and every MCP tool the host cannot answer alone; the host re-checks each reply. */
  onOrrery?: (action: string, args: unknown, caller: OrreryCaller) => Promise<unknown>
  mcpConfig?: string
  core?: string
}

export type PtyHostKind = 'node' | 'rust'

export interface PtyHost {
  endpoint(): Promise<PtyEndpoint>
  census(): Promise<PtySessionInfo[] | null>
  stop(): Promise<void>
}

export interface HostChild {
  post(msg: PtyMainToCore): void
  onMessage(cb: (data: unknown) => void): void
  onExit(cb: (code: number | null) => void): void
  onLog(cb: (chunk: Buffer | string) => void): void
  kill(): void
}

export interface HostLauncher {
  readonly kind: PtyHostKind
  describe(): string
  launch(): HostChild
}

export class NodeHost implements HostLauncher {
  readonly kind = 'node'

  constructor(
    private readonly script: string,
    private readonly nodePty: string,
  ) {}

  describe(): string {
    return `node (${this.script})`
  }

  launch(): HostChild {
    const env: NodeJS.ProcessEnv = {}
    for (const [k, v] of Object.entries(process.env)) if (k.toUpperCase() !== 'CCM_PTY_CONTROL' && k.toUpperCase() !== 'CCM_NODE_PTY') env[k] = v
    env.CCM_NODE_PTY = this.nodePty
    const child = utilityProcess.fork(this.script, [], { env, stdio: 'pipe', serviceName: 'Orrery terminals' })
    return {
      post: (msg) => child.postMessage(msg),
      onMessage: (cb) => void child.on('message', cb),
      onExit: (cb) => void child.once('exit', (code) => cb(code)),
      onLog: (cb) => {
        child.stdout?.on('data', cb)
        child.stderr?.on('data', cb)
      },
      kill: () => void child.kill(),
    }
  }
}

const CORE_ENV_KEYS = new Set(['PATH', 'SYSTEMROOT', 'WINDIR', 'COMSPEC', 'TEMP', 'TMP', 'TMPDIR', 'HOME', 'USERPROFILE', 'LANG', 'LC_ALL'])

export function coreEnv(source: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = {}
  for (const [k, v] of Object.entries(source)) if (v !== undefined && CORE_ENV_KEYS.has(k.toUpperCase())) env[k] = v
  return env
}

export function coreBinaryPath(opts: { env: NodeJS.ProcessEnv; packaged: boolean; resourcesPath: string; repo: string; platform: NodeJS.Platform }): string {
  const override = opts.env.CCM_CORE_BIN?.trim()
  if (override) return override
  const exe = opts.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
  return opts.packaged ? path.join(opts.resourcesPath, 'core', exe) : path.join(opts.repo, 'apps', 'core', 'target', 'release', exe)
}

export class RustHost implements HostLauncher {
  readonly kind = 'rust'

  constructor(private readonly bin: string) {}

  describe(): string {
    return `rust (${this.bin})`
  }

  launch(): HostChild {
    if (!existsSync(this.bin)) throw new Error(`orrery-core not found at ${this.bin} (build apps/core or unset CCM_CORE_PTY)`)
    const proc = spawn(this.bin, [], { env: coreEnv(process.env), stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    let exited = false
    const exitCbs: ((code: number | null) => void)[] = []
    const finish = (code: number | null): void => {
      if (exited) return
      exited = true
      for (const cb of exitCbs) cb(code)
    }
    proc.once('close', (code) => finish(code))
    proc.once('error', () => finish(null))
    proc.stdout.once('end', () => {
      if (!exited) proc.kill()
    })
    proc.stdin.on('error', () => undefined)
    return {
      post: (msg) => {
        if (!exited && proc.stdin.writable) proc.stdin.write(encodeControl(msg))
      },
      onMessage: (cb) => {
        const reader = new NdjsonReader(cb)
        proc.stdout.setEncoding('utf8')
        proc.stdout.on('data', (chunk: string) => reader.push(chunk))
      },
      onExit: (cb) => void exitCbs.push(cb),
      onLog: (cb) => {
        proc.stderr.on('data', cb)
      },
      kill: () => void proc.kill(),
    }
  }
}

const START_TIMEOUT_MS = 10_000
const CENSUS_TIMEOUT_MS = 1_500
const STOP_TIMEOUT_MS = 3_000

/**
 * Owns the PTY utility process. Started lazily on the first terminal so users who never open the
 * workspace pay nothing; the token is fresh per process and only reaches pages through the preload.
 */
export class PtySupervisor implements PtyHost {
  private child: HostChild | null = null
  private readonly launcher: HostLauncher
  private broker: PtyBroker | null = null
  private starting: Promise<PtyEndpoint> | null = null
  private log: WriteStream | null = null
  private token = ''
  private mcp: http.Server | null = null
  private mcpPort = 0
  private mcpStarting: Promise<void> | null = null
  private censusWaiters: ((sessions: PtySessionInfo[]) => void)[] = []
  private stopped = false

  constructor(private readonly opts: PtySupervisorOptions) {
    this.launcher = opts.core ? new RustHost(opts.core) : new NodeHost(opts.script, opts.nodePty)
  }

  get kind(): PtyHostKind {
    return this.launcher.kind
  }

  endpoint(): Promise<PtyEndpoint> {
    if (this.stopped) return Promise.reject(new Error('pty host stopped'))
    this.starting ??= this.start().catch((err: unknown) => {
      this.starting = null
      throw err
    })
    return this.starting
  }

  private logLine(line: string): void {
    this.log?.write(`${line}\n`)
  }

  private startMcp(): Promise<void> {
    const server = http.createServer(
      mcpHttpHandler({
        port: () => this.mcpPort,
        caller: (token) => this.broker?.toolHost(token) ?? null,
        version: app.getVersion(),
        tools: mcpToolsFor(process.env.CCM_DEEPSEEK_FEATURE === '1'),
        log: (line) => this.logLine(line),
      }),
    )
    this.mcp = server
    return new Promise((resolve) => {
      const onError = (err: Error): void => {
        this.logLine(`mcp server failed, orrery MCP off for this host: ${String(err)}`)
        this.mcpPort = 0
        this.mcpStarting = null
        if (this.mcp === server) this.mcp = null
        server.close()
        resolve()
      }
      server.once('error', onError)
      server.listen(0, '127.0.0.1', () => {
        server.off('error', onError)
        if (this.stopped) {
          server.close()
          this.mcpPort = 0
          return resolve()
        }
        const addr = server.address()
        this.mcpPort = typeof addr === 'object' && addr ? addr.port : 0
        server.on('error', (err) => this.logLine(`mcp server error: ${String(err)}`))
        resolve()
      })
    })
  }

  private async start(): Promise<PtyEndpoint> {
    this.token = randomBytes(32).toString('hex')
    this.log ??= createWriteStream(this.opts.logFile, { flags: 'w' })
    await (this.mcpStarting ??= this.startMcp())
    if (this.stopped) throw new Error('pty host stopped')
    this.logLine(`pty host: ${this.launcher.describe()}`)
    let child: HostChild
    try {
      child = this.launcher.launch()
    } catch (err) {
      this.logLine(`pty host did not start: ${String(err)}`)
      throw err
    }
    child.onLog((buf) => void this.log?.write(buf))
    this.child = child
    const post = (msg: PtyMainToCore): void => {
      if (this.child === child) child.post(msg)
    }
    this.broker?.dispose()
    const broker = new PtyBroker({
      send: post,
      port: () => this.mcpPort,
      askPage: (req) => this.askPage(req),
      page: (call) => this.ask(call),
      desk: (call) => this.ask(call),
      shellInitDir: this.opts.shellInitDir,
      mcpConfig: this.opts.mcpConfig,
      log: (line) => this.logLine(line),
    })
    this.broker = broker
    child.post({ t: 'init', token: this.token, origins: this.opts.origins, appVersion: app.getVersion() })
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        child.kill()
        this.logLine('pty host did not report listening within 10s')
        reject(new Error('pty host did not start'))
      }, START_TIMEOUT_MS)
      child.onMessage((data) => {
        const msg = parseCoreToMain(data)
        if (!msg) {
          const bad = invalidCensusRows(data)
          if (bad !== null) this.log?.write(`pty census reply dropped: ${bad < 0 ? 'no session list' : `${bad} invalid rows`}\n`)
          return
        }
        if (msg.t === 'listening') {
          clearTimeout(timer)
          resolve({ url: `ws://127.0.0.1:${msg.port}/pty`, protocol: `${PTY_PROTOCOL_PREFIX}${this.token}` })
        } else if (msg.t === 'census-reply') {
          const waiters = this.censusWaiters
          this.censusWaiters = []
          for (const w of waiters) w(msg.sessions)
        } else broker.handle(msg)
      })
      child.onExit((code) => {
        clearTimeout(timer)
        this.log?.write(`pty host exited (code ${code})\n`)
        broker.dispose()
        if (this.child === child) {
          this.child = null
          this.starting = null
          this.broker = null
        }
        for (const w of this.censusWaiters.splice(0)) w([])
        reject(new Error(`pty host exited (${code})`))
      })
    })
  }

  private askPage(req: PaneOpenRequest): Promise<PaneOpenResult> {
    return Promise.resolve()
      .then(() => this.opts.onOrrery?.('session.open', req, { source: 'mcp', ...(req.from ? { pane: req.from } : {}) }) ?? { ok: false, error: 'unavailable' })
      .then(paneOpenResult, () => paneOpenResult(null))
  }

  private ask(call: McpPageCall | McpDeskCall): Promise<unknown> {
    const action = Object.hasOwn(MCP_TOOL_ACTION, call.tool) ? MCP_TOOL_ACTION[call.tool as keyof typeof MCP_TOOL_ACTION] : null
    const caller: OrreryCaller = { source: 'mcp', ...(call.from ? { pane: call.from } : {}) }
    return Promise.resolve()
      .then(() => (action && this.opts.onOrrery?.(action, call, caller)) || { ok: false, error: action ? 'unavailable' : 'failed' })
      .catch(() => ({ ok: false, error: 'failed' }))
  }

  /** Live terminals, for the quit confirmation. Empty when the host never started; null when it did not answer. */
  census(): Promise<PtySessionInfo[] | null> {
    const child = this.child
    if (!child) return Promise.resolve([])
    return new Promise((resolve) => {
      const timer = setTimeout(() => resolve(null), CENSUS_TIMEOUT_MS)
      this.censusWaiters.push((sessions) => {
        clearTimeout(timer)
        resolve(sessions.filter((s) => s.alive))
      })
      child.post({ t: 'census' })
    })
  }

  async stop(): Promise<void> {
    this.stopped = true
    this.mcp?.close()
    this.mcp?.closeAllConnections()
    this.mcp = null
    const child = this.child
    if (!child) return
    const exited = new Promise<void>((resolve) => child.onExit(() => resolve()))
    child.post({ t: 'shutdown' })
    const timer = setTimeout(() => child.kill(), STOP_TIMEOUT_MS)
    await Promise.race([exited, new Promise((r) => setTimeout(r, STOP_TIMEOUT_MS + 1_000))])
    clearTimeout(timer)
    this.log?.end()
    this.log = null
  }
}
