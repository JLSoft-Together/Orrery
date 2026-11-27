import { beforeEach, describe, expect, it, vi } from 'vitest'

const bridge = vi.hoisted(() => ({
  devInstall: vi.fn(),
  devOpenPair: vi.fn(),
  devReverse: vi.fn(),
  devPkgVersion: vi.fn(),
}))

vi.mock('../../../core/lib/desktop', () => ({ desktop: bridge }))

import { devInstall, devOpenPair, devPkgVersion, devReverse, hasDeviceInstall } from './devices'

describe('device install bridge', () => {
  beforeEach(() => {
    Object.values(bridge).forEach((f) => f.mockReset())
  })

  it('forwards every call to the desktop channel', async () => {
    bridge.devInstall.mockResolvedValue({ ok: false, error: 'signature' })
    bridge.devOpenPair.mockResolvedValue({ ok: true })
    bridge.devReverse.mockResolvedValue({ ok: true })
    bridge.devPkgVersion.mockResolvedValue({ ok: true, installed: true, versionName: '1.2.3', versionCode: 7 })
    expect(hasDeviceInstall).toBe(true)
    expect(await devInstall('R58M', 'C:\a.apk')).toEqual({ ok: false, error: 'signature' })
    expect(await devOpenPair('R58M', 'qr&x')).toEqual({ ok: true })
    expect(await devReverse('R58M', 4317, true)).toEqual({ ok: true })
    expect(await devPkgVersion('R58M', 'dev.ccm.orrery.remote')).toMatchObject({ versionName: '1.2.3' })
    expect(bridge.devInstall).toHaveBeenCalledWith('R58M', 'C:\a.apk')
    expect(bridge.devReverse).toHaveBeenCalledWith('R58M', 4317, true)
  })
})
