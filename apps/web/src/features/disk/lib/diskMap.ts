import { DISK_KINDS, DISK_SORTS } from '@ccm/shared'
export { DISK_KINDS, DISK_SORTS }
import type { DiskScanLevel, DiskVolume, DiskEntry, DiskChildren, DiskProgress, DiskStatus, DiskDeleteResult, DiskDone, DiskDupesDone, DiskDupesProgress, DiskDupesRequest, DiskDupesStart, DiskFastScanResult, DiskHistoryEntry, DiskSearchQuery, DiskSearchResult, DiskSizes, DiskSort } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'

export type { DiskSizes } from '@ccm/shared'
export type { DiskDupeFile, DiskDupeGroup, DiskDupesDone, DiskDupesProgress, DiskDupesRequest, DiskDupesStage, DiskDupesStart } from '@ccm/shared'
export type { DiskVolume, DiskEntry, DiskChildren, DiskProgress, DiskStatus, DiskDeleteResult, DiskDone, DiskFastScanResult, DiskHistoryEntry, DiskSearchQuery, DiskSearchResult } from '@ccm/shared'
export type { DiskSort }
export type DiskLevel = DiskScanLevel
export type DiskChart = 'treemap' | 'sunburst'

interface DiskMapBridge {
  diskMapVolumes?: () => Promise<DiskVolume[]>
  diskMapStatus?: () => Promise<DiskStatus>
  diskMapScan?: (req: { roots: string[]; level: DiskLevel; excludes: string[] }) => Promise<{ started: boolean }>
  diskMapPause?: () => Promise<boolean>
  diskMapResume?: () => Promise<boolean>
  diskMapCancel?: () => Promise<boolean>
  diskMapChildren?: (req: { path: string; sort: 'size' | 'name' | 'mtime'; limit: number }) => Promise<DiskChildren | null>
  diskMapSearch?: (q: DiskSearchQuery) => Promise<DiskSearchResult | null>
  diskMapDelete?: (req: { paths: string[]; permanent: boolean }) => Promise<DiskDeleteResult>
  diskMapReveal?: (path: string) => Promise<void>
  diskMapHistory?: (req: { limit: number }) => Promise<DiskHistoryEntry[]>
  diskMapFastScan?: (req: { drive: string }) => Promise<DiskFastScanResult>
  diskMapSizes?: (req: { paths: string[] }) => Promise<DiskSizes | null>
  diskMapDupes?: (req: DiskDupesRequest) => Promise<DiskDupesStart>
  diskMapDupesCancel?: () => Promise<boolean>
  onDiskMapDupesProgress?: (cb: (p: DiskDupesProgress) => void) => () => void
  onDiskMapDupesDone?: (cb: (d: DiskDupesDone | null) => void) => () => void
  onDiskMapProgress?: (cb: (p: DiskProgress) => void) => () => void
  onDiskMapDone?: (cb: (d: DiskDone | null) => void) => () => void
}

const bridge = desktop as unknown as DiskMapBridge | undefined

export const hasDiskMap = typeof bridge?.diskMapScan === 'function' && typeof bridge?.diskMapChildren === 'function'
export const hasDiskSizes = hasDiskMap && typeof bridge?.diskMapSizes === 'function'
export const hasDiskDupes = hasDiskMap && typeof bridge?.diskMapDupes === 'function' && typeof bridge?.onDiskMapDupesDone === 'function'
export const hasFastScan = hasDiskMap && typeof bridge?.diskMapFastScan === 'function'

async function call<T>(fn: (() => Promise<T>) | undefined, fallback: T): Promise<T> {
  if (!fn) return fallback
  try {
    return await fn()
  } catch {
    return fallback
  }
}

export const diskMapVolumes = (): Promise<DiskVolume[]> => call(bridge?.diskMapVolumes && (() => bridge.diskMapVolumes!()), [])
export const diskMapStatus = (): Promise<DiskStatus | null> => call<DiskStatus | null>(bridge?.diskMapStatus && (() => bridge.diskMapStatus!()), null)
export const diskMapScan = (roots: string[], level: DiskLevel, excludes: string[] = []): Promise<boolean> =>
  call(bridge?.diskMapScan && (async () => (await bridge.diskMapScan!({ roots, level, excludes })).started), false)
