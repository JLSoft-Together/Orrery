import { reportCoreFallback } from './coreFallback'
import path from 'node:path'
import type { ToolStats, UsageBucket, UsageHeatmap, UsageScan, UsageSnapshot } from '@ccm/shared'
import { createCoreTranscript, type CoreMessage, type CoreTranscriptClient } from './coreTranscript'
import { UsageIndex, type UsageSink } from './usage'

export type UsageMode = 'rust' | 'node'

export type UsageOptions = NonNullable<ConstructorParameters<typeof UsageIndex>[3]>

export interface UsageSource {
  start(): Promise<void>
  stop(): Promise<void>
  notify(file: string): void
  whenIdle(): Promise<void>
  scanInfo(): UsageScan
  heatmap(days: number, today?: string): UsageHeatmap | Promise<UsageHeatmap>
  toolStats(days: number, project: string | null, today?: string): ToolStats | Promise<ToolStats>
}

export type ClientFactory = (hooks: { onEvent: (e: CoreMessage) => void; onDown: () => void }) => CoreTranscriptClient | null

export function usageMode(env: NodeJS.ProcessEnv = process.env): UsageMode {
  return env.CCM_CORE_USAGE?.trim().toLowerCase() === 'node' ? 'node' : 'rust'
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

function scanOf(v: unknown): UsageScan | null {
  if (!isObj(v) || typeof v.state !== 'string' || typeof v.filesDone !== 'number' || typeof v.filesTotal !== 'number') return null
  return { state: v.state as UsageScan['state'], filesDone: v.filesDone, filesTotal: v.filesTotal }
}

function snapshotOf(v: unknown): UsageSnapshot | null {
  if (!isObj(v) || !Array.isArray(v.buckets)) return null
  const scan = scanOf(v.scan)
  return scan ? { buckets: v.buckets.filter(isObj) as unknown as UsageBucket[], scan } : null
}

const QUERY = { restartOnTimeout: false }

const emptyStats = (days: number, project: string | null): ToolStats => ({ days, project, projects: [], tools: [], skills: [], agents: [], mcp: [], used30: { skills: [], mcp: [] } })

export class RustUsageSource implements UsageSource {
  private client: CoreTranscriptClient | null = null
  private node: UsageIndex | null = null
  private scan: UsageScan = { state: 'idle', filesDone: 0, filesTotal: 0 }
  private stopped = false
  private recovering: Promise<void> | null = null

  constructor(
    private readonly projectsDir: string,
    private readonly cacheFile: string | null,
    private readonly sink: UsageSink,
    private readonly options: UsageOptions,
    private readonly makeClient: ClientFactory,
    private readonly log: (line: string) => void = (l) => console.warn(l),
  ) {}

  get usingNode(): boolean {
    return this.node !== null
  }

  async start(): Promise<void> {
    this.client = this.makeClient({ onEvent: (e) => this.onEvent(e), onDown: () => this.onDown() })
    if (!(await this.open())) await this.fallback('core-transcript did not open the usage index')
  }

  private async open(): Promise<boolean> {
    if (!this.client || this.stopped) return false
    const o = this.options
    const reply = await this.client.request(
      'usage.scan',
      { projectsDir: this.projectsDir, cacheFile: this.cacheFile, sweepMs: o.sweepMs, fileDebounceMs: o.fileDebounceMs, saveDebounceMs: o.saveDebounceMs, idRetentionMs: o.idRetentionMs },
      { timeoutMs: 120_000 },
    )
    const snap = snapshotOf(reply?.snapshot)
    if (!snap || this.stopped) return false
    this.scan = snap.scan
    this.sink.resetUsage(snap)
    return true
  }

  private async fallback(why: string): Promise<void> {
    if (this.node || this.stopped) return
    this.log(`[ccm] ${why}, using the Node usage index`)
    reportCoreFallback('usage', why)
    this.client?.dispose()
    this.client = null
    this.node = new UsageIndex(this.projectsDir, this.cacheFile, this.sink, this.options)
    await this.node.start()
  }

  private onEvent(e: CoreMessage): void {
    if (this.node || this.stopped) return
    if (e.event === 'usage.scan') {
      const scan = scanOf(e.scan)
      if (!scan) return
      this.scan = scan
      this.sink.setUsageScan({ ...scan })
    } else if (e.event === 'usage.reset') {
      const snap = snapshotOf(e.snapshot)
      if (!snap) return
      this.scan = snap.scan
      this.sink.resetUsage(snap)
    } else if (e.event === 'usage.update' && Array.isArray(e.buckets)) {
      this.sink.updateUsage(e.buckets.filter(isObj) as unknown as UsageBucket[])
    }
  }

  private onDown(): void {
    if (this.stopped || this.node || this.recovering) return
    this.recovering = (async () => {
      if (!(await this.open())) await this.fallback('core-transcript stopped')
    })().finally(() => {
      this.recovering = null
    })
  }

  notify(file: string): void {
    if (this.stopped) return
    if (this.node) return this.node.notify(file)
    if (!file.endsWith('.jsonl')) return
    this.client?.send('usage.notify', { files: [file.split(path.sep).join('/')] })
  }

  scanInfo(): UsageScan {
    return this.node ? this.node.scanInfo() : { ...this.scan }
  }

  private async query(q: Record<string, unknown>): Promise<CoreMessage | null> {
    if (this.recovering) await this.recovering
    if (this.node || !this.client) return null
    const reply = await this.client.request('usage.query', q, QUERY)
    if (reply || !this.recovering) return reply
    await this.recovering
    return this.node || !this.client ? null : this.client.request('usage.query', q, QUERY)
  }

  async heatmap(days: number, today?: string): Promise<UsageHeatmap> {
    const reply = this.node ? null : await this.query({ q: 'heatmap', days, today })
    if (this.node) return this.node.heatmap(days, today)
    return isObj(reply?.heatmap) ? (reply.heatmap as unknown as UsageHeatmap) : { days: [] }
  }

  async toolStats(days: number, project: string | null, today?: string): Promise<ToolStats> {
    const reply = this.node ? null : await this.query({ q: 'toolStats', days, project, today })
    if (this.node) return this.node.toolStats(days, project, today)
    return isObj(reply?.toolStats) ? (reply.toolStats as unknown as ToolStats) : emptyStats(days, project)
  }

  async whenIdle(): Promise<void> {
    if (this.recovering) await this.recovering
    if (this.node) return this.node.whenIdle()
    await this.client?.request('usage.idle', {}, { timeoutMs: 600_000, restartOnTimeout: false })
  }

  async stop(): Promise<void> {
    if (this.stopped) return
    if (this.node) {
      this.stopped = true
      return this.node.stop()
    }
    this.stopped = true
    const client = this.client
    if (client) await client.request('usage.stop', {}, { timeoutMs: 15_000, restartOnTimeout: false })
    client?.dispose()
    this.client = null
  }
}

export function createUsageSource(projectsDir: string, cacheFile: string | null, sink: UsageSink, options: UsageOptions = {}, env: NodeJS.ProcessEnv = process.env): UsageSource {
  if (usageMode(env) !== 'rust') return new UsageIndex(projectsDir, cacheFile, sink, options)
  const bin = env.CCM_CORE_BIN?.trim()
  if (!bin) {
    console.warn('[ccm] CCM_CORE_BIN is not set, using the Node usage index')
    reportCoreFallback('usage', 'CCM_CORE_BIN is not set')
    return new UsageIndex(projectsDir, cacheFile, sink, options)
  }
  return new RustUsageSource(projectsDir, cacheFile, sink, options, (hooks) => createCoreTranscript({ bin, ...hooks }))
}

export function whenReady<T>(value: T | Promise<T>, use: (v: T) => void): void {
  if (value instanceof Promise) void value.then(use)
  else use(value)
}
