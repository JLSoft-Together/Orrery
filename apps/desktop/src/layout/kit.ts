import type { KitSpec } from './download'

declare const __DAEMON_KIT__: { version: string; size: number; sha256: string } | undefined

export const KIT_RELEASE_BASE = 'https://github.com/JLSoft-Together/Orrery/releases/download'

export function kitSpecFor(version: string, size: number, sha256: string): KitSpec {
  return { version, size, sha256, url: `${KIT_RELEASE_BASE}/v${version}/layout-daemon-${version}.zip` }
}

export function embeddedKit(): KitSpec | null {
  if (typeof __DAEMON_KIT__ === 'undefined' || !__DAEMON_KIT__) return null
  return kitSpecFor(__DAEMON_KIT__.version, __DAEMON_KIT__.size, __DAEMON_KIT__.sha256)
}
