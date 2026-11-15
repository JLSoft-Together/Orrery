import type { DiskScanLevel, DiskVolume, DiskEntry, DiskChildren, DiskProgress, DiskStatus, DiskDeleteResult, DiskHistoryEntry } from '@ccm/shared'
import { desktop } from './desktop'

export type { DiskVolume, DiskEntry, DiskChildren, DiskProgress, DiskStatus, DiskDeleteResult, DiskHistoryEntry } from '@ccm/shared'
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
  diskMapDelete?: (req: { paths: string[]; permanent: boolean }) => Promise<DiskDeleteResult>
  diskMapReveal?: (path: string) => Promise<void>
  diskMapHistory?: (req: { limit: number }) => Promise<DiskHistoryEntry[]>
  onDiskMapProgress?: (cb: (p: DiskProgress) => void) => () => void
  onDiskMapDone?: (cb: () => void) => () => void
}

const bridge = desktop as unknown as DiskMapBridge | undefined

export const hasDiskMap = typeof bridge?.diskMapScan === 'function' && typeof bridge?.diskMapChildren === 'function'

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
export const diskMapDelete = (paths: string[], permanent: boolean): Promise<DiskDeleteResult | null> =>
  call<DiskDeleteResult | null>(bridge?.diskMapDelete && (() => bridge.diskMapDelete!({ paths, permanent })), null)
export const diskMapReveal = (path: string): Promise<void> => call<void>(bridge?.diskMapReveal && (() => bridge.diskMapReveal!(path)), undefined)
export const diskMapHistory = (limit = 50): Promise<DiskHistoryEntry[]> => call(bridge?.diskMapHistory && (() => bridge.diskMapHistory!({ limit })), [])
export const onDiskMapProgress = (cb: (p: DiskProgress) => void): (() => void) => bridge?.onDiskMapProgress?.(cb) ?? (() => {})
export const onDiskMapDone = (cb: () => void): (() => void) => bridge?.onDiskMapDone?.(cb) ?? (() => {})

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