export const diskMapPause = (): Promise<boolean> => call(bridge?.diskMapPause && (() => bridge.diskMapPause!()), false)
export const diskMapResume = (): Promise<boolean> => call(bridge?.diskMapResume && (() => bridge.diskMapResume!()), false)
export const diskMapCancel = (): Promise<boolean> => call(bridge?.diskMapCancel && (() => bridge.diskMapCancel!()), false)
export const diskMapChildren = (path: string, limit = 2000): Promise<DiskChildren | null> =>
  call<DiskChildren | null>(bridge?.diskMapChildren && (() => bridge.diskMapChildren!({ path, sort: 'size', limit })), null)
export const diskMapSizes = (paths: string[]): Promise<DiskSizes | null> =>
  call<DiskSizes | null>(bridge?.diskMapSizes && (() => bridge.diskMapSizes!({ paths })), null)
export const diskMapSearch = (q: DiskSearchQuery): Promise<DiskSearchResult | null> =>
  call<DiskSearchResult | null>(bridge?.diskMapSearch && (() => bridge.diskMapSearch!(q)), null)
export const diskMapDelete = (paths: string[], permanent: boolean): Promise<DiskDeleteResult | null> =>
  call<DiskDeleteResult | null>(bridge?.diskMapDelete && (() => bridge.diskMapDelete!({ paths, permanent })), null)
export const diskMapReveal = (path: string): Promise<void> => call<void>(bridge?.diskMapReveal && (() => bridge.diskMapReveal!(path)), undefined)
export const diskMapHistory = (limit = 50): Promise<DiskHistoryEntry[]> => call(bridge?.diskMapHistory && (() => bridge.diskMapHistory!({ limit })), [])
export const onDiskMapProgress = (cb: (p: DiskProgress) => void): (() => void) => bridge?.onDiskMapProgress?.(cb) ?? (() => {})
export const diskMapFastScan = (drive: string): Promise<DiskFastScanResult> =>
  call<DiskFastScanResult>(bridge?.diskMapFastScan && (async () => (await bridge.diskMapFastScan!({ drive })) ?? { started: false, error: 'failed' }), { started: false, error: 'unavailable' })
export const diskMapDupes = (req: DiskDupesRequest): Promise<DiskDupesStart> =>
  call<DiskDupesStart>(bridge?.diskMapDupes && (async () => (await bridge.diskMapDupes!(req)) ?? { started: false, error: 'failed' }), { started: false, error: 'unavailable' })
export const diskMapDupesCancel = (): Promise<boolean> => call(bridge?.diskMapDupesCancel && (() => bridge.diskMapDupesCancel!()), false)
export const onDiskMapDupesProgress = (cb: (p: DiskDupesProgress) => void): (() => void) => bridge?.onDiskMapDupesProgress?.(cb) ?? (() => {})
export const onDiskMapDupesDone = (cb: (d: DiskDupesDone | null) => void): (() => void) => bridge?.onDiskMapDupesDone?.(cb) ?? (() => {})
export const onDiskMapDone = (cb: (d: DiskDone | null) => void): (() => void) => bridge?.onDiskMapDone?.(cb) ?? (() => {})

export const DISK_LEVELS: DiskLevel[] = ['light', 'balanced', 'max']
export const DISK_CHARTS: DiskChart[] = ['treemap', 'sunburst']
const LEVEL_KEY = 'ccm.diskLevel'
const CHART_KEY = 'ccm.diskChart'

export function loadLevel(): DiskLevel {
  try {
    const v = localStorage.getItem(LEVEL_KEY)
    if (v === 'light' || v === 'balanced' || v === 'max') return v
  } catch {
    return 'balanced'
  }
  return 'balanced'
}

export function saveLevel(level: DiskLevel): void {
  try {
    localStorage.setItem(LEVEL_KEY, level)
  } catch {
    return
  }
}

