import { afterEach, describe, expect, it, vi } from 'vitest'

type Bridge = Record<string, unknown>

async function load(bridge: Bridge | undefined) {
  vi.resetModules()
  const w = window as unknown as { ccmDesktop?: unknown }
  if (bridge) w.ccmDesktop = bridge
  else delete w.ccmDesktop
  return import('./remotePush')
}

afterEach(() => {
  delete (window as unknown as { ccmDesktop?: unknown }).ccmDesktop
})

describe('sendPaneToPhone', () => {
  it('calls the bridge with the pane key and parses the reply', async () => {
    const remoteSendPane = vi.fn(async () => ({ ok: true, sent: 2 }))
    const m = await load({ remoteSendPane })
    expect(await m.sendPaneToPhone('pane-1')).toEqual({ ok: true, sent: 2 })
    expect(remoteSendPane).toHaveBeenCalledWith('pane-1')
  })

  it('returns a clear error when the bridge is missing or throws, without calling anything', async () => {
    expect(await (await load(undefined)).sendPaneToPhone('p')).toEqual({ ok: false, error: 'unavailable' })
    expect(await (await load({})).sendPaneToPhone('p')).toEqual({ ok: false, error: 'unavailable' })
    const remoteSendPane = vi.fn(async () => {
      throw new Error('boom')
    })
    const m = await load({ remoteSendPane })
    expect(await m.sendPaneToPhone('p')).toEqual({ ok: false, error: 'unavailable' })
    expect(await m.sendPaneToPhone('')).toEqual({ ok: false, error: 'unavailable' })
    expect(remoteSendPane).toHaveBeenCalledTimes(1)
  })

  it('keeps the known desktop errors and maps anything else to unavailable', async () => {
    const m = await load({})
    for (const error of ['off', 'invalid', 'notFound', 'none', 'limit']) expect(m.parseSendPane({ ok: false, error })).toEqual({ ok: false, error })
    expect(m.parseSendPane({ ok: false, error: 'weird' })).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseSendPane(null)).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseSendPane('x')).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseSendPane({ ok: true })).toEqual({ ok: true, sent: 0 })
    expect(m.parseSendPane({ ok: true, sent: -3 })).toEqual({ ok: true, sent: 0 })
    expect(m.parseSendPane({ ok: true, sent: 2.9 })).toEqual({ ok: true, sent: 2 })
  })
})

describe('hasPushPhone', () => {
  it('is true only for a device with push on, a push channel and not revoking', async () => {
    const m = await load({})
    expect(m.hasPushPhone(undefined)).toBe(false)
    expect(m.hasPushPhone([])).toBe(false)
    expect(m.hasPushPhone([{ push: 'none', pushOn: true }])).toBe(false)
    expect(m.hasPushPhone([{ push: 'fcm', pushOn: false }])).toBe(false)
    expect(m.hasPushPhone([{ push: 'fcm', pushOn: true, revoking: true }])).toBe(false)
    expect(m.hasPushPhone([{ push: 'none' }, { push: 'fcm', pushOn: true }])).toBe(true)
    expect(m.hasPushPhone([{ push: 'web' }])).toBe(true)
  })
})
