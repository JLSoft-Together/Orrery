import type { CleanCategory, CleanItem, CleanLock, CleanProgress, CleanScanResult, DiskDeleteResult } from '@ccm/shared'
import { desktop } from './desktop'

export type { CleanCategory, CleanItem, CleanLock, CleanProgress, CleanScanResult }

interface CleanBridge {
  diskCleanCategories?: () => Promise<unknown>
  diskCleanScan?: (req: unknown) => Promise<unknown>
  diskCleanCancel?: () => Promise<unknown>
  diskCleanDelete?: (req: unknown) => Promise<unknown>
  onDiskCleanProgress?: (cb: (p: unknown) => void) => () => void
}
const bridge = desktop as unknown as CleanBridge | undefined

export const hasDiskClean = typeof bridge?.diskCleanScan === 'function' && typeof bridge?.diskCleanDelete === 'function'

const call = async <T>(fn: (() => Promise<unknown>) | undefined, fallback: T): Promise<T> => {
  if (!fn) return fallback
  try {
    const v = await fn()
    return (v ?? fallback) as T
  } catch {
    return fallback
  }
}

export const diskCleanCategories = (): Promise<CleanCategory[]> => call(bridge?.diskCleanCategories && (() => bridge.diskCleanCategories!()), [])
export const diskCleanScan = (ids: string[], roots: string[], open: string[]): Promise<CleanScanResult> =>
  call(bridge?.diskCleanScan && (() => bridge.diskCleanScan!({ ids: [...ids], roots: [...roots], open: [...open] })), { items: [] })
export const diskCleanCancel = (): Promise<boolean> => call(bridge?.diskCleanCancel && (() => bridge.diskCleanCancel!()), false)
export const diskCleanDelete = (paths: string[], permanent: boolean, open: string[]): Promise<DiskDeleteResult | null> =>
  call<DiskDeleteResult | null>(bridge?.diskCleanDelete && (() => bridge.diskCleanDelete!({ paths: [...paths], permanent, open: [...open] })), null)
export const onDiskCleanProgress = (cb: (p: CleanProgress) => void): (() => void) => bridge?.onDiskCleanProgress?.((p) => cb(p as CleanProgress)) ?? (() => {})

export const DAY_MS = 86_400_000
export const DAYS_KEY = 'ccm.diskCleanDays'
export const DAYS_DEFAULT = 30
export const DAYS_MIN = 1
export const DAYS_MAX = 3650
export const PROJECT_IDS = ['node_modules', 'build']
export const ON_DEMAND = ['cache']

export function clampDays(v: unknown): number {
  const n = typeof v === 'number' ? v : Number(v)
  if (!Number.isFinite(n)) return DAYS_DEFAULT
  return Math.min(DAYS_MAX, Math.max(DAYS_MIN, Math.round(n)))
}

export function loadDays(): number {
  try {
    const raw = localStorage.getItem(DAYS_KEY)
    return raw === null ? DAYS_DEFAULT : clampDays(raw)
  } catch {
    return DAYS_DEFAULT
  }
}

export function saveDays(days: number): void {
  try {
    localStorage.setItem(DAYS_KEY, String(clampDays(days)))
  } catch {
    return
  }
}

const norm = (p: string): string => p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

export function under(child: string, base: string): boolean {
  if (!child || !base) return false
  const c = norm(child)
  const b = norm(base)
  return c === b || c.startsWith(`${b}/`)
}

export function lockReason(item: Pick<CleanItem, 'path' | 'project' | 'locked'>, open: readonly string[]): CleanLock | undefined {
  if (item.locked) return item.locked
  const inside = (cwd: string): boolean => (!!item.project && under(cwd, item.project)) || under(cwd, item.path)
  return open.some(inside) ? 'open' : undefined
}

export function isStale(item: Pick<CleanItem, 'lastTouched'>, days: number, now: number): boolean {
  return typeof item.lastTouched === 'number' && now - item.lastTouched > days * DAY_MS
}

export function preselect(items: readonly CleanItem[], categories: readonly Pick<CleanCategory, 'id' | 'preselect'>[], days: number, now: number, open: readonly string[]): Set<string> {
  const on = new Set(categories.filter((c) => c.preselect).map((c) => c.id))
  return new Set(items.filter((i) => on.has(i.category) && !lockReason(i, open) && isStale(i, days, now)).map((i) => i.path))
}

export function inProject(item: Pick<CleanItem, 'path' | 'project'>, dir: string | null): boolean {
  if (!dir) return true
  return under(item.path, dir) || (!!item.project && (under(item.project, dir) || under(dir, item.project)))
}

export interface OpenSources {
  terminals: ReadonlyArray<{ cwd?: string | null; status?: string; endedAt?: string | number | null }>
  panes: ReadonlyArray<{ cwd?: string; running: boolean }>
  runs: ReadonlyArray<{ cwd: string; live: boolean }>
}

export function openRoots(src: OpenSources): string[] {
  const out: string[] = []
  const add = (p: string | null | undefined): void => {
    if (p && !out.some((o) => norm(o) === norm(p))) out.push(p)
  }
  for (const t of src.terminals) if (t.cwd && t.status !== 'stale' && !t.endedAt) add(t.cwd)
  for (const p of src.panes) if (p.running) add(p.cwd)
  for (const r of src.runs) if (r.live) add(r.cwd)
  return out.slice(0, 500)
}

export function baseName(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? p
}

export function relTo(p: string, base: string | undefined): string {
  if (!base || !under(p, base)) return p
  const rest = p.slice(base.replace(/[\\/]+$/, '').length).replace(/^[\\/]+/, '')
  return rest || baseName(p)
}

export function ageDays(lastTouched: number | undefined, now: number): number | null {
  return typeof lastTouched === 'number' ? Math.max(0, Math.floor((now - lastTouched) / DAY_MS)) : null
}

export const FOLDERS_KEY = 'ccm.diskCleanFolders'

export function loadFolders(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(FOLDERS_KEY) ?? '[]') as unknown
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string' && x.length > 0).slice(0, 50) : []
  } catch {
    return []
  }
}

export function saveFolders(list: string[]): void {
  try {
    localStorage.setItem(FOLDERS_KEY, JSON.stringify(list.slice(0, 50)))
  } catch {
    return
  }
}
