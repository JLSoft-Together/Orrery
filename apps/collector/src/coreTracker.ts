import { reportCoreFallback } from './coreFallback'
import path from 'node:path'
import type { Agent, TelegramAsk, TerminalSession, ToolSpan } from '@ccm/shared'
import type { CoreMessage, CoreTranscriptClient } from './coreTranscript'
import type { PeerInbound, PeerSent } from './peers'
import type { RegistryRecord } from './registry'
import type { ToolTouch } from './sensitive/rules'
import { SpanMirror, isDrop, type SpanEvent } from './spanMirror'
import { SessionTracker, type ActivityDraft, type ContextHints, type SyncScope } from './tracker'
import { buildAgentsOf, buildTerminalOf, codexCallsOf, emptyView, lastDeviceCallOf, lastPortConflictOf, mergeView, type TrackerView, type ViewContext, type ViewDelta } from './trackerView'

export interface CoreHub {
  client(): CoreTranscriptClient | null
  nextKey(): string
  register(tracker: object, key: string): void
  unregister(tracker: object): void
  log(line: string): void
}

interface SyncPlan {
  all: boolean
  unresolved: boolean
  files: Set<string>
}

const PAGE_TIMEOUT_MS = 30_000
const PAGE_BYTES = 4 * 1024 * 1024
const REQUEST = { timeoutMs: PAGE_TIMEOUT_MS, restartOnTimeout: false }
export interface CoreTrackerOptions {
  safetyNetMs?: number
  clock?: () => number
}

const ALL: SyncPlan = { all: true, unresolved: false, files: new Set() }

type Draft = ActivityDraft & { touch?: ToolTouch }

function sameRecord(a: RegistryRecord, b: RegistryRecord): boolean {
  const x = a as unknown as Record<string, unknown>
  const y = b as unknown as Record<string, unknown>
  for (const k in x) if (x[k] !== y[k]) return false
  for (const k in y) if (!(k in x) && y[k] !== undefined) return false
  return true
}

export class CoreTracker {
  record: RegistryRecord
  shell?: string
  parentPid?: number
  endedAt?: string

  private key: string | null = null
  private opened = false
  private disposed = false
  private epoch = 0
  private view: TrackerView = emptyView()
  private readonly mirror: SpanMirror
  private pendingSpans: ToolSpan[] = []
  private pendingPeers: { inbound: PeerInbound[]; sent: PeerSent[] } = { inbound: [], sent: [] }
  private syncing: Promise<ActivityDraft[]> | null = null
  private syncAgain = false
  private plan: SyncPlan = { all: false, unresolved: false, files: new Set() }
  private fallback: SessionTracker | null = null
  private changed = true
  private lastSyncAt = Number.NEGATIVE_INFINITY

  constructor(
    readonly terminalId: string,
    record: RegistryRecord,
    private readonly projectsDir: string,
    private readonly hints: ContextHints,
    private readonly now: () => number,
    private readonly hub: CoreHub,
    private readonly opts: CoreTrackerOptions = {},
  ) {
    this.record = record
    this.mirror = new SpanMirror(now)
  }

  get usingNode(): boolean {
    return this.fallback !== null
  }

  private fb(): SessionTracker | null {
    const t = this.fallback
    if (!t) return null
    t.shell = this.shell
    t.parentPid = this.parentPid
    return t
  }

  get sessionId(): string | undefined {
    return this.record.sessionId
  }

  get mainAgentId(): string {
    return `${this.terminalId}:main`
  }

  get ended(): boolean {
    return this.endedAt !== undefined
  }

  get lastReply(): string | undefined {
    const t = this.fb()
    return t ? t.lastReply : this.view.main.lastReply
  }

  get pendingAsk(): TelegramAsk | undefined {
    const t = this.fb()
    return t ? t.pendingAsk : this.view.main.pendingAsk
  }

  setRecord(record: RegistryRecord): { sessionChanged: boolean } {
    const sessionChanged = record.sessionId !== this.record.sessionId
    if (!sameRecord(record, this.record)) this.changed = true
    this.record = record
    this.fallback?.setRecord(record)
    if (sessionChanged) {
      this.epoch++
      this.view = emptyView()
      this.lastSyncAt = Number.NEGATIVE_INFINITY
    }
    return { sessionChanged }
  }

