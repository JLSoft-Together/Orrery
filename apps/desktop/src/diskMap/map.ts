import path from 'node:path'
import { DISK_KINDS, DISK_SCAN_LEVELS, DISK_SORTS, type DiskChildren, type DiskFastScanRequest, type DiskChildrenRequest, type DiskDone, type DiskDupeGroup, type DiskDupesDone, type DiskDupesProgress, type DiskDupesRequest, type DiskDupesStage, type DiskEntry, type DiskProgress, type DiskScanRequest, type DiskSearchQuery, type DiskSearchResult, type DiskSizes, type DiskSort, type DiskStatus, type DiskVolume } from '@ccm/shared'

export const MAX_PATH_CHARS = 32_000
export const MAX_ROOTS = 32
export const MAX_EXCLUDES = 500
export const MAX_LIMIT = 10_000
export const MAX_DELETE = 5_000
export const MAX_SIZES = 2_000

type Obj = Record<string, unknown>

const obj = (v: unknown): Obj | null => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Obj) : null)
const num = (v: unknown, fallback = 0): number => (typeof v === 'number' && Number.isFinite(v) ? v : fallback)
const str = (v: unknown, fallback = ''): string => (typeof v === 'string' ? v : fallback)

export function isAbsolutePath(p: unknown, platform: NodeJS.Platform = process.platform): p is string {
  if (typeof p !== 'string' || !p || p.length > MAX_PATH_CHARS || p.includes('\0')) return false
  if (platform === 'win32') return /^[A-Za-z]:[\\/]/.test(p) || /^[\\/]{2}[^\\/?.][^\\/]*[\\/][^\\/]+/.test(p)
  return path.posix.isAbsolute(p)
}

export function toEntry(v: unknown): DiskEntry | null {
  const o = obj(v)
  if (!o || typeof o.path !== 'string') return null
  const dir = o.dir === true
  const entry: DiskEntry = { path: o.path, name: str(o.name, o.path), size: num(o.size), dir, mtime: num(o.mtime) }
  if (o.cloud === true) entry.cloud = true
  if (!dir && typeof o.kind === 'string' && o.kind !== 'other') entry.kind = o.kind
  return entry
}

const entries = (v: unknown): DiskEntry[] => (Array.isArray(v) ? v.map(toEntry).filter((e): e is DiskEntry => e !== null) : [])

export function toChildren(v: unknown): DiskChildren | null {
  const o = obj(v)
  if (!o) return null
  return { path: str(o.path), size: num(o.size), total: num(o.total), items: entries(o.items) }
}

export function toSearch(v: unknown): DiskSearchResult {
  const o = obj(v)
  return { total: num(o?.total), ms: num(o?.ms), items: entries(o?.items) }
}

export function toStatus(v: unknown): DiskStatus {
  const o = obj(v)
  const ix = obj(o?.index)
  const status: DiskStatus = { scanning: o?.scanning === true, paused: o?.paused === true, indexed: ix !== null, roots: Array.isArray(ix?.roots) ? ix.roots.filter((r): r is string => typeof r === 'string') : [] }
  if (ix) {
    if (typeof ix.scannedAt === 'number' && ix.scannedAt > 0) status.scannedAt = ix.scannedAt
    if (typeof ix.entries === 'number') status.entries = ix.entries
  }
  return status
}

export function toProgress(v: unknown): DiskProgress {
  const o = obj(v)
  return { files: num(o?.files), bytes: num(o?.bytes), current: str(o?.current), elapsedMs: num(o?.elapsedMs) }
}

export function toDone(v: unknown): DiskDone {
  const o = obj(v)
  const done: DiskDone = { ok: o?.ok === true }
  if (o?.cancelled === true) done.cancelled = true
  if (typeof o?.error === 'string') done.error = o.error.slice(0, 300)
  if (typeof o?.entries === 'number') done.entries = o.entries
  if (typeof o?.bytes === 'number') done.bytes = o.bytes
  if (Array.isArray(o?.roots)) done.roots = o.roots.filter((r): r is string => typeof r === 'string')
  if (o?.mft === true) done.mft = true
  return done
}

