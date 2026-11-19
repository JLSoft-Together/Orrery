import type { RemoteDevice, RemoteGuardBlock, RemoteStatus } from '@ccm/shared'

export type PhoneTone = 'idle' | 'live' | 'alert'

export interface PhoneChip {
  show: boolean
  tone: PhoneTone
  connected: number
  paired: number
  pushOn: number
  pushReady: number
  blocked: number
}

export const PHONE_ALERT_SEC = 24 * 3600

const active = (d: RemoteDevice): boolean => d.state !== 'revoked'
const canPush = (d: RemoteDevice): boolean => !!d.push && d.push !== 'none'

export function recentBlocks(blocked: readonly RemoteGuardBlock[], nowSec: number): RemoteGuardBlock[] {
  return blocked.filter((b) => b.source !== 'manual' && (b.until === null || b.until > nowSec) && nowSec - b.at < PHONE_ALERT_SEC)
}

export function phoneChip(status: RemoteStatus | null, blocked: readonly RemoteGuardBlock[], nowMs: number): PhoneChip {
  const devices = (status?.devices ?? []).filter(active)
  const ready = devices.filter(canPush)
  const alerts = status?.enabled ? recentBlocks(blocked, Math.floor(nowMs / 1000)).length : 0
  const connected = status?.enabled ? devices.filter((d) => d.state === 'connected').length : 0
  return {
    show: !!status?.enabled,
    tone: alerts ? 'alert' : connected ? 'live' : 'idle',
    connected,
    paired: devices.length,
    pushOn: ready.filter((d) => d.pushOn !== false).length,
    pushReady: ready.length,
    blocked: alerts,
  }
}

export const pushMuted = (c: PhoneChip): boolean => c.pushReady > 0 && c.pushOn === 0
