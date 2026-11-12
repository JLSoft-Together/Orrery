import { describe, expect, it } from 'vitest'
import { parentOf, parsePath, toPath, type Route } from './router'
import { fromBase64Url, isSupported, registerCredential, toBase64Url, verifyCredential, type AuthDeps, type CredentialsLike } from './security/webauthn'
import { DEFAULT_LOCAL_PREFS, loadLocalPrefs, normalizeLocalPrefs, saveLocalPrefs } from './storage/localPrefs'
import { bellOn, DEFAULT_NOTIFY, normalizeNotify } from './storage/notifyPrefs'

describe('router paths', () => {
  const routes: Route[] = [
    { name: 'home' },
    { name: 'pair' },
    { name: 'settings' },
    { name: 'debug' },
    { name: 'monitor', machine: 'm-1' },
    { name: 'session', machine: 'm-1', terminal: 't 2' },
    { name: 'workspace', machine: 'm-1' },
    { name: 'workspace', machine: 'm-1', pane: 'pane/9' },
  ]

  it('round-trips every route, including encoded ids', () => {
    for (const route of routes) expect(parsePath(toPath(route))).toEqual(route)
  })

  it('parses the deep link used by push and bounce', () => {
    expect(parsePath('/m/abc/p/p1')).toEqual({ name: 'workspace', machine: 'abc', pane: 'p1' })
  })

  it('treats unknown or malformed paths as not found', () => {
    for (const path of ['/x', '/m', '/m/a/s', '/m/a/s/b/c', '/pair/extra', '/m/a/zzz']) expect(parsePath(path).name).toBe('notfound')
    expect(parsePath('').name).toBe('home')
    expect(parsePath('/?q=1').name).toBe('home')
  })

  it('goes up one level', () => {
    expect(parentOf({ name: 'session', machine: 'a', terminal: 'b' })).toEqual({ name: 'monitor', machine: 'a' })
    expect(parentOf({ name: 'debug' })).toEqual({ name: 'settings' })
    expect(parentOf({ name: 'monitor', machine: 'a' })).toEqual({ name: 'home' })
  })
})

function fakeDeps(options: { available?: boolean; fail?: string; none?: boolean } = {}) {
  const calls: Array<{ kind: 'create' | 'get'; options: CredentialCreationOptions | CredentialRequestOptions }> = []
  const raw = new Uint8Array([1, 2, 3, 250, 251, 252])
  const credentials: CredentialsLike = {
    async create(o) {
      calls.push({ kind: 'create', options: o })
      if (options.fail) throw Object.assign(new Error('x'), { name: options.fail })
      return options.none ? null : ({ rawId: raw.buffer } as unknown as Credential)
    },
    async get(o) {
      calls.push({ kind: 'get', options: o })
      if (options.fail) throw Object.assign(new Error('x'), { name: options.fail })
      return options.none ? null : ({} as Credential)
    },
  }
  const deps: AuthDeps = { credentials, platformAvailable: async () => options.available ?? true, random: (n) => new Uint8Array(n).fill(9) }
  return { deps, calls, raw }
}

describe('webauthn app lock', () => {
  it('registers a platform credential that requires user verification', async () => {
    const { deps, calls, raw } = fakeDeps()
    const result = await registerCredential(deps)
    expect(result).toEqual({ outcome: 'ok', id: toBase64Url(raw) })
    const publicKey = (calls[0]!.options as CredentialCreationOptions).publicKey!
    expect(publicKey.authenticatorSelection).toEqual({ authenticatorAttachment: 'platform', userVerification: 'required', residentKey: 'preferred' })
    expect(publicKey.attestation).toBe('none')
  })

  it('unlocks with the stored credential id and requires verification', async () => {
    const { deps, calls, raw } = fakeDeps()
    expect(await verifyCredential(toBase64Url(raw), deps)).toBe('ok')
    const publicKey = (calls[0]!.options as CredentialRequestOptions).publicKey!
    expect(publicKey.userVerification).toBe('required')
    expect([...new Uint8Array(publicKey.allowCredentials![0]!.id as ArrayBuffer)]).toEqual([...raw])
  })

  it('maps cancel, failure, missing credential and unsupported devices', async () => {
    expect(await verifyCredential('AQID', fakeDeps({ fail: 'NotAllowedError' }).deps)).toBe('cancelled')
    expect(await verifyCredential('AQID', fakeDeps({ fail: 'SecurityError' }).deps)).toBe('failed')
    expect(await verifyCredential('AQID', fakeDeps({ none: true }).deps)).toBe('failed')
    expect((await registerCredential(fakeDeps({ available: false }).deps)).outcome).toBe('unsupported')
    expect(await isSupported({ ...fakeDeps().deps, credentials: null })).toBe(false)
    expect(await verifyCredential('AQID', { ...fakeDeps().deps, credentials: null })).toBe('unsupported')
  })

  it('round-trips base64url', () => {
    const bytes = new Uint8Array([251, 255, 0, 62, 63])
    expect([...fromBase64Url(toBase64Url(bytes))]).toEqual([...bytes])
  })
})

describe('local prefs', () => {
  it('normalizes garbage to defaults and never enables the lock without a credential', () => {
    expect(normalizeLocalPrefs(null)).toEqual(DEFAULT_LOCAL_PREFS)
    expect(normalizeLocalPrefs({ language: 'fr', theme: 'x', lockEnabled: true, lockTimeout: 5 })).toEqual(DEFAULT_LOCAL_PREFS)
    expect(normalizeLocalPrefs({ lockEnabled: true, credentialId: 'abc', lockTimeout: 900_000 })).toMatchObject({ lockEnabled: true, lockTimeout: 900_000 })
  })

  it('saves and loads through a storage object', () => {
    const map = new Map<string, string>()
    const storage = { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => void map.set(k, v) }
    saveLocalPrefs({ ...DEFAULT_LOCAL_PREFS, monitorMode: 'groups', theme: 'dark' }, storage)
    expect(loadLocalPrefs(storage)).toMatchObject({ monitorMode: 'groups', theme: 'dark' })
    expect(loadLocalPrefs({ getItem: () => '{broken' })).toEqual(DEFAULT_LOCAL_PREFS)
  })

  it('keeps notify choices and per machine bells, bell on by default', () => {
    expect(normalizeNotify(undefined)).toEqual(DEFAULT_NOTIFY)
    const prefs = normalizeNotify({ waiting: false, content: true, bells: { a: false, b: 'x' } })
    expect(prefs).toMatchObject({ waiting: false, done: true, error: false, content: true, bells: { a: false } })
    expect(bellOn(prefs, 'a')).toBe(false)
    expect(bellOn(prefs, 'zzz')).toBe(true)
  })
})
