import { beforeEach, describe, expect, it, vi } from 'vitest'

const remote = vi.hoisted(() => ({
  remoteStatus: vi.fn(async () => null),
  remoteEnable: vi.fn(async () => null),
  remotePairStart: vi.fn(async (): Promise<unknown> => ({ qr: 'ORR1', exp: 9, relay: true, lan: 2, via: 'usb' })),
  remotePairCancel: vi.fn(async () => true),
  remoteCheck: vi.fn(async (): Promise<unknown> => ({ lan: { ok: true } })),
  onInstallFetched: vi.fn((handler: (f: { ip: string; at: number }) => void, since?: number) => {
    void since
    handler({ ip: '192.168.1.40', at: 5 })
    return () => undefined
  }),
}))
const install = vi.hoisted(() => ({
  installArm: vi.fn(async () => true),
  installGet: vi.fn(),
  installStart: vi.fn(),
  installStop: vi.fn(),
  installUseSuggested: vi.fn(),
}))

vi.mock('./remote', () => remote)
vi.mock('./remoteInstall', () => install)
vi.mock('../../devices/lib/devices', () => ({
  devInstall: vi.fn(),
  devOpenPair: vi.fn(),
  devReverse: vi.fn(),
  devSnapshot: vi.fn(async () => null),
  hasDeviceInstall: false,
  onDevEvent: vi.fn(() => () => undefined),
}))

const { connectApi, installPagePath, parsePairReply } = await import('./connectApi')

beforeEach(() => {
  vi.clearAllMocks()
})

describe('connectApi', () => {
  it('starts pairing through the typed wrapper and parses the reply', async () => {
    const opts = { via: 'usb' as const, extraLan: ['127.0.0.1'], adbSerialHash: '0123456789abcdef' }
    expect(await connectApi.pairStart(opts)).toEqual({ qr: 'ORR1', exp: 9, relay: true, lan: 2 })
    expect(remote.remotePairStart).toHaveBeenLastCalledWith(opts)
    remote.remotePairStart.mockResolvedValueOnce(null)
    expect(await connectApi.pairStart({ via: 'qr' })).toBeNull()
    expect(parsePairReply({ qr: '', exp: 1 })).toBeNull()
  })

  it('checks, arms and listens through the remote wrappers', async () => {
    expect(await connectApi.check('d1')).toEqual({ lan: { ok: true } })
    expect(remote.remoteCheck).toHaveBeenLastCalledWith('d1')
    expect(await connectApi.installArm(false)).toBe(true)
    expect(install.installArm).toHaveBeenLastCalledWith(false)
    const seen: string[] = []
    connectApi.onInstallFetched((ip) => seen.push(ip), 1234)
    expect(seen).toEqual(['192.168.1.40'])
    expect(remote.onInstallFetched).toHaveBeenLastCalledWith(expect.any(Function), 1234)
  })

  it('uses the page path from the ticket and falls back to cutting the APK name', () => {
    expect(installPagePath({ path: '/install/tok/orrery.apk', page: '/install/tok/' })).toBe('/install/tok/')
    expect(installPagePath({ path: '/install/tok/orrery.apk', page: '/install/other/' })).toBe('/install/other/')
    expect(installPagePath({ path: '/install/tok/orrery.apk' })).toBe('/install/tok/')
    expect(installPagePath({ path: '/install/tok/orrery.apk', page: '' })).toBe('/install/tok/')
    expect(installPagePath({ path: '/install/tok/orrery.apk', page: 'http://evil/' })).toBe('/install/tok/')
  })
})
