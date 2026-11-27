import type { DevPkgVersion } from './devices'
import type { DeviceRow } from './mergeDevices'

export type RowAction = 'update' | 'install' | 'boot' | 'blocked'

const parse = (v: string): number[] | null => {
  const m = /^v?(\d+(?:\.\d+)*)/.exec(v.trim())
  return m ? (m[1] as string).split('.').map(Number) : null
}

export function compareVersions(a: string, b: string): -1 | 0 | 1 | null {
  const x = parse(a)
  const y = parse(b)
  if (!x || !y) return null
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0)
    if (d) return d < 0 ? -1 : 1
  }
  return 0
}

export const isOlder = (version: string, than: string): boolean => compareVersions(version, than) === -1

export const isX86Emulator = (row: DeviceRow): boolean => /x86/i.test(row.avd?.abi ?? '')

export function installedVersion(row: DeviceRow, pkg: DevPkgVersion | undefined): string {
  if (pkg && pkg.ok && pkg.installed) return pkg.versionName
  return row.remote?.appVersion ?? ''
}

export interface ActionContext {
  desktopVersion: string
  pkg: Readonly<Record<string, DevPkgVersion | undefined>>
  canInstall: boolean
}

export function rowAction(row: DeviceRow, ctx: ActionContext): RowAction | null {
  if (row.remote?.revoking) return null
  if (row.avd && !row.adb) return 'boot'
  const adb = row.adb
  if (!adb || !adb.online || adb.state !== 'ready' || !ctx.canInstall) return null
  const pkg = ctx.pkg[adb.id]
  if (!pkg || !pkg.ok) return null
  if (!pkg.installed) return row.kind === 'emulator' && isX86Emulator(row) ? 'blocked' : 'install'
  const current = installedVersion(row, pkg)
  return current && ctx.desktopVersion && isOlder(current, ctx.desktopVersion) ? 'update' : null
}
