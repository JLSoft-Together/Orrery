export type CleanGroup = 'dev' | 'system'
export type CleanLock = 'open' | 'run' | 'server' | 'self' | 'admin' | 'browser' | 'app' | 'denied'
export type CleanAction = 'files' | 'bin'

export interface CleanCategory {
  id: string
  group: CleanGroup
  titleKey: string
  hintKey: string
  preselect: boolean
}

export interface CleanItem {
  path: string
  size: number
  group: CleanGroup
  category: string
  kind?: string
  project?: string
  lastTouched?: number
  restore?: string
  locked?: CleanLock
  action?: CleanAction
  file?: boolean
  files?: number
  detail?: string
  unknown?: boolean
}

export interface CleanScanRequest {
  ids: string[]
  roots: string[]
  open: string[]
}

export interface CleanProgress {
  category: string
  state: 'start' | 'items' | 'done'
  items?: CleanItem[]
  count: number
  bytes: number
}

export interface CleanScanResult {
  items: CleanItem[]
  cancelled?: boolean
  busy?: boolean
}

export interface CleanDeleteRequest {
  paths: string[]
  permanent: boolean
  open: string[]
  oneTap?: boolean
}

export const CLEAN_PERMANENT_OK: readonly string[] = ['temp', 'browser']
export const CLEAN_ONE_TAP_SYSTEM: readonly string[] = ['temp', 'browser', 'crash', 'logs']
export const CLEAN_ONE_TAP_DEV: readonly string[] = ['node_modules', 'build']
export const CLEAN_ONE_TAP: readonly string[] = [...CLEAN_ONE_TAP_SYSTEM, ...CLEAN_ONE_TAP_DEV]

export interface CleanBinResult {
  done: boolean
  freed: number
  cancelled?: boolean
  error?: string
}

export const CLEAN_ROOTS_MAX = 200
export const CLEAN_OPEN_MAX = 500
export const CLEAN_DELETE_MAX = 5000

const textOk = (v: unknown, max = 4096): v is string => typeof v === 'string' && v.length > 0 && v.length <= max && !/[\u0000-\u001f]/.test(v)

function strings(raw: unknown, max: number): string[] | null {
  if (!Array.isArray(raw) || raw.length > max) return null
  const out: string[] = []
  for (const r of raw) {
    if (!textOk(r)) return null
    if (!out.includes(r)) out.push(r)
  }
  return out
}

export function parseCleanScan(raw: unknown): CleanScanRequest | null {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null
  const o = raw as Record<string, unknown>
  const ids = strings(o.ids, 32)
  const roots = strings(o.roots ?? [], CLEAN_ROOTS_MAX)
  const open = strings(o.open ?? [], CLEAN_OPEN_MAX)
  if (!ids || !roots || !open) return null
  return { ids, roots, open }
}

export function parseCleanDelete(raw: unknown): CleanDeleteRequest | null {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null
  const o = raw as Record<string, unknown>
  const paths = strings(o.paths, CLEAN_DELETE_MAX)
  const open = strings(o.open ?? [], CLEAN_OPEN_MAX)
  if (!paths || !paths.length || !open) return null
  return { paths, permanent: o.permanent === true, open, ...(o.oneTap === true ? { oneTap: true } : {}) }
}
