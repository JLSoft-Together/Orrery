import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'
import type { JobStopResult, MonitorEvent, ServerMessage } from '@ccm/shared'
import type { CollectorConfig } from '../src/config'
import { readDiffStat } from '../src/gitstat'
import { CodexMonitor } from '../src/codex/monitor'
import { DeepseekMonitor } from '../src/deepseek/monitor'
import { Monitor } from '../src/monitor'
import { MonitorStore } from '../src/store'
import type { Trace, TraceStep } from './trace-scenario'

const exe = process.platform === 'win32' ? 'monitor-script.exe' : 'monitor-script'
const target = path.resolve(__dirname, '../../core/target')
export const SCRIPT_BIN = process.env.CCM_MONITOR_SCRIPT?.trim() || [path.join(target, 'debug', exe), path.join(target, 'release', exe)].find((p) => existsSync(p))

export function twinConfig(root: string): CollectorConfig {
  return {
    claudeRoot: root,
    codexHome: root,
    codex: false,
    codewhaleHome: root,
    deepseek: false,
    host: '127.0.0.1',
    port: 0,
    toast: false,
    toastBackend: null,
    appKey: null,
    dev: true,
    toastLink: null,
    staleTtlMs: 120 * 60_000,
    reconcileMs: 5_000,
    processVerifyMs: 60_000,
    activityLimit: 300,
    batchMs: 3_600_000,
    verifyProcesses: false,
    webDist: root,
    dataDir: root,
    devOriginPorts: [],
    statusLineBridge: path.join(root, 'statusline-bridge.mjs'),
    statusLineCommand: 'node statusline-bridge.mjs',
  }
}

class RustSide {
  private readonly child: ChildProcessWithoutNullStreams
  private readonly waiting: ((line: string) => void)[] = []
  private readonly lines: string[] = []

  constructor(bin: string) {
    this.child = spawn(bin, [], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    createInterface({ input: this.child.stdout }).on('line', (l) => {
      const w = this.waiting.shift()
      if (w) w(l)
      else this.lines.push(l)
    })
    this.child.stderr.on('data', () => undefined)
  }

  send(cmd: Record<string, unknown>): Promise<{ ok: boolean; error?: string; result?: unknown; frames: ServerMessage[] }> {
    return new Promise((resolve, reject) => {
      const done = (l: string) => {
        const r = JSON.parse(l) as { ok: boolean; error?: string; result?: unknown; frames: ServerMessage[] }
        if (!r.ok) reject(new Error(`monitor-script ${String(cmd.cmd)}: ${r.error}`))
        else resolve(r)
      }
      const queued = this.lines.shift()
      if (queued !== undefined) done(queued)
      else this.waiting.push(done)
      this.child.stdin.write(`${JSON.stringify(cmd)}\n`)
    })
  }

  close(): void {
    this.child.stdin.end()
    if (this.child.exitCode === null) this.child.kill()
  }
}

export interface TwinAgents {
  codexHome: string
  codewhaleHome: string
  homeDir: string
}

export interface TwinOptions {
  root: string
  now: number
  diff?: boolean
  agents?: TwinAgents
}

export class MonitorTwin {
  readonly store = new MonitorStore({ activityLimit: 300, batchMs: 3_600_000 })
  readonly node: Monitor
  private readonly rust: RustSide
  private readonly pending = new Set<Promise<unknown>>()
  private readonly nodeSteps: TraceStep[] = []
  private readonly rustSteps: TraceStep[] = []
  private nodeStep: TraceStep = { name: '', sent: [], frames: [] }
  private rustStep: TraceStep = { name: '', sent: [], frames: [] }
  private readonly wallStart = Date.now()
  now: number
  stopResult: JobStopResult = 'ok'
  readonly dead = new Set<number>()
  codexProcs: unknown[] | null = []
  deepseekProcs: unknown[] | null = []

