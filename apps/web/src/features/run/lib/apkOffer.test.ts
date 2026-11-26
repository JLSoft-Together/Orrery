import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { RunSession } from '@ccm/shared'

const offerApkToPhone = vi.fn()
const devices = { value: [{ push: 'fcm', pushOn: true }] as unknown[] }

vi.mock('@features/remote', () => ({
  offerApkToPhone: (id: string) => offerApkToPhone(id),
  hasPushPhone: (d: unknown[]) => d.length > 0,
  useRemoteStore: () => ({ get devices() { return devices.value } }),
}))

vi.mock('../../../i18n', () => ({ i18n: { global: { t: (key: string) => key } } }))

const run = (over: Record<string, unknown> = {}): RunSession => ({ id: 's1', cwd: 'C:/work/app', state: 'installing', ...over }) as unknown as RunSession
const apk = { apk: { path: 'x', size: 1, applicationId: 'a', variant: 'd' } }

async function load() {
  vi.resetModules()
  setActivePinia(createPinia())
  const runApk = await import('./runApk')
  const lib = await import('./apkOffer')
  const toasts = (await import('../../../core/stores/toasts')).useToastsStore()
  return { runApk, lib, toasts }
}

beforeEach(() => {
  localStorage.clear()
  offerApkToPhone.mockReset()
  offerApkToPhone.mockResolvedValue({ ok: true, sent: 1 })
  devices.value = [{ push: 'fcm', pushOn: true }]
})

describe('maybeAutoOffer', () => {
  it('sends exactly once when the switch is on and the apk first shows up', async () => {
    const { runApk, lib } = await load()
    runApk.setAutoApk('C:/work/app', true)
    lib.maybeAutoOffer(run(), run(apk))
    lib.maybeAutoOffer(run(), run({ ...apk, state: 'running' }))
    lib.maybeAutoOffer(run(apk), run({ ...apk, state: 'running' }))
    await Promise.resolve()
    expect(offerApkToPhone).toHaveBeenCalledTimes(1)
    expect(offerApkToPhone).toHaveBeenCalledWith('s1')
  })

  it('never sends with the switch off, without a push phone, or without an apk', async () => {
    const { runApk, lib } = await load()
    lib.maybeAutoOffer(run(), run(apk))
    runApk.setAutoApk('C:/work/app', true)
    lib.maybeAutoOffer(run(), run())
    devices.value = []
    lib.maybeAutoOffer(run(), run({ id: 's2', ...apk }))
    expect(offerApkToPhone).not.toHaveBeenCalled()
  })
})

describe('offerSession', () => {
  it('shows a warning toast on failure and an info toast on success', async () => {
    const { lib, toasts } = await load()
    offerApkToPhone.mockResolvedValueOnce({ ok: false, error: 'outside' })
    await lib.offerSession('s1')
    expect(toasts.list[0]?.kind).toBe('warning')
    offerApkToPhone.mockResolvedValueOnce({ ok: true, sent: 2 })
    await lib.offerSession('s1')
    expect(toasts.list).toHaveLength(1)
    expect(toasts.list[0]?.kind).toBe('info')
  })

  it('maps results to i18n keys', async () => {
    const { lib } = await load()
    expect(lib.offerToastKey({ ok: true, sent: 1 })).toBe('runApk.sent')
    expect(lib.offerToastKey({ ok: true, sent: 0 })).toBe('runApk.none')
    expect(lib.offerToastKey({ ok: false, error: 'gone' })).toBe('runApk.gone')
    expect(lib.offerToastKey({ ok: false, error: 'invalid' })).toBe('runApk.unavailable')
  })
})