export function toVolumes(v: unknown): DiskVolume[] {
  if (!Array.isArray(v)) return []
  return v.flatMap((x) => {
    const o = obj(x)
    if (!o || typeof o.mount !== 'string') return []
    return [{ mount: o.mount, label: str(o.label, o.mount), fs: str(o.fs), kind: str(o.kind, 'unknown'), network: o.network === true, total: num(o.total), free: num(o.free) }]
  })
}

const isSort = (v: unknown): v is DiskSort => typeof v === 'string' && (DISK_SORTS as readonly string[]).includes(v)
const limitOf = (v: unknown, fallback: number): number => (typeof v === 'number' && Number.isInteger(v) && v >= 0 ? Math.min(v, MAX_LIMIT) : fallback)

export function cleanScan(v: unknown, platform: NodeJS.Platform = process.platform): DiskScanRequest | null {
  const o = obj(v)
  if (!o || !Array.isArray(o.roots) || o.roots.length < 1 || o.roots.length > MAX_ROOTS) return null
  if (!o.roots.every((r) => isAbsolutePath(r, platform))) return null
  const level = typeof o.level === 'string' && (DISK_SCAN_LEVELS as readonly string[]).includes(o.level) ? (o.level as DiskScanRequest['level']) : 'balanced'
  const ex = o.excludes === undefined ? [] : o.excludes
  if (!Array.isArray(ex) || ex.length > MAX_EXCLUDES || !ex.every((e) => typeof e === 'string' && e.length > 0 && e.length <= 1024 && !e.includes('\0'))) return null
  return { roots: [...new Set(o.roots as string[])], level, excludes: ex as string[] }
}

export function cleanFastScan(v: unknown, platform: NodeJS.Platform = process.platform): DiskFastScanRequest | null {
  if (platform !== 'win32') return null
  const o = obj(v)
  const m = typeof o?.drive === 'string' ? /^([A-Za-z])(?::[\\/]?)?$/.exec(o.drive) : null
  return m ? { drive: m[1]!.toUpperCase() } : null
}

export function cleanChildren(v: unknown, platform: NodeJS.Platform = process.platform): DiskChildrenRequest | null {
  const o = obj(v)
  if (!o) return null
  const p = o.path === undefined || o.path === '' ? '' : o.path
  if (p !== '' && !isAbsolutePath(p, platform)) return null
  return { path: p as string, sort: isSort(o.sort) ? o.sort : 'size', limit: limitOf(o.limit, 500) }
}

const optNum = (v: unknown): number | undefined | null => (v === undefined || v === null ? undefined : typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : null)

export function cleanSearch(v: unknown, platform: NodeJS.Platform = process.platform): DiskSearchQuery | null {
  const o = obj(v) ?? {}
  const q: DiskSearchQuery = {}
  if (o.query !== undefined) {
    if (typeof o.query !== 'string' || o.query.length > 1024) return null
    q.query = o.query
  }
  for (const k of ['minSize', 'maxSize', 'after', 'before'] as const) {
    const n = optNum(o[k])
    if (n === null) return null
    if (n !== undefined) q[k] = n
  }
  if (o.kind !== undefined && o.kind !== '') {
    if (typeof o.kind !== 'string' || !(DISK_KINDS as readonly string[]).includes(o.kind)) return null
    q.kind = o.kind
  }
  if (o.only !== undefined) {
    if (o.only !== 'file' && o.only !== 'dir') return null
    q.only = o.only
  }
  if (o.under !== undefined && o.under !== '') {
    if (!isAbsolutePath(o.under, platform)) return null
    q.under = o.under
  }
  if (o.sort !== undefined) {
    if (!isSort(o.sort)) return null
    q.sort = o.sort
  }
  q.limit = limitOf(o.limit, 100)
  return q
}

export function cleanHistoryLimit(v: unknown): number {
  const n = obj(v)?.limit
  return typeof n === 'number' && Number.isInteger(n) && n > 0 ? Math.min(n, 1000) : 50
}

export function cleanSizes(v: unknown, platform: NodeJS.Platform = process.platform): string[] | null {
  const o = obj(v)
  if (!o || !Array.isArray(o.paths) || o.paths.length < 1 || o.paths.length > MAX_SIZES) return null
  if (!o.paths.every((p) => isAbsolutePath(p, platform))) return null
  return [...new Set(o.paths as string[])]
}

