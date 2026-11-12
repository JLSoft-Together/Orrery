import type { RemoteDevice, RemotePairStart, RemoteStatus, RemoteWorkspaceSnapshot } from '@ccm/shared'
import { desktop, hasRemote } from './desktop'

export { hasRemote }

export const REMOTE_PAIR_MS = 2 * 60_000
export const REMOTE_TOUCH_MS = 60_000
export const REMOTE_LOG_MAX = 50

export interface RemoteLogLine {
  id: number
  at: number
  device: string
  text: string
  ok: boolean
}

export const remoteStatus = async (): Promise<RemoteStatus | null> => (await desktop?.remoteStatus?.().catch(() => null)) ?? null
export const remoteEnable = async (on: boolean): Promise<RemoteStatus | null> => (await desktop?.remoteEnable?.(on).catch(() => null)) ?? null
export const remoteKeepAwake = async (on: boolean): Promise<RemoteStatus | null> => (await desktop?.remoteKeepAwake?.(on).catch(() => null)) ?? null
export const remotePairStart = async (): Promise<RemotePairStart | null> => (await desktop?.remotePairStart?.().catch(() => null)) ?? null
export const remotePairCancel = async (): Promise<boolean> => (await desktop?.remotePairCancel?.().catch(() => false)) ?? false
export const remoteRevoke = async (deviceId: string): Promise<boolean> => (await desktop?.remoteRevoke?.(deviceId).catch(() => false)) ?? false
export const remoteDeviceRename = async (deviceId: string, name: string): Promise<boolean> => (await desktop?.remoteDeviceRename?.(deviceId, name).catch(() => false)) ?? false
export const remoteRevokeAll = async (): Promise<boolean> => (await desktop?.remoteRevokeAll?.().catch(() => false)) ?? false
export const remotePublish = async (summary: RemoteWorkspaceSnapshot): Promise<boolean> => (await desktop?.remotePublish?.(summary).catch(() => false)) ?? false
export const onRemoteChanged = (handler: (status: RemoteStatus) => void): (() => void) => desktop?.onRemoteChanged?.(handler) ?? (() => undefined)

export const isConnected = (d: RemoteDevice): boolean => d.state === 'connected'

export const activeDevices = (devices: readonly RemoteDevice[]): RemoteDevice[] => devices.filter((d) => d.state !== 'revoked')

export const expMs = (exp: number): number => (exp < 1e12 ? exp * 1000 : exp)

export const pairLeftMs = (exp: number, at: number): number => Math.max(0, expMs(exp) - at)

export function clock(ms: number): string {
  const s = Math.ceil(ms / 1000)
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}