  markEnded(at: string): void {
    if (this.endedAt !== at) this.changed = true
    this.endedAt = at
    this.fallback?.markEnded(at)
  }

  takeChanged(): boolean {
    const changed = this.changed || this.fallback !== null
    this.changed = false
    return changed
  }

  drainSpans(): ToolSpan[] {
    const out = this.pendingSpans
    this.pendingSpans = []
    return this.fallback ? [...out, ...this.fallback.drainSpans()] : out
  }

  drainPeers(): { inbound: PeerInbound[]; sent: PeerSent[] } {
    const out = this.pendingPeers
    this.pendingPeers = { inbound: [], sent: [] }
    if (!this.fallback) return out
    const more = this.fallback.drainPeers()
    return { inbound: [...out.inbound, ...more.inbound], sent: [...out.sent, ...more.sent] }
  }

  closeOpenSpans(at: string): ToolSpan[] {
    if (this.fallback) return this.fallback.closeOpenSpans(at)
    const changed = this.mirror.closeOpen(at)
    if (this.key) this.hub.client()?.send('tracker.closeSpans', { key: this.key, at })
    return changed
  }

  toolSpans(sinceMs?: number): ToolSpan[] {
    return this.fallback ? this.fallback.toolSpans(sinceMs) : this.mirror.list(sinceMs)
  }

  codexCalls(): { agentId: string; at: string }[] {
    return this.fallback ? this.fallback.codexCalls() : codexCallsOf(this.view, this.mainAgentId)
  }

  lastDeviceCallAt(): string | undefined {
    return this.fallback ? this.fallback.lastDeviceCallAt() : lastDeviceCallOf(this.view)
  }

  lastPortConflict(): { port: number; at: string } | undefined {
    return this.fallback ? this.fallback.lastPortConflict() : lastPortConflictOf(this.view)
  }

  private ctx(): ViewContext {
    return { terminalId: this.terminalId, record: this.record, shell: this.shell, parentPid: this.parentPid, endedAt: this.endedAt, hints: this.hints, view: this.view }
  }

  buildTerminal(): TerminalSession {
    const t = this.fb()
    return t ? t.buildTerminal() : buildTerminalOf(this.ctx())
  }

  buildAgents(terminal: TerminalSession): Agent[] {
    const t = this.fb()
    return t ? t.buildAgents(terminal) : buildAgentsOf(this.ctx(), terminal)
  }

  sync(catchUp: boolean, scope: SyncScope = {}): Promise<ActivityDraft[]> {
    if (!catchUp && scope.skipResolved && !scope.files && this.recentlySynced()) return Promise.resolve([])
    if (scope.files) for (const f of scope.files) this.plan.files.add(path.basename(f))
    else if (scope.skipResolved) this.plan.unresolved = true
    else this.plan.all = true
    if (this.syncing) {
      this.syncAgain = true
      return this.syncing.then(() => [])
    }
    this.syncing = (async () => {
      const out: ActivityDraft[] = []
      do {
        this.syncAgain = false
        const plan = this.plan
        this.plan = { all: false, unresolved: false, files: new Set() }
        out.push(...(await this.syncOnce(catchUp, plan)))
        if (plan.all || plan.unresolved) this.lastSyncAt = this.clock()
      } while (this.syncAgain)
      return out
    })().finally(() => {
      this.syncing = null
    })
    return this.syncing
  }

  private clock(): number {
    return this.opts.clock ? this.opts.clock() : performance.now()
  }

  private recentlySynced(): boolean {
    const ms = this.opts.safetyNetMs
    if (!ms || !this.opened || this.fallback || this.syncing || this.disposed) return false
    return this.clock() - this.lastSyncAt < ms
  }

  private scopeOf(plan: SyncPlan): SyncScope {
    return plan.all || (plan.unresolved && plan.files.size > 0) ? {} : plan.unresolved ? { skipResolved: true } : { files: [...plan.files] }
  }

