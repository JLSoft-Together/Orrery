export type InstalledAppSource = 'registry' | 'bundle'
export type InstalledAppScope = 'machine' | 'user'

export interface InstalledApp {
  id: string
  name: string
  publisher: string | null
  version: string | null
  installedAt: number | null
  size: number | null
  sizeFrom: 'index' | 'estimate' | 'du' | null
  location: string | null
  source: InstalledAppSource
  scope: InstalledAppScope
  arch: '64' | '32' | null
  bundleId: string | null
  lastUsed: number | null
  uninstall: string | null
  blocked: 'system' | 'self' | null
}

export interface InstalledAppsResult {
  apps: InstalledApp[]
  removed: InstalledApp[]
  ms: number
  ok: boolean
  storeApps: 'settings' | 'none'
}

export type AppUninstallError = 'unknown' | 'blocked' | 'unparsable' | 'missing' | 'changed' | 'cancelled' | 'failed' | 'busy' | 'unsupported'

export interface AppUninstallResult {
  ok: boolean
  error?: AppUninstallError
  mode?: 'started' | 'trashed'
}

export type AppLeftoverBase = 'appdata' | 'localappdata' | 'programdata' | 'location' | 'support' | 'caches' | 'preferences' | 'containers' | 'savedState' | 'logs'

export interface AppLeftover {
  path: string
  base: AppLeftoverBase
  match: 'exact' | 'loose'
  size: number | null
  file: boolean
}

export interface AppLeftoversResult {
  id: string
  installed: boolean
  items: AppLeftover[]
}

export type RamLock = 'self' | 'parent' | 'system'
export type RamTag = 'agent' | 'dev' | 'self'

export interface RamProc {
  pid: number
  ppid: number
  start: string
  name: string
  ramBytes: number | null
  cpu: number | null
  tag: RamTag | null
  lock: RamLock | null
}

export interface RamSnapshot {
  at: number
  ok: boolean
  totalMem: number
  freeMem: number
  cpus: number
  procs: RamProc[]
}

export interface RamKillTarget {
  pid: number
  start: string
}

export type RamKillError = 'invalid' | 'gone' | 'locked' | 'cancelled' | 'failed' | 'busy'

export interface RamKillResult {
  ok: boolean
  killed: number
  error?: RamKillError
  lock?: RamLock
}

export const RAM_KILL_MAX = 64
export const RAM_POLL_MS = 2_000
