import { applyJsonPatch, deepCopy, PatchError } from './jsonPatch'
import { applyMonitorEvent, type MonitorSnapshot } from './monitorReducer'
import type { JsonMap, LinkState } from './types'

export interface ConfirmAsk {
  cid: string
  title: string
  detail?: string
  ok: string
  cancel: string
  receivedAt: number
}

export type WorkspaceDoc = Record<string, unknown> & { tabs?: unknown[] }

export interface MachineView {
  snapshot: MonitorSnapshot | null
  seq: number
  workspace: WorkspaceDoc | null
  wsRev: number
  confirms: ConfirmAsk[]
  hello: JsonMap | null
  stale: boolean
  fromCache: boolean
  updatedAt: number | null
  revision: number
}

export interface CacheEntry {
  savedAt: number
  seq: number
  wsRev: number
  snapshot: MonitorSnapshot | null
  workspace: WorkspaceDoc | null
  hello: JsonMap | null
}

export interface SnapshotCache {
  load(machineId: string): Promise<CacheEntry | null>
  save(machineId: string, entry: CacheEntry): Promise<void>
  delete(machineId: string): Promise<void>
}

export type Commander = (type: string, fields?: JsonMap) => Promise<JsonMap>

export interface MachineDataOptions {
  machineId: string
  commander: Commander
  cache?: SnapshotCache
  now?: () => number
  saveDelayMs?: number
  confirmTtlMs?: number
}

export const EMPTY_VIEW: MachineView = {
  snapshot: null,
  seq: 0,
  workspace: null,
  wsRev: 0,
  confirms: [],
  hello: null,
  stale: true,
  fromCache: false,
  updatedAt: null,
  revision: 0,
}

const isMap = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)

export class MachineData {
  readonly machineId: string
  onChange: (view: MachineView) => void = () => {}
  private readonly commander: Commander
  private readonly cache?: SnapshotCache
  private readonly now: () => number
  private readonly saveDelayMs: number
  private readonly confirmTtlMs: number
  private current: MachineView = EMPTY_VIEW
  private resyncPending = false
  private wsGetPending = false
  private saveTimer: ReturnType<typeof setTimeout> | null = null
  private readonly confirmTimers = new Map<string, ReturnType<typeof setTimeout>>()

  constructor(options: MachineDataOptions) {
    this.machineId = options.machineId
    this.commander = options.commander
    this.cache = options.cache
    this.now = options.now ?? Date.now
    this.saveDelayMs = options.saveDelayMs ?? 2000
    this.confirmTtlMs = options.confirmTtlMs ?? 60_000
  }

  get view(): MachineView {
    return this.current
  }

  private set(patch: Partial<MachineView>): void {
    this.current = { ...this.current, ...patch, revision: this.current.revision + 1 }
    this.onChange(this.current)
  }

  async restoreFromCache(): Promise<void> {
    const entry = await this.cache?.load(this.machineId)
    if (!entry || this.current.snapshot || this.current.workspace) return
    this.set({ snapshot: entry.snapshot, seq: entry.seq, workspace: entry.workspace, wsRev: entry.wsRev, hello: entry.hello, stale: true, fromCache: true, updatedAt: entry.savedAt })
  }

  onLink(link: LinkState): void {
    if (link.status === 'ready' || this.current.stale) return
    void this.flush()
    this.set({ stale: true })
  }

  handleEvent(event: JsonMap): void {
    switch (event.t) {
      case 'hello': {
        const { re: _re, ok: _ok, ...rest } = event
        this.set({ hello: rest })
        break
      }
      case 'snapshot':
        this.onSnapshot(event)
        break
      case 'events':
        this.onEvents(event)
        break
      case 'ws.snapshot':
        this.onWsSnapshot(event)
        break
      case 'ws.patch':
        this.onWsPatch(event)
        break
      case 'confirm.ask':
        this.onConfirm(event)
        break
      default:
        return
    }
    this.scheduleSave()
  }

  private onSnapshot(event: JsonMap): void {
    if (!isMap(event.data) || typeof event.seq !== 'number') return
    this.resyncPending = false
    this.set({ snapshot: deepCopy(event.data), seq: event.seq, stale: false, fromCache: false, updatedAt: this.now() })
  }

