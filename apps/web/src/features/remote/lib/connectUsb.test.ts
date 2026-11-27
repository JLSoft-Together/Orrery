import { describe, expect, it, vi } from 'vitest'
import type { AdbDevice, RemoteDevice } from '@ccm/shared'
import { pairBaseline } from './connectFlow'
import { abiHint, bestUsbTarget, checkDevice, mapInstallError, runUsb, usbTargets, type UsbDeps, type UsbStage } from './connectUsb'

const adb = (over: Partial<AdbDevice> = {}): AdbDevice => ({ id: 'SER1', state: 'ready', transport: 'usb', model: 'SM-A336E', product: 'a33xnsxx', device: 'a33x', transportId: '1', alias: null, name: 'Galaxy A33', online: true, lastSeen: 1, ...over })
const phone: RemoteDevice = { deviceId: 'd1', name: 'Galaxy A33', platform: 'android', pairedAt: 500, lastSeen: 500, state: 'connected' }

function deps(over: Partial<UsbDeps> = {}): UsbDeps & { calls: string[] } {
  const calls: string[] = []
  const d: UsbDeps = {
    apkPath: async () => 'C:/orrery.apk',
    install: async () => (calls.push('install'), { ok: true }),
    reverse: async (_id, port, on) => (calls.push(`reverse:${port}:${on}`), true),
    pairStart: async (o) => (calls.push(`pairStart:${JSON.stringify(o)}`), { qr: 'QR', exp: 9 }),
    pairCancel: async () => void calls.push('pairCancel'),
    openPair: async (_id, qr) => (calls.push(`open:${qr}`), true),
    waitNewDevice: async () => (calls.push('wait'), { ok: true, device: phone }),
    serialHash: async () => 'abcdef0123456789',
    baseline: () => pairBaseline([]),
    port: () => 4330,
    ...over,
  }
  return Object.assign(d, { calls })
}

const ctl = (): AbortSignal => new AbortController().signal

describe('checkDevice', () => {
  it('accepts a ready usb phone', () => {
    expect(checkDevice(adb())).toEqual({ ok: true })
  })

  it('rejects unauthorized, offline and x86 emulators with a reason', () => {
    expect(checkDevice(adb({ state: 'unauthorized' }))).toEqual({ ok: false, reason: 'unauthorized' })
    expect(checkDevice(adb({ state: 'offline' }))).toEqual({ ok: false, reason: 'offline' })
    expect(checkDevice(adb({ online: false }))).toEqual({ ok: false, reason: 'offline' })
    expect(checkDevice(adb({ transport: 'emulator', product: 'sdk_gphone64_x86_64' }))).toEqual({ ok: false, reason: 'abi' })
  })

  it('accepts an arm emulator and treats an unknown emulator as allowed', () => {
    expect(checkDevice(adb({ transport: 'emulator', product: 'sdk_gphone64_arm64' })).ok).toBe(true)
    expect(abiHint(adb({ transport: 'emulator', product: 'x', device: 'y' }))).toBe('unknown')
    expect(checkDevice(adb({ transport: 'emulator', product: 'x', device: 'y' })).ok).toBe(true)
  })
})

describe('usbTargets', () => {
  it('sorts usable devices first and prefers cable over Wi-Fi', () => {
    const list = [adb({ id: 'E', transport: 'emulator', product: 'sdk_x86_64' }), adb({ id: 'W', transport: 'wifi' }), adb({ id: 'U' })]
    expect(usbTargets(list).map((t) => t.device.id)).toEqual(['U', 'W', 'E'])
    expect(bestUsbTarget(list)?.device.id).toBe('U')
  })

  it('returns null when nothing is usable and drops remembered offline devices', () => {
    expect(bestUsbTarget([adb({ transport: 'emulator', product: 'x86' })])).toBeNull()
    expect(usbTargets([adb({ online: false, state: 'unknown' })])).toEqual([])
  })
})

describe('mapInstallError', () => {
  it('keeps known codes and folds the rest into other', () => {
    expect(mapInstallError('restricted')).toBe('restricted')
    expect(mapInstallError('signature')).toBe('signature')
    expect(mapInstallError('not-connected')).toBe('offline')
    expect(mapInstallError('whatever')).toBe('other')
  })
})