  constructor(
    bin: string,
    private readonly options: TwinOptions,
  ) {
    this.now = options.now
    const track = <T>(p: Promise<T>): Promise<T> => {
      this.pending.add(p)
      void p.finally(() => this.pending.delete(p))
      return p
    }
    this.node = new Monitor(twinConfig(options.root), this.store, {
      isPidAlive: (pid) => !this.dead.has(pid),
      queryProcesses: async () => null,
      now: () => this.now,
      diffDelayMs: 0,
      ...(options.diff ? { diffStat: (cwd: string) => track(readDiffStat(cwd)) } : {}),
      stopJob: async () => this.stopResult,
    })
    ;(this.node as unknown as { setupWatchers: () => void }).setupWatchers = () => undefined
    const agents = options.agents
    if (agents) {
      const common = { staleTtlMs: 120 * 60_000, reconcileMs: 5_000, verifyMs: 60_000, verifyProcesses: true }
      const codex = new CodexMonitor(
        { ...common, codexHome: agents.codexHome },
        this.store,
        () => this.node.claudeTargets(),
        { listProcesses: async () => this.codexProcs as never, isPidAlive: (pid) => !this.dead.has(pid), now: () => this.now },
      )
      const deepseek = new DeepseekMonitor(
        { ...common, codewhaleHome: agents.codewhaleHome, homeDir: agents.homeDir },
        this.store,
        { listProcesses: async () => this.deepseekProcs as never, isPidAlive: (pid) => !this.dead.has(pid), now: () => this.now },
      )
      const quiet = (o: object, names: string[]) => names.forEach((n) => ((o as Record<string, unknown>)[n] = () => undefined))
      quiet(codex, ['setupWatchers', 'scheduleScan'])
      quiet(deepseek, ['setupWatchers', 'scheduleScan', 'scheduleRead'])
      this.node.codex = codex
      this.node.deepseek = deepseek
    }
    this.store.subscribe((m) => this.nodeStep.frames.push(m))
    this.rust = new RustSide(bin)
  }

  readonly reply = (event: MonitorEvent): void => {
    this.nodeStep.frames.push({ seq: -1, direct: true, events: [event] })
  }

  async init(): Promise<void> {
    const agents = this.options.agents
    await this.rust.send({
      cmd: 'init',
      claudeRoot: this.options.root,
      dataDir: this.options.root,
      now: this.now,
      diff: this.options.diff === true,
      diffDelayMs: 0,
      batchMs: 3_600_000,
      ...(agents ? { codex: true, codexHome: agents.codexHome, deepseek: true, codewhaleHome: agents.codewhaleHome, homeDir: agents.homeDir } : {}),
    })
  }

  async rustCmd(cmd: Record<string, unknown>): Promise<unknown> {
    const r = await this.rust.send(cmd)
    this.rustStep.frames.push(...r.frames)
    return r.result
  }

  async step(name: string, run: () => Promise<void>): Promise<void> {
    this.nodeStep = { name, sent: [], frames: [] }
    this.rustStep = { name, sent: [], frames: [] }
    this.nodeSteps.push(this.nodeStep)
    this.rustSteps.push(this.rustStep)
    await run()
    this.store.flush()
    await this.rustCmd({ cmd: 'flush' })
  }

  async snapshot(): Promise<void> {
    this.nodeStep.frames.push(this.store.snapshotMessage())
    await this.rustCmd({ cmd: 'snapshot' })
  }

  async setCodexProcs(list: unknown[] | null): Promise<void> {
    this.codexProcs = list
    await this.rustCmd({ cmd: 'codexProcs', list })
  }

  async setDeepseekProcs(list: unknown[] | null): Promise<void> {
    this.deepseekProcs = list
    await this.rustCmd({ cmd: 'deepseekProcs', list })
  }

  async kill(pid: number): Promise<void> {
    this.dead.add(pid)
    await this.rustCmd({ cmd: 'dead', pid })
  }

  async setNow(value: number): Promise<void> {
    this.now = value
    await this.rustCmd({ cmd: 'now', value })
  }

  async wait(): Promise<void> {
    for (let i = 0; i < 50; i++) {
      await new Promise((r) => setTimeout(r, 30))
      if (this.pending.size === 0) break
      await Promise.allSettled([...this.pending])
    }
    await new Promise((r) => setTimeout(r, 30))
    await this.rustCmd({ cmd: 'wait', ms: 15_000 })
  }

  async both(node: () => unknown, rust: Record<string, unknown>): Promise<[unknown, unknown]> {
    const a = await node()
    const b = await this.rustCmd(rust)
    return [a, b]
  }

  trace(side: 'node' | 'rust'): Trace {
    return {
      format: 'ccm-trace/1',
      scenario: 'monitor-twin',
      source: 'node',
      wall: { start: this.wallStart, end: Date.now() },
      pids: {},
      replace: [[this.options.root, '<root>']],
      ignore: [],
      steps: JSON.parse(JSON.stringify(side === 'node' ? this.nodeSteps : this.rustSteps)) as TraceStep[],
    }
  }

  close(): void {
    this.node.stop()
    this.rust.close()
  }
}
