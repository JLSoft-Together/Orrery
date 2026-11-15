export type DiskScanLevel = 'light' | 'balanced' | 'max'
export type DiskSort = 'size' | 'name' | 'mtime'

export const DISK_SCAN_LEVELS: readonly DiskScanLevel[] = ['light', 'balanced', 'max']
export const DISK_SORTS: readonly DiskSort[] = ['size', 'name', 'mtime']
export const DISK_KINDS = ['image', 'video', 'audio', 'doc', 'archive', 'installer', 'code'] as const

export interface DiskVolume {
  mount: string
  label: string
  fs: string
  kind: string
  network: boolean
  total: number
  free: number
}

export interface DiskEntry {
  path: string
  name: string
  size: number
  files?: number
  dir: boolean
  mtime: number
  cloud?: boolean
  kind?: string
}

export interface DiskChildren {
  path: string
  size: number
  total: number
  items: DiskEntry[]
}

export interface DiskProgress {
  files: number
  bytes: number
  current: string
  elapsedMs: number
}

export interface DiskStatus {
  scanning: boolean
  paused: boolean
  indexed: boolean
  roots: string[]
  scannedAt?: number
  entries?: number
}

export interface DiskScanRequest {
  roots: string[]
  level: DiskScanLevel
  excludes: string[]
}

export interface DiskFastScanRequest {
  drive: string
}

export interface DiskFastScanResult {
  started: boolean
  error?: string
}

export interface DiskChildrenRequest {
  path: string
  sort: DiskSort
  limit: number
}

export interface DiskSearchQuery {
  query?: string
  minSize?: number
  maxSize?: number
  after?: number
  before?: number
  kind?: string
  only?: 'file' | 'dir'
  under?: string
  sort?: DiskSort
  limit?: number
}

export interface DiskSearchResult {
  total: number
  ms: number
  items: DiskEntry[]
}

export interface DiskDeleteRequest {
  paths: string[]
  permanent: boolean
}

export interface DiskDeleteResult {
  done: { path: string; size: number }[]
  failed: { path: string; reason: string }[]
  freed: number
  permanent: boolean
  cancelled?: boolean
}

export interface DiskDone {
  ok: boolean
  cancelled?: boolean
  error?: string
  entries?: number
  bytes?: number
  roots?: string[]
  mft?: boolean
}

export interface DiskHistoryEntry {
  at: number
  action: 'trash' | 'delete' | 'move'
  count: number
  freed: number
  paths: string[]
  failed: number
}

export interface DiskSizesRequest {
  paths: string[]
}

export interface DiskSizes {
  version: string
  sizes: Record<string, number | null>
}

export type DiskDupesStage = 'idle' | 'size' | 'quick' | 'full'

export interface DiskDupesRequest {
  under?: string[]
  minSize?: number
  excludes?: string[]
  level?: DiskScanLevel
}

export interface DiskDupesProgress {
  stage: DiskDupesStage
  candidates: number
  files: number
  filesDone: number
  bytes: number
  bytesRead: number
  groups: number
  skipped: number
  current: string
  elapsedMs: number
  paused: boolean
}

export interface DiskDupeFile {
  path: string
  mtime: number
}

export interface DiskDupeGroup {
  size: number
  hash: string
  count: number
  files: DiskDupeFile[]
}

export interface DiskDupesSkipped {
  stale: number
  cloud: number
  locked: number
  changed: number
  gone: number
}

export interface DiskDupesDone {
  ok: boolean
  cancelled?: boolean
  error?: string
  groups: DiskDupeGroup[]
  totalGroups: number
  wasted: number
  candidates: number
  hashedFiles: number
  hashedBytes: number
  skipped: DiskDupesSkipped
  ms: number
}

export interface DiskDupesStart {
  started: boolean
  error?: string
}

export const DISK_SNAPSHOT_DEPTH = 4
export const DISK_SNAPSHOT_KEEP = 30
export const DISK_SNAPSHOT_TOP = 20
export const DISK_FREQS = ['off', 'daily', 'weekly'] as const
export type DiskFreq = (typeof DISK_FREQS)[number]
export const isDiskFreq = (v: unknown): v is DiskFreq => typeof v === 'string' && (DISK_FREQS as readonly string[]).includes(v)

export interface DiskSnapshotMeta {
  id: string
  at: number
  bytes: number
  roots: string[]
}

export interface DiskDiffRow {
  path: string
  name: string
  before: number
  after: number
  delta: number
  state: 'same' | 'new' | 'gone' | 'changed'
}

export interface DiskCompareRequest {
  id: string
  path?: string
}

export interface DiskCompare {
  base: DiskSnapshotMeta
  path: string
  covered: boolean
  rows: DiskDiffRow[]
  top: DiskDiffRow[]
}

export interface DiskScheduleLast {
  at: number
  ok: boolean
}

export interface DiskSchedule {
  freq: DiskFreq
  telegram: boolean
  last: DiskScheduleLast | null
  running: boolean
  paused: boolean
}

export interface DiskSchedulePatch {
  freq?: DiskFreq
  telegram?: boolean
}
