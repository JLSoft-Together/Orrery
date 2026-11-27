import { afterEach, describe, expect, it, vi } from 'vitest'

type Bridge = Record<string, unknown>

async function load(bridge: Bridge | undefined) {
  vi.resetModules()
  const w = window as unknown as { ccmDesktop?: unknown }
  if (bridge) w.ccmDesktop = bridge
  else delete w.ccmDesktop
  return import('./remoteClip')
}

afterEach(() => {
  delete (window as unknown as { ccmDesktop?: unknown }).ccmDesktop
})

describe('sendClipboardToPhone', () => {
  it('calls the bridge and parses the reply', async () => {
    const remoteClipSend = vi.fn(async () => ({ ok: true, sent: 1 }))
    const m = await load({ remoteClipSend })
    expect(await m.sendClipboardToPhone()).toEqual({ ok: true, sent: 1 })
    expect(remoteClipSend).toHaveBeenCalledTimes(1)
  })

  it('reports unavailable when the bridge is missing or throws', async () => {
    expect(await (await load(undefined)).sendClipboardToPhone()).toEqual({ ok: false, error: 'unavailable' })
    expect(await (await load({})).sendClipboardToPhone()).toEqual({ ok: false, error: 'unavailable' })
    const m = await load({
      remoteClipSend: async () => {
        throw new Error('boom')
      },
    })
    expect(await m.sendClipboardToPhone()).toEqual({ ok: false, error: 'unavailable' })
  })

  it('keeps the known desktop errors and maps anything else to unavailable', async () => {
    const m = await load({})
    for (const error of ['off', 'noSession', 'empty', 'limit', 'secret', 'internal']) expect(m.parseClipSend({ ok: false, error })).toEqual({ ok: false, error })
    expect(m.parseClipSend({ ok: false, error: 'weird' })).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseClipSend(null)).toEqual({ ok: false, error: 'unavailable' })
    expect(m.parseClipSend({ ok: true })).toEqual({ ok: true, sent: 0 })
    expect(m.parseClipSend({ ok: true, sent: 2.9 })).toEqual({ ok: true, sent: 2 })
  })

  it('maps results to toast keys', async () => {
    const m = await load({})
    expect(m.clipToastKey({ ok: true, sent: 1 })).toBe('remoteClip.sent')
    expect(m.clipToastKey({ ok: true, sent: 0 })).toBe('remoteClip.noSession')
    expect(m.clipToastKey({ ok: false, error: 'noSession' })).toBe('remoteClip.noSession')
    expect(m.clipToastKey({ ok: false, error: 'secret' })).toBe('remoteClip.secret')
  })
})
