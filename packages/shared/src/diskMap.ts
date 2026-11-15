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
  action: 'trash' | 'delete'
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
