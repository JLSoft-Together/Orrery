import { describe, expect, it } from 'vitest'
import type { RemoteDevice, RemoteGuardBlock, RemoteStatus } from '@ccm/shared'
import { phoneChip, pushMuted, recentBlocks } from './phoneChip'

const dev = (id: string, over: Partial<RemoteDevice> = {}): RemoteDevice => ({ deviceId: id, name: id, platform: 'android', pairedAt: 1, lastSeen: 1, state: 'idle', ...over })
const status = (devices: RemoteDevice[], enabled = true): RemoteStatus => ({ enabled, listening: true, addresses: [], port: 4320, relay: 'off', keepAwake: true, devices })
const block = (over: Partial<RemoteGuardBlock> = {}): RemoteGuardBlock => ({ key: 'k', source: 'relay', why: 'token', count: 5, at: 1000, until: null, level: 1, ...over })
const NOW = 1000 * 1000

describe('phoneChip', () => {
  it('hides when remote control is off or unknown', () => {
    expect(phoneChip(null, [], NOW).show).toBe(false)
    const off = phoneChip(status([dev('a', { state: 'connected' })], false), [block()], NOW)
    expect(off).toMatchObject({ show: false, connected: 0, blocked: 0, tone: 'idle' })
  })

  it('counts connected and paired phones, skipping revoked ones', () => {
    const c = phoneChip(status([dev('a', { state: 'connected' }), dev('b'), dev('c', { state: 'revoked' })]), [], NOW)
    expect(c).toMatchObject({ show: true, tone: 'live', connected: 1, paired: 2 })
    expect(phoneChip(status([dev('a')]), [], NOW).tone).toBe('idle')
  })

  it('counts push only for phones that turned it on', () => {
    const c = phoneChip(status([dev('a', { push: 'fcm' }), dev('b', { push: 'web', pushOn: false }), dev('c', { push: 'none' }), dev('d')]), [], NOW)
    expect(c).toMatchObject({ pushOn: 1, pushReady: 2 })
    expect(pushMuted(c)).toBe(false)
    expect(pushMuted(phoneChip(status([dev('b', { push: 'web', pushOn: false })]), [], NOW))).toBe(true)
    expect(pushMuted(phoneChip(status([dev('c')]), [], NOW))).toBe(false)
  })

  it('alerts on automatic blocks from the last day that still hold', () => {
    const now = NOW / 1000
    const list = [block({ key: 'a', at: now - 60 }), block({ key: 'b', at: now - 90_000 }), block({ key: 'c', source: 'manual', at: now - 60 }), block({ key: 'd', at: now - 60, until: now - 1 }), block({ key: 'e', source: 'lan', at: now - 10, until: now + 600 })]
    expect(recentBlocks(list, now).map((b) => b.key)).toEqual(['a', 'e'])
    const c = phoneChip(status([dev('x', { state: 'connected' })]), list, NOW)
    expect(c).toMatchObject({ tone: 'alert', blocked: 2, connected: 1 })
  })
})
