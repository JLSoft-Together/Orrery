import type { LayoutInstallStatus } from './layoutPreview'

export type InstallKind = 'unavailable' | 'not-installed' | 'update' | 'installed' | 'downloading' | 'error'

export function installKind(s: LayoutInstallStatus): InstallKind {
  if (s.bundle.state === 'downloading') return 'downloading'
  if (!s.available) return 'unavailable'
  if (s.installed) return 'installed'
  if (s.bundle.state === 'error') return 'error'
  return s.version !== null && s.pinnedVersion !== null && s.version !== s.pinnedVersion ? 'update' : 'not-installed'
}

export const megabytes = (bytes: number): string => (bytes < 1024 * 1024 ? '<1' : String(Math.round(bytes / 1024 / 1024)))
