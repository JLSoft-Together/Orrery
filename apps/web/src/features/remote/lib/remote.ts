import type { RemoteDevice, RemotePairStart, RemoteStatus, RemoteWorkspaceSnapshot } from '@ccm/shared'
import { desktop, hasRemote, type RemotePairOpts } from '../../../core/lib/desktop'

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
export const remotePairStart = async (opts?: RemotePairOpts): Promise<RemotePairStart | null> =>
  (await (opts ? desktop?.remotePairStart?.(opts) : desktop?.remotePairStart?.())?.catch(() => null)) ?? null
export const remotePairCancel = async (): Promise<boolean> => (await desktop?.remotePairCancel?.().catch(() => false)) ?? false
export const remoteRevoke = async (deviceId: string, grace?: number): Promise<boolean> =>
  (await desktop?.remoteRevoke?.(grace && grace > 0 ? { id: deviceId, grace: Math.min(30, Math.round(grace)) } : deviceId).catch(() => false)) === true
export const remoteRevokeUndo = async (deviceId: string): Promise<boolean> => (await desktop?.remoteRevokeUndo?.(deviceId).catch(() => false)) === true
export const remoteCheck = async (deviceId: string): Promise<unknown> => (await desktop?.remoteCheck?.(deviceId).catch(() => null)) ?? null

export interface InstallFetched {
  ip: string
  at: number
}

const IPV4 = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/

export function parseFetched(v: unknown): InstallFetched | null {
  if (typeof v !== 'object' || v === null) return null
  const { ip, at } = v as { ip?: unknown; at?: unknown }
  if (typeof ip !== 'string' || !IPV4.test(ip)) return null
  return { ip, at: typeof at === 'number' && at > 0 ? at : 0 }
}

export function onInstallFetched(handler: (fetched: InstallFetched) => void, since = 0): () => void {
  if (desktop?.onRemoteInstallFetched) {
    return desktop.onRemoteInstallFetched((e) => {
      const hit = parseFetched(e)
      if (hit) handler(hit)
    })
  }
  let last = since
  return onRemoteChanged((status) => {
    const hit = parseFetched((status as unknown as { installFetched?: unknown }).installFetched)
    if (!hit || hit.at <= last) return
    last = hit.at
    handler(hit)
  })
}
export const remoteDeviceRename = async (deviceId: string, name: string): Promise<boolean> => (await desktop?.remoteDeviceRename?.(deviceId, name).catch(() => false)) ?? false
export const remoteRevokeAll = async (): Promise<boolean> => (await desktop?.remoteRevokeAll?.().catch(() => false)) ?? false
export const remotePushDevice = async (deviceId: string, on: boolean): Promise<boolean> => {
  const r = (await desktop?.remotePushDevice?.(deviceId, on).catch(() => null)) as { ok?: unknown } | null | undefined
  return r?.ok === true
}
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
