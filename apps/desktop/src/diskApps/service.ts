import fs from 'node:fs/promises'
import path from 'node:path'
import type { MessageBoxOptions } from 'electron'
import type { AppLeftover, AppLeftoversResult, AppUninstallResult, DiskDeleteResult, DiskHistoryEntry, InstalledApp, InstalledAppsResult } from '@ccm/shared'
import { dirSize } from '../diskClean/size'
import { measureRest, mapLimit, runDelete, type DeleteAsk } from '../diskMap/delete'
import { findLeftovers, type LeftoverRoot } from './leftovers'
import { readMacApps } from './mac'
import { readFixtureText, readRegistryText, readWindowsApps } from './registry'
import { diskAppsText } from './strings'
import { parseUninstall, type Launch } from './uninstall'

export interface DiskAppsDeps {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  regFixture: string | null
  macRoots: string[]
  selfBundle: string | null
  leftoverRoots: LeftoverRoot[]
  indexSizes: (paths: string[]) => Promise<Record<string, number | null> | null>
  exists: (file: string) => boolean
  confirm: (opts: MessageBoxOptions) => Promise<boolean>
  confirmDelete: (ask: DeleteAsk, second: boolean) => Promise<boolean>
  autoConfirm: boolean
  launch: Launch
  trash: (p: string) => Promise<void>
  remove: (p: string) => Promise<void>
  locale: () => string
  keep: () => string[]
  keepTree: () => string[]
  record: (e: DiskHistoryEntry) => Promise<void>
  readApps?: () => Promise<InstalledApp[] | null>
}

const REMOVED_MAX = 20
const MAX_LEFTOVER_DELETE = 200

export class DiskAppsService {
  private apps = new Map<string, InstalledApp>()
  private seen = new Map<string, InstalledApp>()
  private leftovers = new Map<string, AppLeftover[]>()
  private busy = false
  private listing: Promise<InstalledAppsResult> | null = null

  constructor(private readonly deps: DiskAppsDeps) {}

  get supported(): boolean {
    return this.deps.platform === 'win32' || this.deps.platform === 'darwin'
  }

  private async readRaw(): Promise<InstalledApp[] | null> {
    if (this.deps.readApps) return this.deps.readApps()
    if (this.deps.platform === 'win32') {
      const fixture = this.deps.regFixture
      return readWindowsApps(fixture ? () => readFixtureText(fixture) : () => readRegistryText(this.deps.env), this.deps.env)
    }
    if (this.deps.platform === 'darwin') return readMacApps({ roots: this.deps.macRoots, selfBundle: this.deps.selfBundle, indexSizes: this.deps.indexSizes })
    return null
  }

  private async withIndexSizes(apps: InstalledApp[]): Promise<InstalledApp[]> {
    if (this.deps.platform !== 'win32') return apps
    const locs = [...new Set(apps.map((a) => a.location).filter((l): l is string => !!l))]
    if (!locs.length) return apps
    const sizes = await this.deps.indexSizes(locs.slice(0, 2000)).catch(() => null)
    if (!sizes) return apps
    return apps.map((a) => {
      const s = a.location ? sizes[a.location] : null
      return typeof s === 'number' && s > 0 ? { ...a, size: s, sizeFrom: 'index' as const } : a
    })
  }

  list(): Promise<InstalledAppsResult> {
    this.listing ??= this.doList().finally(() => {
      this.listing = null
    })
    return this.listing
  }

  private async doList(): Promise<InstalledAppsResult> {
    const t0 = Date.now()
    const raw = this.supported ? await this.readRaw().catch(() => null) : null
    if (!raw) return { apps: [...this.apps.values()], ms: Date.now() - t0, ok: false, storeApps: 'none', removed: this.removed() }
    const apps = await this.withIndexSizes(raw)
    this.apps = new Map(apps.map((a) => [a.id, a]))
    for (const a of apps) {
      this.seen.delete(a.id)
      this.seen.set(a.id, a)
    }
    return { apps, ms: Date.now() - t0, ok: true, storeApps: this.deps.platform === 'win32' ? 'settings' : 'none', removed: this.removed() }
  }

  private removed(): InstalledApp[] {
    const out = [...this.seen.values()].filter((a) => !this.apps.has(a.id))
    for (const a of out.slice(0, Math.max(0, out.length - REMOVED_MAX))) this.seen.delete(a.id)
    return out.slice(-REMOVED_MAX).reverse()
  }

  async uninstall(id: unknown): Promise<AppUninstallResult> {
    if (typeof id !== 'string') return { ok: false, error: 'unknown' }
    const app = this.apps.get(id)
    if (!app) return { ok: false, error: 'unknown' }
    if (app.blocked) return { ok: false, error: 'blocked' }
    if (this.busy) return { ok: false, error: 'busy' }
    this.busy = true
    try {
      return this.deps.platform === 'darwin' ? await this.trashApp(app) : this.deps.platform === 'win32' ? await this.runUninstaller(app) : { ok: false, error: 'unsupported' }
    } finally {
      this.busy = false
    }
  }

