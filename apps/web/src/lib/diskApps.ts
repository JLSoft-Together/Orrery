import {
  RAM_KILL_MAX,
  RAM_POLL_MS,
  type AppLeftover,
  type AppLeftoversResult,
  type AppUninstallResult,
  type DiskDeleteResult,
  type InstalledApp,
  type InstalledAppsResult,
  type RamKillResult,
  type RamKillTarget,
  type RamLock,
  type RamProc,
  type RamSnapshot,
  type RamTag,
} from '@ccm/shared'
import { desktop } from './desktop'

export type { AppLeftover, AppLeftoversResult, AppUninstallResult, InstalledApp, InstalledAppsResult, RamKillResult, RamKillTarget, RamLock, RamProc, RamSnapshot, RamTag }
export { RAM_KILL_MAX, RAM_POLL_MS }

interface DiskAppsBridge {
  diskAppsList?: () => Promise<unknown>
  diskAppsUninstall?: (id: string) => Promise<unknown>
  diskAppsLeftovers?: (id: string) => Promise<unknown>
  diskAppsDelete?: (req: unknown) => Promise<unknown>
  diskAppsSettings?: () => Promise<unknown>
  diskRamSnapshot?: () => Promise<unknown>
  diskRamKill?: (targets: unknown) => Promise<unknown>
}
const bridge = desktop as unknown as DiskAppsBridge | undefined

export const hasDiskApps = typeof bridge?.diskAppsList === 'function' && typeof bridge?.diskAppsUninstall === 'function'
export const hasDiskRam = typeof bridge?.diskRamSnapshot === 'function' && typeof bridge?.diskRamKill === 'function'

const call = async <T>(fn: (() => Promise<unknown>) | undefined, fallback: T): Promise<T> => {
  if (!fn) return fallback
  try {
    const v = await fn()
    return (v ?? fallback) as T
  } catch {
    return fallback
  }
}

const EMPTY_LIST: InstalledAppsResult = { apps: [], removed: [], ms: 0, ok: false, storeApps: 'none' }
const NO_DELETE: DiskDeleteResult = { done: [], failed: [], freed: 0, permanent: false, cancelled: true }

export const diskAppsList = (): Promise<InstalledAppsResult> => call(bridge?.diskAppsList && (() => bridge.diskAppsList!()), EMPTY_LIST)
export const diskAppsUninstall = (id: string): Promise<AppUninstallResult> => call<AppUninstallResult>(bridge?.diskAppsUninstall && (() => bridge.diskAppsUninstall!(id)), { ok: false, error: 'failed' })
export const diskAppsLeftovers = (id: string): Promise<AppLeftoversResult | null> => call<AppLeftoversResult | null>(bridge?.diskAppsLeftovers && (() => bridge.diskAppsLeftovers!(id)), null)
export const diskAppsDelete = (id: string, paths: string[], permanent: boolean): Promise<DiskDeleteResult> =>
  call(bridge?.diskAppsDelete && (() => bridge.diskAppsDelete!({ id, paths: [...paths], permanent })), NO_DELETE)
export const diskAppsSettings = (): Promise<boolean> => call(bridge?.diskAppsSettings && (() => bridge.diskAppsSettings!()), false)
export const diskRamSnapshot = (): Promise<RamSnapshot | null> => call<RamSnapshot | null>(bridge?.diskRamSnapshot && (() => bridge.diskRamSnapshot!()), null)
export const diskRamKill = (targets: RamKillTarget[]): Promise<RamKillResult> =>
  call<RamKillResult>(bridge?.diskRamKill && (() => bridge.diskRamKill!(targets.map((t) => ({ pid: t.pid, start: t.start })))), { ok: false, killed: 0, error: 'failed' })

export type AppSort = 'size' | 'name' | 'date'
export const APP_SORTS: AppSort[] = ['size', 'name', 'date']

export function filterApps(apps: readonly InstalledApp[], query: string, sort: AppSort): InstalledApp[] {
  const q = query.trim().toLocaleLowerCase()
  const list = q ? apps.filter((a) => `${a.name} ${a.publisher ?? ''}`.toLocaleLowerCase().includes(q)) : [...apps]
  if (sort === 'name') return list.sort((a, b) => a.name.localeCompare(b.name))
  if (sort === 'date') return list.sort((a, b) => (b.installedAt ?? 0) - (a.installedAt ?? 0) || a.name.localeCompare(b.name))
  return list.sort((a, b) => (b.size ?? -1) - (a.size ?? -1) || a.name.localeCompare(b.name))
}

export interface RamGroup {
  key: string
  name: string
  ramBytes: number
  cpu: number | null
  count: number
  tag: RamTag | null
  lock: RamLock | null
  targets: RamKillTarget[]
}

export const procBase = (name: string): string => name.trim().replace(/\.exe$/i, '')

export function groupRam(procs: readonly RamProc[]): RamGroup[] {
  const map = new Map<string, RamGroup & { killable: RamProc[] }>()
  for (const p of procs) {
    const name = procBase(p.name) || String(p.pid)
    const key = name.toLowerCase()
    let g = map.get(key)
    if (!g) {
      g = { key, name, ramBytes: 0, cpu: null, count: 0, tag: null, lock: null, targets: [], killable: [] }
      map.set(key, g)
    }
    g.count++
    g.ramBytes += p.ramBytes ?? 0
    if (p.cpu !== null) g.cpu = (g.cpu ?? 0) + p.cpu
    if (p.tag === 'self' || (!g.tag && p.tag)) g.tag = p.tag
    if (p.lock) g.lock = g.lock === 'self' || p.lock === 'self' ? 'self' : g.lock === 'parent' || p.lock === 'parent' ? 'parent' : (g.lock ?? p.lock)
    else if (p.start) g.killable.push(p)
  }
  return [...map.values()]
    .map(({ killable, ...g }) => ({
      ...g,
      targets: (g.lock === 'self' || g.lock === 'parent' ? [] : killable)
        .sort((a, b) => (b.ramBytes ?? 0) - (a.ramBytes ?? 0))
        .slice(0, RAM_KILL_MAX)
        .map((p) => ({ pid: p.pid, start: p.start })),
    }))
    .sort((a, b) => b.ramBytes - a.ramBytes)
}

export function filterRam<T extends { name: string }>(rows: readonly T[], query: string): T[] {
  const q = query.trim().toLocaleLowerCase()
  return q ? rows.filter((r) => r.name.toLocaleLowerCase().includes(q)) : [...rows]
}

export function ramShare(bytes: number | null, total: number): number | null {
  if (bytes === null || !total) return null
  return Math.min(100, (bytes / total) * 100)
}

export function formatPct(v: number | null): string {
  if (v === null) return '-'
  return v >= 10 ? `${Math.round(v)}%` : `${v.toFixed(1)}%`
}
