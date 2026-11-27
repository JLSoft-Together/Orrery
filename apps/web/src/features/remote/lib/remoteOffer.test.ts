import { afterEach, describe, expect, it, vi } from 'vitest'

async function load(bridge: Record<string, unknown> | undefined) {
  vi.resetModules()
  const w = window as unknown as { ccmDesktop?: unknown }
  if (bridge) w.ccmDesktop = bridge
  else delete w.ccmDesktop
  return import('./remoteOffer')
}

afterEach(() => {
  delete (window as unknown as { ccmDesktop?: unknown }).ccmDesktop
})

describe('offerApkToPhone', () => {
  it('passes the session id only and parses the reply', async () => {
    const remoteOfferApk = vi.fn(async () => ({ ok: true, sent: 2 }))
    const m = await load({ remoteOfferApk })
    expect(await m.offerApkToPhone('run-1')).toEqual({ ok: true, sent: 2 })
    expect(remoteOfferApk).toHaveBeenCalledWith('run-1')
  })

  it('is unavailable without the bridge, an id, or when the bridge throws', async () => {
    expect(await (await load(undefined)).offerApkToPhone('r')).toEqual({ ok: false, error: 'unavailable' })
    const remoteOfferApk = vi.fn(async () => {
      throw new Error('x')
    })
    const m = await load({ remoteOfferApk })
    expect(await m.offerApkToPhone('r')).toEqual({ ok: false, error: 'unavailable' })
    expect(await m.offerApkToPhone('')).toEqual({ ok: false, error: 'unavailable' })
    expect(remoteOfferApk).toHaveBeenCalledTimes(1)
  })

  it('keeps known desktop errors and maps the rest to unavailable', async () => {
    const m = await load({})
    for (const error of ['off', 'noApk', 'gone', 'outside', 'notApk', 'none', 'lan', 'limit']) expect(m.parseOfferApk({ ok: false, error })).toEqual({ ok: false, error })
    expect(m.parseOfferApk({ ok: false, error: 'weird' })).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseOfferApk(null)).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseOfferApk({ ok: true, sent: -1 })).toEqual({ ok: true, sent: 0 })
  })
})