  private async runUninstaller(app: InstalledApp): Promise<AppUninstallResult> {
    const fresh = (await this.readRaw().catch(() => null))?.find((a) => a.id === app.id)
    if (!fresh || fresh.uninstall !== app.uninstall) return { ok: false, error: 'changed' }
    const plan = parseUninstall(fresh.uninstall, { exists: this.deps.exists, env: this.deps.env })
    if ('error' in plan) return { ok: false, error: plan.error === 'missing' ? 'missing' : 'unparsable' }
    const text = diskAppsText(this.deps.locale())
    const opts: MessageBoxOptions = { type: 'warning', message: text.uninstallTitle(app.name), detail: text.uninstallDetail(plan.display), buttons: [text.uninstallConfirm, text.cancel], defaultId: 1, cancelId: 1, noLink: true }
    if (!this.deps.autoConfirm && !(await this.deps.confirm(opts))) return { ok: false, error: 'cancelled' }
    const again = parseUninstall(fresh.uninstall, { exists: this.deps.exists, env: this.deps.env })
    if ('error' in again || again.file !== plan.file || again.args !== plan.args) return { ok: false, error: 'changed' }
    return (await this.deps.launch(plan)) ? { ok: true, mode: 'started' } : { ok: false, error: 'failed' }
  }

  private async checkBundle(p: string): Promise<boolean> {
    if (!p.endsWith('.app') || !this.deps.macRoots.some((r) => path.posix.dirname(p) === path.posix.normalize(r))) return false
    const st = await fs.lstat(p).catch(() => null)
    return !!st && st.isDirectory() && !st.isSymbolicLink()
  }

  private async trashApp(app: InstalledApp): Promise<AppUninstallResult> {
    if (!(await this.checkBundle(app.id))) return { ok: false, error: 'blocked' }
    const text = diskAppsText(this.deps.locale())
    const opts: MessageBoxOptions = { type: 'warning', message: text.trashTitle(app.name), detail: text.trashDetail(app.id), buttons: [text.trashConfirm, text.cancel], defaultId: 1, cancelId: 1, noLink: true }
    if (!this.deps.autoConfirm && !(await this.deps.confirm(opts))) return { ok: false, error: 'cancelled' }
    if (!(await this.checkBundle(app.id))) return { ok: false, error: 'changed' }
    try {
      await this.deps.trash(app.id)
      return { ok: true, mode: 'trashed' }
    } catch {
      return { ok: false, error: 'failed' }
    }
  }

  async scanLeftovers(id: unknown): Promise<AppLeftoversResult | null> {
    if (typeof id !== 'string') return null
    const app = this.apps.get(id) ?? this.seen.get(id)
    if (!app || app.blocked) return null
    const installed = this.apps.has(id)
    const others = [...this.apps.values()].filter((a) => a.id !== id).map((a) => ({ name: a.name, publisher: a.publisher, bundleId: a.bundleId, location: a.location }))
    const hits = await findLeftovers({ name: app.name, publisher: app.publisher, bundleId: app.bundleId, location: app.location, installed, others }, this.deps.leftoverRoots, this.deps.platform)
    const items = await mapLimit(hits, 4, async (h): Promise<AppLeftover> => {
      if (h.file) return { ...h, size: (await fs.lstat(h.path).catch(() => null))?.size ?? null }
      return { ...h, size: await dirSize(h.path, AbortSignal.timeout(30_000)).catch(() => null) }
    })
    items.sort((a, b) => (a.match === b.match ? (b.size ?? 0) - (a.size ?? 0) : a.match === 'exact' ? -1 : 1))
    this.leftovers.set(id, items)
    return { id, installed, items }
  }

  async deleteLeftovers(req: unknown): Promise<DiskDeleteResult> {
    const empty = (permanent: boolean, reason = 'invalid'): DiskDeleteResult => ({ done: [], failed: [{ path: '', reason }], freed: 0, permanent })
    const o = typeof req === 'object' && req !== null ? (req as Record<string, unknown>) : null
    const permanent = o?.permanent === true
    if (!o || typeof o.id !== 'string' || !Array.isArray(o.paths) || o.paths.length < 1 || o.paths.length > MAX_LEFTOVER_DELETE) return empty(permanent)
    const list = this.leftovers.get(o.id)
    if (!list) return empty(permanent, 'expired')
    const byKey = new Map(list.map((l) => [l.path.toLowerCase(), l]))
    const paths: string[] = []
    for (const p of o.paths) {
      const hit = typeof p === 'string' ? byKey.get(p.toLowerCase()) : undefined
      if (!hit) return empty(permanent, 'unknown')
      paths.push(hit.path)
    }
    if (this.busy) return empty(permanent, 'busy')
    this.busy = true
    try {
      const yes = async (): Promise<boolean> => true
      const result = await runDelete(paths, permanent, {
        guard: async () => ({ platform: this.deps.platform, home: this.deps.home, keep: this.deps.keep(), roots: paths, keepTree: this.deps.keepTree() }),
        sizeOf: async (p) => byKey.get(p.toLowerCase())?.size ?? null,
        confirm: this.deps.autoConfirm ? yes : (a) => this.deps.confirmDelete(a, false),
        confirmPermanent: this.deps.autoConfirm ? yes : (a) => this.deps.confirmDelete(a, true),
        trash: this.deps.trash,
        remove: this.deps.remove,
        forget: async () => undefined,
        record: this.deps.record,
        measure: measureRest,
      })
      const gone = new Set(result.done.map((d) => d.path.toLowerCase()))
      this.leftovers.set(o.id, list.filter((l) => !gone.has(l.path.toLowerCase())))
      return result
    } finally {
      this.busy = false
    }
  }
}
