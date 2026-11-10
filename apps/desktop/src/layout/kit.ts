import type { KitSpec } from './download'

declare const __DAEMON_KIT__: { version: string; size: number; sha256: string; url: string } | undefined

export const KIT_RELEASE_BASE = 'https://github.com/JLSoft-Together/Orrery/releases/download'

export function kitSpecFor(version: string, size: number, sha256: string, url = `${KIT_RELEASE_BASE}/layout-daemon-v${version}/layout-daemon-${version}.zip`): KitSpec {
  return { version, size, sha256, url }
}

export function embeddedKit(): KitSpec | null {
  if (typeof __DAEMON_KIT__ === 'undefined' || !__DAEMON_KIT__) return null
  return kitSpecFor(__DAEMON_KIT__.version, __DAEMON_KIT__.size, __DAEMON_KIT__.sha256, __DAEMON_KIT__.url)
}