  private async syncOnce(catchUp: boolean, plan: SyncPlan): Promise<ActivityDraft[]> {
    if (this.fallback) return this.fallback.sync(catchUp, this.scopeOf(plan))
    if (this.disposed) return []
    const fresh = !this.opened
    const kept: ActivityDraft[] = []
    if (this.key) {
      const r = await this.run(catchUp, plan)
      if (r.ok) return r.out
      kept.push(...r.out)
    }
    const before = fresh ? null : this.mirror.all()
    if (await this.open()) {
      const r = fresh ? await this.run(catchUp, plan) : await this.run(true, ALL)
      if (r.ok) {
        if (fresh) return r.out
        this.queueRecovered(before!, this.mirror.all())
        return kept
      }
      kept.push(...r.out)
    }
    const t = this.toNode()
    if (!before) return [...kept, ...(await t.sync(catchUp, this.scopeOf(plan)))]
    await t.sync(true)
    this.queueRecovered(before, t.toolSpans())
    return kept
  }

  private queueRecovered(before: ToolSpan[], after: ToolSpan[]): void {
    const old = new Map(before.map((s) => [s.id, s]))
    for (const s of after) {
      const prev = old.get(s.id)
      if (!prev || prev.end !== s.end || prev.outcome !== s.outcome) this.pendingSpans.push({ ...s })
    }
  }

  dispose(): void {
    if (this.disposed) return
    this.disposed = true
    if (this.key) this.hub.client()?.send('tracker.close', { key: this.key })
    this.hub.unregister(this)
    this.key = null
  }

  private async open(): Promise<boolean> {
    const client = this.hub.client()
    if (!client) return false
    if (this.key) client.send('tracker.close', { key: this.key })
    const key = this.hub.nextKey()
    const r = await client.request('tracker.open', { key, terminalId: this.terminalId, projectsDir: this.projectsDir }, REQUEST)
    if (!r) return false
    this.hub.unregister(this)
    this.hub.register(this, key)
    this.key = key
    this.opened = true
    this.mirror.clear()
    return true
  }

  private toNode(): SessionTracker {
    this.hub.log(`[ccm] core-transcript tracker unavailable for ${this.terminalId}, using the Node session tracker`)
    reportCoreFallback('transcript', 'core-transcript tracker unavailable')
    this.hub.unregister(this)
    this.key = null
    const t = new SessionTracker(this.terminalId, this.record, this.projectsDir, this.hints, this.now)
    t.shell = this.shell
    t.parentPid = this.parentPid
    if (this.endedAt !== undefined) t.markEnded(this.endedAt)
    this.fallback = t
    return t
  }

  private async run(catchUp: boolean, plan: SyncPlan): Promise<{ out: ActivityDraft[]; ok: boolean }> {
    const client = this.hub.client()
    if (!client || !this.key) return { out: [], ok: false }
    const epoch = this.epoch
    const args = {
      key: this.key,
      sessionId: this.record.sessionId ?? null,
      cwd: this.record.cwd ?? null,
      epoch,
      catchUp,
      plan: { all: plan.all, unresolved: plan.unresolved, files: [...plan.files] },
      maxBytes: PAGE_BYTES,
    }
    const out: ActivityDraft[] = []
    for (;;) {
      const r = await client.request('tracker.sync', { ...args, nowMs: this.now() }, REQUEST)
      if (!r || this.disposed) return { out, ok: !!r }
      this.apply(r, catchUp, out, epoch)
      if (r.more !== true) return { out, ok: true }
    }
  }

  private apply(r: CoreMessage, catchUp: boolean, out: ActivityDraft[], epoch: number): void {
    for (const e of (r.spans ?? []) as SpanEvent[]) {
      this.mirror.apply(e)
      if (!catchUp && !isDrop(e)) this.pendingSpans.push({ ...e })
    }
    this.pendingPeers.inbound.push(...((r.inbound ?? []) as PeerInbound[]))
    this.pendingPeers.sent.push(...((r.sent ?? []) as PeerSent[]))
    for (const { touch, ...d } of (r.activity ?? []) as Draft[]) {
      out.push(d)
      if (!touch || !this.hints.sensitive) continue
      for (const h of this.hints.sensitive.check(this.terminalId, touch, this.record.cwd)) {
        out.push({ kind: 'sensitive', terminalId: this.terminalId, agentId: d.agentId, at: d.at, data: { tool: d.data?.tool, rule: h.rule, path: h.path, file: h.file, project: h.project } })
      }
    }
    if (r.view && epoch === this.epoch) {
      this.view = mergeView(this.view, r.view as unknown as ViewDelta)
      this.changed = true
    }
  }
}
