import { describe, expect, it } from 'vitest'
import { DEFAULT_LOCAL_PREFS, loadLocalPrefs, normalizeLocalPrefs, saveLocalPrefs } from '../storage/localPrefs'
import { lockKind, shouldOfferLock } from './lockOffer'

const base = { machineCount: 1, lockEnabled: false, offered: false, supported: true }

describe('lock offer after pairing', () => {
  it('shows only for the first machine, with the lock off, never asked, on a supported device', () => {
    expect(shouldOfferLock(base)).toBe(true)
    expect(shouldOfferLock({ ...base, machineCount: 2 })).toBe(false)
    expect(shouldOfferLock({ ...base, lockEnabled: true })).toBe(false)
    expect(shouldOfferLock({ ...base, offered: true })).toBe(false)
    expect(shouldOfferLock({ ...base, supported: false })).toBe(false)
  })

  it('remembers that it asked, through storage', () => {
    expect(DEFAULT_LOCAL_PREFS.lockOffered).toBe(false)
    expect(normalizeLocalPrefs({ lockOffered: 'yes' }).lockOffered).toBe(false)
    const map = new Map<string, string>()
    const storage = { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => void map.set(k, v) }
    saveLocalPrefs({ ...DEFAULT_LOCAL_PREFS, lockOffered: true }, storage)
    const loaded = loadLocalPrefs(storage)
    expect(shouldOfferLock({ ...base, offered: loaded.lockOffered })).toBe(false)
  })

  it('names the lock by platform', () => {
    expect(lockKind('Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)')).toBe('faceId')
    expect(lockKind('Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)')).toBe('faceId')
    expect(lockKind('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', 5)).toBe('faceId')
    expect(lockKind('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', 0)).toBe('touchId')
    expect(lockKind('Mozilla/5.0 (Linux; Android 15)')).toBe('screen')
  })
})
