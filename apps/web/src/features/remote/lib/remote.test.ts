import { afterEach, describe, expect, it, vi } from 'vitest'

type Bridge = Record<string, unknown>

async function load(bridge: Bridge) {
  vi.resetModules()
  ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = bridge
  return import('./remote')
}

afterEach(() => {
  delete (window as unknown as { ccmDesktop?: unknown }).ccmDesktop
})

describe('remote wrappers', () => {
  it('passes pair options only when given', async () => {
    const remotePairStart = vi.fn(async () => ({ qr: 'ORR1', exp: 9, via: 'usb' }))
    const m = await load({ remotePairStart })
    await m.remotePairStart()
    expect(remotePairStart).toHaveBeenLastCalledWith()
    const opts = { via: 'usb' as const, extraLan: ['127.0.0.1'], adbSerialHash: '0123456789abcdef' }
    expect(await m.remotePairStart(opts)).toEqual({ qr: 'ORR1', exp: 9, via: 'usb' })
    expect(remotePairStart).toHaveBeenLastCalledWith(opts)
  })

  it('sends a grace only when it is positive and caps it at 30 seconds', async () => {
    const remoteRevoke = vi.fn(async () => true)
    const remoteRevokeUndo = vi.fn(async () => true)
    const m = await load({ remoteRevoke, remoteRevokeUndo })
    expect(await m.remoteRevoke('p1')).toBe(true)
    expect(remoteRevoke).toHaveBeenLastCalledWith('p1')
    await m.remoteRevoke('p1', 5)
    expect(remoteRevoke).toHaveBeenLastCalledWith({ id: 'p1', grace: 5 })
    await m.remoteRevoke('p1', 99)
    expect(remoteRevoke).toHaveBeenLastCalledWith({ id: 'p1', grace: 30 })
    await m.remoteRevoke('p1', 0)
    expect(remoteRevoke).toHaveBeenLastCalledWith('p1')
    expect(await m.remoteRevokeUndo('p1')).toBe(true)
    expect(remoteRevokeUndo).toHaveBeenLastCalledWith('p1')
  })

  it('falls back safely when the bridge is missing or throws', async () => {
    const m = await load({ remoteRevoke: async () => Promise.reject(new Error('x')), remoteCheck: async () => Promise.reject(new Error('x')) })
    expect(await m.remoteRevoke('p1', 5)).toBe(false)
    expect(await m.remoteRevokeUndo('p1')).toBe(false)
    expect(await m.remoteCheck('p1')).toBeNull()
    expect(await m.remotePairStart({ via: 'qr' })).toBeNull()
  })

  it('returns the raw check reply', async () => {
    const reply = { lan: { ok: true, ms: 12 }, relay: { state: 'off' }, push: { ok: false, error: 'rate' } }
    const remoteCheck = vi.fn(async () => reply)
    const m = await load({ remoteCheck })
    expect(await m.remoteCheck('p1')).toEqual(reply)
    expect(remoteCheck).toHaveBeenCalledWith('p1')
  })

  it('parses the fetched event defensively', async () => {
    const m = await load({})
    expect(m.parseFetched({ ip: '192.168.1.20', at: 5 })).toEqual({ ip: '192.168.1.20', at: 5 })
    expect(m.parseFetched({ ip: '192.168.1.20' })).toEqual({ ip: '192.168.1.20', at: 0 })
    expect(m.parseFetched({ ip: 'fe80::1' })).toBeNull()
    expect(m.parseFetched({ ip: 7 })).toBeNull()
    expect(m.parseFetched(null)).toBeNull()
  })

  it('listens on the dedicated event when the shell has it', async () => {
    let fire: ((e: { ip?: unknown }) => void) | undefined
    const off = vi.fn()
    const m = await load({ onRemoteInstallFetched: (h: (e: { ip?: unknown }) => void) => ((fire = h), off) })
    const seen: string[] = []
    const stop = m.onInstallFetched((f) => seen.push(f.ip))
    fire?.({ ip: '10.0.0.5' })
    fire?.({ ip: 'bad' })
    expect(seen).toEqual(['10.0.0.5'])
    stop()
    expect(off).toHaveBeenCalled()
  })

  it('falls back to the status field and fires once per new fetch', async () => {
    let fire: ((s: unknown) => void) | undefined
    const m = await load({ onRemoteChanged: (h: (s: unknown) => void) => ((fire = h), () => undefined) })
    const seen: string[] = []
    m.onInstallFetched((f) => seen.push(`${f.ip}@${f.at}`), 100)
    fire?.({ installFetched: { ip: '10.0.0.5', at: 90 } })
    fire?.({ installFetched: null })
    fire?.({ installFetched: { ip: '10.0.0.5', at: 120 } })
    fire?.({ installFetched: { ip: '10.0.0.5', at: 120 } })
    fire?.({ installFetched: { ip: '10.0.0.6', at: 130 } })
    expect(seen).toEqual(['10.0.0.5@120', '10.0.0.6@130'])
  })
})