export function loadChart(): DiskChart {
  try {
    const v = localStorage.getItem(CHART_KEY)
    if (v === 'treemap' || v === 'sunburst') return v
  } catch {
    return 'treemap'
  }
  return 'treemap'
}

const ROOTS_KEY = 'ccm.diskRoots'

export function loadRoots(): string[] | null {
  try {
    const raw = localStorage.getItem(ROOTS_KEY)
    if (raw === null) return null
    const v: unknown = JSON.parse(raw)
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : null
  } catch {
    return null
  }
}

export function saveRoots(roots: string[]): void {
  try {
    localStorage.setItem(ROOTS_KEY, JSON.stringify(roots))
  } catch {
    return
  }
}

export function onlyMounts(roots: string[], mounts: string[]): string[] {
  const key = (p: string): string => p.replace(/[\\/]+$/, '').toLowerCase()
  const known = new Map(mounts.map((m) => [key(m), m]))
  const out: string[] = []
  for (const r of roots) {
    const m = known.get(key(r))
    if (m && !out.includes(m)) out.push(m)
  }
  return out
}

export function saveChart(chart: DiskChart): void {
  try {
    localStorage.setItem(CHART_KEY, chart)
  } catch {
    return
  }
}

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']

export function formatSize(bytes: number | null | undefined, locale = 'en'): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return '-'
  if (bytes < 1024) return `${Math.round(bytes)} B`
  let v = bytes
  let u = 0
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024
    u++
  }
  const digits = v >= 100 ? 0 : v >= 10 ? 1 : 2
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(v)} ${UNITS[u]}`
}

export function formatDuration(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const r = s % 60
  const mm = String(m).padStart(2, '0')
  const rr = String(r).padStart(2, '0')
  return h ? `${h}:${mm}:${rr}` : `${m}:${rr}`
}

export function percent(part: number, whole: number): number {
  if (!(whole > 0) || !(part > 0)) return 0
  return Math.min(100, (part / whole) * 100)
}

export function formatPercent(part: number, whole: number): string {
  const p = percent(part, whole)
  if (p === 0) return '0%'
  if (p < 0.1) return '<0.1%'
  return `${p < 10 ? p.toFixed(1) : Math.round(p)}%`
}

export type DiskGroup = 'dir' | 'media' | 'code' | 'archive' | 'doc' | 'app' | 'other'
export const DISK_GROUPS: DiskGroup[] = ['dir', 'media', 'code', 'archive', 'doc', 'app', 'other']

export const GROUP_VAR: Record<DiskGroup | 'rest', string> = {
  dir: '--ccm-cat-1',
  media: '--ccm-cat-5',
  code: '--ccm-cat-3',
  archive: '--ccm-cat-2',
  doc: '--ccm-cat-4',
  app: '--ccm-cat-7',
  other: '--ccm-ink-faint',
  rest: '--ccm-line-strong',
}

const EXT_GROUP: Record<string, DiskGroup> = {}
const addExt = (g: DiskGroup, exts: string): void => {
  for (const e of exts.split(' ')) EXT_GROUP[e] = g
}
addExt('media', 'png jpg jpeg gif webp bmp svg ico heic mp4 mkv mov avi webm mp3 wav flac aac ogg m4a psd raw')
addExt('code', 'ts tsx js jsx mjs cjs vue py rs go java kt kts swift c cc cpp h hpp cs rb php dart json yml yaml toml xml html css scss sh bat ps1 gradle lock map')
addExt('archive', 'zip rar 7z tar gz tgz bz2 xz iso img dmg vhd vhdx vmdk cab apk aab ipa')
addExt('doc', 'pdf doc docx xls xlsx ppt pptx txt md rtf csv epub log')
addExt('app', 'exe dll msi sys so dylib bin node pak dat db sqlite jar class o obj pdb')

const KIND_GROUP: Record<string, DiskGroup> = {
  media: 'media',
  image: 'media',
  video: 'media',
  audio: 'media',
  code: 'code',
  source: 'code',
  archive: 'archive',
  doc: 'doc',
  document: 'doc',
  app: 'app',
  binary: 'app',
  system: 'app',
}

export function groupOf(e: Pick<DiskEntry, 'dir' | 'name' | 'kind'>): DiskGroup {
  if (e.dir) return 'dir'
  const byKind = KIND_GROUP[(e.kind ?? '').toLowerCase()]
  if (byKind) return byKind
  const dot = e.name.lastIndexOf('.')
  if (dot > 0) return EXT_GROUP[e.name.slice(dot + 1).toLowerCase()] ?? 'other'
  return 'other'
}

export interface Slice {
  entry: DiskEntry | null
  size: number
  more: number
}

export function topSlices(items: DiskEntry[], max: number, total: number): Slice[] {
  const sorted = items.filter((i) => i.size > 0).sort((a, b) => b.size - a.size)
  const out: Slice[] = sorted.slice(0, max).map((entry) => ({ entry, size: entry.size, more: 0 }))
  let listed = 0
  for (const s of sorted) listed += s.size
  let tail = 0
  for (let i = max; i < sorted.length; i++) tail += sorted[i]?.size ?? 0
  const unlisted = Math.max(0, total - listed)
  const rest = tail + unlisted
  if (rest > 0) out.push({ entry: null, size: rest, more: Math.max(0, sorted.length - max) })
  return out
}

export function parentPath(path: string): string | null {
  const trimmed = path.replace(/[\\/]+$/, '')
  const i = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'))
  if (i < 0) return null
  if (i === 0) return '/'
  const head = trimmed.slice(0, i)
  if (/^[A-Za-z]:$/.test(head)) return `${head}\\`
  return head
}

export interface Crumb {
  label: string
  path: string
}

export function breadcrumbs(path: string): Crumb[] {
  if (!path) return []
  const win = /^[A-Za-z]:/.test(path) || path.includes('\\')
  const parts = path.split(/[\\/]/).filter(Boolean)
  const out: Crumb[] = []
  let acc = ''
  parts.forEach((p, i) => {
    if (win) acc = i === 0 ? `${p}\\` : `${acc.replace(/\\$/, '')}\\${p}`
    else acc = `${acc}/${p}`
    out.push({ label: p, path: acc })
  })
  if (!win) out.unshift({ label: '/', path: '/' })
  return out
}

export function isUnder(path: string, root: string): boolean {
  const a = path.replace(/[\\/]+$/, '').toLowerCase()
  const b = root.replace(/[\\/]+$/, '').toLowerCase()
  return a === b || a.startsWith(`${b}\\`) || a.startsWith(`${b}/`)
}

export function volumeUsedPct(v: Pick<DiskVolume, 'total' | 'free'>): number {
  if (!(v.total > 0)) return 0
  return Math.min(100, Math.max(0, ((v.total - v.free) / v.total) * 100))
}

export type DiskMode = 'browse' | 'search' | 'big' | 'old'
export type DiskKindFilter = '' | (typeof DISK_KINDS)[number]

export interface DiskFilters {
  text: string
  kind: DiskKindFilter
  minSize: number | null
  maxSize: number | null
  after: number | null
  before: number | null
  only: '' | 'file' | 'dir'
  scoped: boolean
  sort: DiskSort
}

export const DISK_MODES: DiskMode[] = ['browse', 'search', 'big', 'old']
export const SIZE_PRESETS = [100 * 1024 ** 2, 1024 ** 3, 10 * 1024 ** 3]
export const OLD_MONTHS = [3, 6, 12, 24]
export const BIG_MIN_DEFAULT = SIZE_PRESETS[0]!
export const SEARCH_LIMIT = 500
export const SEARCH_DEBOUNCE_MS = 120
const DAY = 86_400_000

export function defaultFilters(): DiskFilters {
  return { text: '', kind: '', minSize: null, maxSize: null, after: null, before: null, only: '', scoped: false, sort: 'size' }
}

export function monthsAgo(now: number, months: number): number {
  const d = new Date(now)
  d.setMonth(d.getMonth() - months)
  return d.getTime()
}

export function hasCriteria(f: DiskFilters): boolean {
  return !!(f.text.trim() || f.kind || f.minSize || f.maxSize || f.after || f.before || f.only)
}

export function buildSearchQuery(mode: DiskMode, f: DiskFilters, ctx: { now: number; path: string; oldMonths: number }): DiskSearchQuery | null {
  if (mode === 'browse') return null
  if (mode === 'search' && !hasCriteria(f)) return null
  const q: DiskSearchQuery = { limit: SEARCH_LIMIT }
  const text = f.text.trim()
  if (text) q.query = text
  if (f.kind) q.kind = f.kind
  if (f.maxSize && f.maxSize > 0) q.maxSize = f.maxSize
  if (f.scoped && ctx.path) q.under = ctx.path
  if (mode === 'big') {
    q.only = 'file'
    q.sort = 'size'
    q.minSize = f.minSize && f.minSize > 0 ? f.minSize : BIG_MIN_DEFAULT
    return q
  }
  if (mode === 'old') {
    q.only = 'file'
    q.sort = 'size'
    q.before = monthsAgo(ctx.now, ctx.oldMonths)
    if (f.minSize && f.minSize > 0) q.minSize = f.minSize
    return q
  }
  if (f.minSize && f.minSize > 0) q.minSize = f.minSize
  if (f.after) q.after = f.after
  if (f.before) q.before = f.before
  if (f.only) q.only = f.only
  q.sort = f.sort
  return q
}

export function dateToMs(value: string, endOfDay = false): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
  if (!m) return null
  const ms = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).getTime()
  if (!Number.isFinite(ms)) return null
  return endOfDay ? ms + DAY : ms
}

export function msToDate(ms: number | null, endOfDay = false): string {
  if (!ms) return ''
  const d = new Date(endOfDay ? ms - DAY : ms)
  const mm = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  return `${d.getFullYear()}-${mm}-${dd}`
}

const isVi = (locale: string): boolean => locale.toLowerCase().startsWith('vi')

export function datePattern(locale: string): string {
  return isVi(locale) ? 'dd/mm/yyyy' : 'yyyy-mm-dd'
}

export function formatDateInput(ms: number | null, locale: string, endOfDay = false): string {
  const iso = msToDate(ms, endOfDay)
  if (!iso || !isVi(locale)) return iso
  const [y, m, d] = iso.split('-')
  return `${d}/${m}/${y}`
}

export function parseDateInput(value: string, locale: string, endOfDay = false): number | null {
  const raw = value.trim()
  const slash = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})$/.exec(raw)
  const iso = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(raw)
  let y: number
  let m: number
  let d: number
  if (iso) [y, m, d] = [Number(iso[1]), Number(iso[2]), Number(iso[3])]
  else if (slash) [y, m, d] = [Number(slash[3]), isVi(locale) ? Number(slash[2]) : Number(slash[1]), isVi(locale) ? Number(slash[1]) : Number(slash[2])]
  else return null
  const at = new Date(y, m - 1, d)
  if (at.getFullYear() !== y || at.getMonth() !== m - 1 || at.getDate() !== d) return null
  const ms = at.getTime()
  return endOfDay ? ms + DAY : ms
}

export function mbToBytes(mb: number): number | null {
  return Number.isFinite(mb) && mb > 0 ? Math.round(mb * 1024 ** 2) : null
}

export function shortPath(p: string, max = 56): string {
  if (p.length <= max) return p
  const keep = max - 3
  const head = Math.ceil(keep * 0.4)
  return `${p.slice(0, head)}...${p.slice(p.length - (keep - head))}`
}

export function fastScanDrive(v: Pick<DiskVolume, 'mount' | 'fs' | 'network' | 'kind'>): string | null {
  if (v.network || v.fs !== 'NTFS' || (v.kind !== 'fixed' && v.kind !== 'removable')) return null
  const m = /^([A-Za-z]):[\\/]?$/.exec(v.mount)
  return m ? m[1]!.toUpperCase() : null
}

export type FastScanError = 'declined' | 'unsupported' | 'busy' | 'failed'

export function fastScanError(error: string | undefined): FastScanError {
  if (error === 'elevation-declined') return 'declined'
  if (error === 'unsupported' || error === 'unavailable') return 'unsupported'
  if (error === 'busy') return 'busy'
  return 'failed'
}
