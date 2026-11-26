import type { AdbDevice, DevSnapshot, RemoteStatus } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { devInstall, devOpenPair, devReverse, devSnapshot, hasDeviceInstall, onDevEvent } from '../../devices/lib/devices'
import { remoteEnable, remotePairCancel, remoteStatus } from './remote'
import { installGet, installStart, installStop, installUseSuggested, type InstallReply, type InstallState, type InstallTicket } from './remoteInstall'
import type { UsbInstallReply, UsbPairOpts } from './connectUsb'

export type ConnectApiError = 'unavailable'
export type ConnectPairReply = { qr: string; exp: number; relay?: boolean; lan?: number } | null

export interface ConnectApi {
  hasUsb: boolean
  remoteStatus(): Promise<RemoteStatus | null>
  remoteEnable(on: boolean): Promise<RemoteStatus | null>
  pairStart(opts: UsbPairOpts | { via: 'qr' }): Promise<ConnectPairReply>
  pairCancel(): Promise<boolean>
  check(deviceId: string): Promise<unknown>
  installState(): Promise<InstallReply<InstallState>>
  installUseSuggested(): Promise<InstallReply<InstallState>>
  installStart(): Promise<InstallReply<InstallTicket>>
  installStop(): Promise<void>
  installArm(on: boolean): Promise<boolean>
  onInstallFetched(handler: (ip: string) => void): () => void
  devices(): Promise<AdbDevice[]>
  onDevices(handler: (devices: AdbDevice[]) => void): () => void
  usbInstall(id: string, apk: string): Promise<UsbInstallReply>
  usbReverse(id: string, port: number, on: boolean): Promise<boolean>
  usbOpenPair(id: string, qr: string): Promise<boolean>
  serialHash(id: string): Promise<string>
  qrSvg(text: string): Promise<string>
}

interface E1Bridge {
  remotePairStart?: (opts?: unknown) => Promise<unknown>
  remoteCheck?: (deviceId: string) => Promise<unknown>
  remoteInstall?: (op: { op: string }) => Promise<unknown>
  onRemoteInstallFetched?: (handler: (e: { ip?: unknown }) => void) => () => void
}

const bridge = desktop as unknown as E1Bridge | undefined

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null

export function parsePairReply(v: unknown): ConnectPairReply {
  if (!isObj(v) || typeof v.qr !== 'string' || !v.qr || typeof v.exp !== 'number') return null
  return { qr: v.qr, exp: v.exp, relay: v.relay === true, lan: typeof v.lan === 'number' ? v.lan : undefined }
}

export async function hashSerial(id: string): Promise<string> {
  const data = new TextEncoder().encode(id)
  const digest = await crypto.subtle.digest('SHA-256', data)
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
    .slice(0, 16)
}

async function snapshotDevices(): Promise<AdbDevice[]> {
  const snap: DevSnapshot | null = await devSnapshot()
  return snap?.devices ?? []
}

export const connectApi: ConnectApi = {
  get hasUsb() {
    return hasDeviceInstall
  },
  remoteStatus,
  remoteEnable,
  pairStart: async (opts) => parsePairReply(await bridge?.remotePairStart?.(opts).catch(() => null)),
  pairCancel: remotePairCancel,
  check: async (id) => (await bridge?.remoteCheck?.(id).catch(() => null)) ?? null,
  installState: installGet,
  installUseSuggested,
  installStart,
  installStop,
  installArm: async (on) => {
    const r = await bridge?.remoteInstall?.({ op: on ? 'arm' : 'disarm' }).catch(() => null)
    return isObj(r) && r.ok === true
  },
  onInstallFetched: (handler) => bridge?.onRemoteInstallFetched?.((e) => handler(typeof e?.ip === 'string' ? e.ip : '')) ?? (() => undefined),
  devices: snapshotDevices,
  onDevices: (handler) => onDevEvent((e) => e.type === 'devices' && handler(e.devices)),
  usbInstall: async (id, apk) => {
    const r = await devInstall(id, apk)
    return r.ok ? { ok: true } : { ok: false, error: r.error, detail: r.detail }
  },
  usbReverse: async (id, port, on) => (await devReverse(id, port, on)).ok,
  usbOpenPair: async (id, qr) => (await devOpenPair(id, qr)).ok,
  serialHash: hashSerial,
  qrSvg: (text) => import('qrcode').then((m) => m.default.toString(text, { type: 'svg', margin: 1, errorCorrectionLevel: 'M' })).catch(() => ''),
}