describe('runUsb', () => {
  it('runs install, reverse, pair, open, wait and removes the reverse', async () => {
    const d = deps()
    const stages: UsbStage[] = []
    const r = await runUsb(d, { id: 'SER1', check: { ok: true }, signal: ctl(), onStage: (s) => stages.push(s) })
    expect(r).toEqual({ ok: true, device: phone })
    expect(stages).toEqual(['install', 'reverse', 'pair', 'open', 'waiting'])
    expect(d.calls).toEqual(['install', 'reverse:4330:true', 'pairStart:{"via":"usb","extraLan":["127.0.0.1"],"adbSerialHash":"abcdef0123456789"}', 'open:QR', 'wait', 'reverse:4330:false'])
  })

  it('refuses a device that failed the check without touching adb', async () => {
    const d = deps()
    expect(await runUsb(d, { id: 'S', check: { ok: false, reason: 'abi' }, signal: ctl() })).toEqual({ ok: false, error: 'abi' })
    expect(d.calls).toEqual([])
  })

  it.each([
    ['restricted', 'restricted'],
    ['storage', 'storage'],
    ['signature', 'signature'],
    ['downgrade', 'downgrade'],
    ['abi', 'abi'],
    ['timeout', 'timeout'],
    ['unauthorized', 'unauthorized'],
    ['weird', 'other'],
  ])('install error %s maps to %s and nothing else runs', async (err, mapped) => {
    const d = deps({ install: async () => ({ ok: false, error: err, detail: 'x' }) })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toEqual({ ok: false, error: mapped, detail: 'x' })
    expect(d.calls.some((c) => c.startsWith('reverse') || c.startsWith('pairStart'))).toBe(false)
  })

  it('fails with other when there is no APK', async () => {
    const r = await runUsb(deps({ apkPath: async () => null }), { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toMatchObject({ ok: false, error: 'other' })
  })

  it('pairs without extraLan when the reverse failed, and still tries to remove it', async () => {
    const d = deps({ reverse: vi.fn(async () => false) })
    await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(d.calls.find((c) => c.startsWith('pairStart'))).toBe('pairStart:{"via":"usb","adbSerialHash":"abcdef0123456789"}')
    expect(d.reverse).toHaveBeenLastCalledWith('S', 4330, false)
  })

  it('pair start failure removes the reverse and reports pair', async () => {
    const d = deps({ pairStart: async () => null })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toMatchObject({ ok: false, error: 'pair' })
    expect(d.calls.at(-1)).toBe('reverse:4330:false')
  })

  it('open failure removes the reverse and cancels the pairing code', async () => {
    const d = deps({ openPair: async () => false })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toMatchObject({ ok: false, error: 'pair' })
    expect(d.calls).toContain('reverse:4330:false')
    expect(d.calls).toContain('pairCancel')
  })

  it('denied on the computer is its own result and removes the reverse', async () => {
    const d = deps({ waitNewDevice: async () => ({ ok: false, reason: 'denied' }) })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toEqual({ ok: false, error: 'denied' })
    expect(d.calls).toContain('reverse:4330:false')
    expect(d.calls).toContain('pairCancel')
  })

  it('timeout waiting for the phone reports timeout', async () => {
    const d = deps({ waitNewDevice: async () => ({ ok: false, reason: 'timeout' }) })
    expect(await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })).toMatchObject({ ok: false, error: 'timeout' })
  })

  it('a thrown error still removes the reverse', async () => {
    const d = deps({
      openPair: async () => {
        throw new Error('boom')
      },
    })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(r).toEqual({ ok: false, error: 'other', detail: 'boom' })
    expect(d.calls).toContain('reverse:4330:false')
  })

  it('a failing reverse removal does not hide the result', async () => {
    let n = 0
    const d = deps({
      reverse: async (_i, _p, on) => {
        if (!on) throw new Error('adb gone')
        n++
        return true
      },
    })
    expect((await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })).ok).toBe(true)
    expect(n).toBe(1)
  })

  it('re-pairing an old phone passes the baseline taken before the pair', async () => {
    const seen: unknown[] = []
    const old: RemoteDevice = { ...phone, pairedAt: 100 }
    const d = deps({
      baseline: () => pairBaseline([old]),
      waitNewDevice: async (base) => (seen.push(base.pairedAt.get('d1')), { ok: true, device: { ...phone, pairedAt: 900 } }),
    })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ctl() })
    expect(seen).toEqual([100])
    expect(r.ok && r.device.pairedAt).toBe(900)
  })

  it('cancel after install stops before pairing', async () => {
    const ac = new AbortController()
    const d = deps({ install: async () => (ac.abort(), { ok: true }) })
    const r = await runUsb(d, { id: 'S', check: { ok: true }, signal: ac.signal })
    expect(r.ok).toBe(false)
    expect(d.calls.some((c) => c.startsWith('pairStart'))).toBe(false)
  })
})
