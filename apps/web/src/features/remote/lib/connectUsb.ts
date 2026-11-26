import type { AdbDevice, RemoteDevice } from '@ccm/shared'
import type { ConnectUsbError, PairBaseline } from './connectFlow'

export const USB_PAIR_WAIT_MS = 125_000
export const USB_REVERSE_HOST = '127.0.0.1'

export type UsbStage = 'install' | 'reverse' | 'pair' | 'open' | 'waiting'
export type UsbFailure = ConnectUsbError | 'denied'
export type UsbResult = { ok: true; device: RemoteDevice } | { ok: false; error: UsbFailure; detail?: string }

export type UsbInstallReply = { ok: true } | { ok: false; error: string; detail?: string }
export type UsbWaitReply = { ok: true; device: RemoteDevice } | { ok: false; reason: 'denied' | 'timeout' | 'cancelled' }

export interface UsbPairOpts {
  via: 'usb'
  extraLan?: string[]
  adbSerialHash?: string
}

export interface UsbDeps {
  apkPath(): Promise<string | null>
  install(id: string, apk: string): Promise<UsbInstallReply>
  reverse(id: string, port: number, on: boolean): Promise<boolean>
  pairStart(opts: UsbPairOpts): Promise<{ qr: string; exp: number } | null>
  pairCancel(): Promise<void>
  openPair(id: string, qr: string): Promise<boolean>
  waitNewDevice(base: PairBaseline, timeoutMs: number, signal: AbortSignal): Promise<UsbWaitReply>
  serialHash(id: string): Promise<string>
  baseline(): PairBaseline
  port(): number
}

export type UsbCheck = { ok: true } | { ok: false; reason: 'offline' | 'unauthorized' | 'abi' }

const ARM = /arm|aarch64/i
const X86 = /x86|i686|intel/i

export function abiHint(d: Pick<AdbDevice, 'transport' | 'product' | 'device'>): 'arm' | 'x86' | 'unknown' {
  const text = `${d.product ?? ''} ${d.device ?? ''}`
  if (X86.test(text)) return 'x86'
  if (ARM.test(text)) return 'arm'
  return d.transport === 'emulator' ? 'unknown' : 'arm'
}

export function checkDevice(d: Pick<AdbDevice, 'state' | 'online' | 'transport' | 'product' | 'device'>): UsbCheck {
  if (d.state === 'unauthorized') return { ok: false, reason: 'unauthorized' }
  if (!d.online || d.state !== 'ready') return { ok: false, reason: 'offline' }
  if (abiHint(d) === 'x86') return { ok: false, reason: 'abi' }
  return { ok: true }
}

export interface UsbTarget {
  device: AdbDevice
  check: UsbCheck
}

export function usbTargets(devices: readonly AdbDevice[]): UsbTarget[] {
  return devices
    .filter((d) => d.online || d.state === 'unauthorized')
    .map((device) => ({ device, check: checkDevice(device) }))
    .sort((a, b) => Number(b.check.ok) - Number(a.check.ok) || Number(b.device.transport === 'usb') - Number(a.device.transport === 'usb'))
}

export const bestUsbTarget = (devices: readonly AdbDevice[]): UsbTarget | null => usbTargets(devices).find((t) => t.check.ok) ?? null

const INSTALL_ERRORS: Record<string, ConnectUsbError> = {
  restricted: 'restricted',
  storage: 'storage',
  signature: 'signature',
  downgrade: 'downgrade',
  abi: 'abi',
  offline: 'offline',
  unauthorized: 'unauthorized',
  timeout: 'timeout',
  'not-connected': 'offline',
  'no-devices': 'offline',
  'adb-missing': 'offline',
}

export const mapInstallError = (error: string): ConnectUsbError => INSTALL_ERRORS[error] ?? 'other'

async function reverseOff(deps: UsbDeps, id: string, port: number): Promise<void> {
  try {
    await deps.reverse(id, port, false)
  } catch {
    return
  }
}

export interface UsbRun {
  id: string
  check: UsbCheck
  onStage?: (stage: UsbStage) => void
  signal: AbortSignal
}

export async function runUsb(deps: UsbDeps, run: UsbRun): Promise<UsbResult> {
  if (!run.check.ok) return { ok: false, error: run.check.reason }
  const port = deps.port()
  const base = deps.baseline()
  let reversed = false
  let reverseTried = false
  let paired = false
  try {
    const apk = await deps.apkPath()
    if (!apk) return { ok: false, error: 'other', detail: 'apk' }
    run.onStage?.('install')
    const inst = await deps.install(run.id, apk)
    if (!inst.ok) return { ok: false, error: mapInstallError(inst.error), detail: inst.detail }
    if (run.signal.aborted) return { ok: false, error: 'offline', detail: 'cancelled' }

    run.onStage?.('reverse')
    reverseTried = true
    reversed = await deps.reverse(run.id, port, true).catch(() => false)

    run.onStage?.('pair')
    const hash = await deps.serialHash(run.id).catch(() => '')
    const start = await deps.pairStart({ via: 'usb', ...(reversed ? { extraLan: [USB_REVERSE_HOST] } : {}), ...(hash ? { adbSerialHash: hash } : {}) })
    if (!start) return { ok: false, error: 'pair', detail: 'start' }
    paired = true

    run.onStage?.('open')
    if (!(await deps.openPair(run.id, start.qr))) return { ok: false, error: 'pair', detail: 'open' }

    run.onStage?.('waiting')
    const hit = await deps.waitNewDevice(base, USB_PAIR_WAIT_MS, run.signal)
    if (hit.ok) {
      paired = false
      return { ok: true, device: hit.device }
    }
    if (hit.reason === 'denied') return { ok: false, error: 'denied' }
    return { ok: false, error: 'timeout', detail: hit.reason }
  } catch (err) {
    return { ok: false, error: 'other', detail: err instanceof Error ? err.message : String(err) }
  } finally {
    if (reverseTried) await reverseOff(deps, run.id, port)
    if (paired) await deps.pairCancel().catch(() => undefined)
  }
}