export function toSizes(v: unknown): DiskSizes | null {
  const o = obj(v)
  const sizes = obj(o?.sizes)
  if (!o || !sizes || typeof o.version !== 'string') return null
  const out: Record<string, number | null> = {}
  for (const [k, n] of Object.entries(sizes)) out[k] = typeof n === 'number' && Number.isFinite(n) && n >= 0 ? n : null
  return { version: o.version, sizes: out }
}

export const MIN_DUPES_SIZE = 1024
export const MAX_DUPE_GROUPS = 5_000
export const MAX_DUPE_FILES = 60_000

const DUPE_STAGES: readonly DiskDupesStage[] = ['idle', 'size', 'quick', 'full']

export function cleanDupes(v: unknown, platform: NodeJS.Platform = process.platform): DiskDupesRequest | null {
  const o = obj(v) ?? {}
  const out: DiskDupesRequest = {}
  if (o.under !== undefined) {
    if (!Array.isArray(o.under) || o.under.length > MAX_ROOTS || !o.under.every((p) => isAbsolutePath(p, platform))) return null
    if (o.under.length) out.under = [...new Set(o.under as string[])]
  }
  if (o.minSize !== undefined) {
    if (typeof o.minSize !== 'number' || !Number.isFinite(o.minSize) || o.minSize < 0) return null
    out.minSize = Math.max(MIN_DUPES_SIZE, Math.floor(o.minSize))
  }
  if (o.excludes !== undefined) {
    const ex = o.excludes
    if (!Array.isArray(ex) || ex.length > MAX_EXCLUDES || !ex.every((e) => typeof e === 'string' && e.length > 0 && e.length <= 1024 && !e.includes('\0'))) return null
    out.excludes = ex as string[]
  }
  if (typeof o.level === 'string' && (DISK_SCAN_LEVELS as readonly string[]).includes(o.level)) out.level = o.level as DiskDupesRequest['level']
  return out
}

export function toDupesProgress(v: unknown): DiskDupesProgress {
  const o = obj(v)
  const stage = DUPE_STAGES.find((s) => s === o?.stage) ?? 'idle'
  return {
    stage,
    candidates: num(o?.candidates),
    files: num(o?.files),
    filesDone: num(o?.filesDone),
    bytes: num(o?.bytes),
    bytesRead: num(o?.bytesRead),
    groups: num(o?.groups),
    skipped: num(o?.skipped),
    current: str(o?.current),
    elapsedMs: num(o?.elapsedMs),
    paused: o?.paused === true,
  }
}

function toDupeGroup(v: unknown): DiskDupeGroup | null {
  const o = obj(v)
  if (!o || !Array.isArray(o.files)) return null
  const files = o.files.flatMap((f) => {
    const x = obj(f)
    return x && typeof x.path === 'string' ? [{ path: x.path, mtime: num(x.mtime) }] : []
  })
  if (files.length < 2) return null
  return { size: num(o.size), hash: str(o.hash), count: Math.max(files.length, num(o.count, files.length)), files }
}

export function toDupesDone(v: unknown): DiskDupesDone {
  const o = obj(v)
  const sk = obj(o?.skipped)
  const done: DiskDupesDone = {
    ok: o?.ok === true,
    groups: [],
    totalGroups: num(o?.totalGroups),
    wasted: num(o?.wasted),
    candidates: num(o?.candidates),
    hashedFiles: num(o?.hashedFiles),
    hashedBytes: num(o?.hashedBytes),
    skipped: { stale: num(sk?.stale), cloud: num(sk?.cloud), locked: num(sk?.locked), changed: num(sk?.changed), gone: num(sk?.gone) },
    ms: num(o?.ms),
  }
  if (o?.cancelled === true) done.cancelled = true
  if (typeof o?.error === 'string') done.error = o.error.slice(0, 300)
  if (Array.isArray(o?.groups)) {
    let budget = MAX_DUPE_FILES
    for (const g of o.groups) {
      if (done.groups.length >= MAX_DUPE_GROUPS || budget < 2) break
      const group = toDupeGroup(g)
      if (!group) continue
      if (group.files.length > budget) group.files = group.files.slice(0, budget)
      budget -= group.files.length
      done.groups.push(group)
    }
  }
  return done
}