  private onEvents(event: JsonMap): void {
    if (typeof event.seq !== 'number' || !Array.isArray(event.events)) return
    const snap = this.current.snapshot
    if (!snap || this.current.fromCache) {
      this.requestResync()
      return
    }
    const direct = event.direct === true
    if (!direct) {
      if (event.seq <= this.current.seq) return
      if (event.seq !== this.current.seq + 1) {
        this.requestResync()
        return
      }
    }
    const next = { ...snap }
    for (const item of event.events) applyMonitorEvent(next, item)
    this.set({ snapshot: next, seq: direct ? this.current.seq : event.seq, stale: false, updatedAt: this.now() })
  }

  private requestResync(): void {
    if (this.resyncPending) return
    this.resyncPending = true
    this.commander('resync').then(
      () => {},
      () => {
        this.resyncPending = false
      },
    )
  }

  private onWsSnapshot(event: JsonMap): void {
    if (typeof event.rev !== 'number') return
    this.wsGetPending = false
    const { t: _t, ...rest } = event
    this.set({ workspace: deepCopy(rest) as WorkspaceDoc, wsRev: event.rev, updatedAt: this.now() })
  }

  private onWsPatch(event: JsonMap): void {
    const { rev, ops, base } = event
    if (typeof rev !== 'number' || !Array.isArray(ops)) return
    const workspace = this.current.workspace
    if (!workspace) {
      this.requestWsGet()
      return
    }
    if (rev <= this.current.wsRev) return
    const lost = typeof base === 'number' ? base !== this.current.wsRev : rev > this.current.wsRev + 1
    if (lost) {
      this.requestWsGet()
      return
    }
    try {
      const next = applyJsonPatch(workspace, ops)
      if (!isMap(next)) throw new PatchError('root')
      this.set({ workspace: { ...next, rev }, wsRev: rev, updatedAt: this.now() })
    } catch (error) {
      if (!(error instanceof PatchError)) throw error
      this.requestWsGet()
    }
  }

  private requestWsGet(): void {
    if (this.wsGetPending) return
    this.wsGetPending = true
    this.commander('ws.get').then(
      () => {},
      () => {
        this.wsGetPending = false
      },
    )
  }

  private onConfirm(event: JsonMap): void {
    if (typeof event.cid !== 'string' || typeof event.title !== 'string') return
    const cid = event.cid
    const ask: ConfirmAsk = {
      cid,
      title: event.title,
      detail: typeof event.detail === 'string' ? event.detail : undefined,
      ok: typeof event.ok === 'string' ? event.ok : 'OK',
      cancel: typeof event.cancel === 'string' ? event.cancel : 'Cancel',
      receivedAt: this.now(),
    }
    this.set({ confirms: [...this.current.confirms.filter((c) => c.cid !== cid), ask] })
    clearTimeout(this.confirmTimers.get(cid))
    this.confirmTimers.set(cid, setTimeout(() => this.dropConfirm(cid), this.confirmTtlMs))
  }

  private dropConfirm(cid: string): void {
    clearTimeout(this.confirmTimers.get(cid))
    this.confirmTimers.delete(cid)
    const remaining = this.current.confirms.filter((c) => c.cid !== cid)
    if (remaining.length !== this.current.confirms.length) this.set({ confirms: remaining })
  }

  async answerConfirm(cid: string, ok: boolean): Promise<void> {
    this.dropConfirm(cid)
    await this.commander('confirm.answer', { cid, ok })
  }

  private scheduleSave(): void {
    if (!this.cache || this.saveTimer) return
    this.saveTimer = setTimeout(() => void this.flush(), this.saveDelayMs)
  }

  async flush(): Promise<void> {
    if (this.saveTimer) clearTimeout(this.saveTimer)
    this.saveTimer = null
    const view = this.current
    if (!this.cache || (!view.snapshot && !view.workspace)) return
    await this.cache.save(this.machineId, {
      savedAt: view.updatedAt ?? this.now(),
      seq: view.seq,
      wsRev: view.wsRev,
      snapshot: view.snapshot,
      workspace: view.workspace,
      hello: view.hello,
    })
  }

  dispose(): void {
    if (this.saveTimer) clearTimeout(this.saveTimer)
    this.saveTimer = null
    for (const timer of this.confirmTimers.values()) clearTimeout(timer)
    this.confirmTimers.clear()
  }
}
