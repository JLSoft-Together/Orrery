import 'fake-indexeddb/auto'
import { IDBFactory } from 'fake-indexeddb'
import { beforeEach, describe, expect, it } from 'vitest'
import { fakeNoise, sampleMachine } from '../test/fakes'
import { Db } from './idb'
import { createKeyStore, type Unlocker } from './keyStore'
import { IdbSnapshotCache, MachineRepo } from './repos'

let db: Db

beforeEach(async () => {
  db = await Db.open(new IDBFactory(), `t-${Math.random()}`)
})

const noise = async () => fakeNoise

describe('key store', () => {
  it('creates an identity once and returns the same key and device id', async () => {
    const keys = createKeyStore({ db, noise })
    const a = await keys.load()
    const b = await keys.load()
    expect(a.deviceId).toMatch(/^pwa-[0-9a-f]{16}$/)
    expect(b.deviceId).toBe(a.deviceId)
    expect([...a.privateKey]).toEqual(new Array(32).fill(7))
    expect([...b.privateKey]).toEqual([...a.privateKey])
  })

  it('stores only ciphertext and a non-extractable wrapping key', async () => {
    const keys = createKeyStore({ db, noise })
    await keys.load()
    const sealed = await db.get<{ cipher: Uint8Array; iv: Uint8Array }>('kv', 'identity')
    expect(sealed?.cipher.length).toBe(32 + 16)
    expect(JSON.stringify([...(sealed?.cipher ?? [])])).not.toContain('7,7,7,7,7,7')
    const wrap = await db.get<CryptoKey>('kv', 'wrapKey')
    expect(wrap?.extractable).toBe(false)
    await expect(crypto.subtle.exportKey('raw', wrap as CryptoKey)).rejects.toThrow()
  })

  it('concurrent first loads create a single identity', async () => {
    const keys = createKeyStore({ db, noise })
    const [a, b] = await Promise.all([keys.load(), keys.load()])
    expect(a.deviceId).toBe(b.deviceId)
  })

  it('goes through the unlock hook for the decryption key', async () => {
    let unlocks = 0
    const inner = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt'])
    const unlocker: Unlocker = {
      unlock: async () => {
        unlocks++
        return inner
      },
    }
    const keys = createKeyStore({ db, noise, unlocker })
    await keys.load()
    await keys.load()
    expect(unlocks).toBeGreaterThanOrEqual(2)
  })

  it('reset removes the identity so a new one is made', async () => {
    const keys = createKeyStore({ db, noise })
    const a = await keys.load()
    await keys.reset()
    expect((await keys.load()).deviceId).not.toBe(a.deviceId)
  })
})

describe('machine repo and snapshot cache', () => {
  it('round-trips machines including binary keys', async () => {
    const repo = new MachineRepo(db)
    await repo.add(sampleMachine({ id: 'a', pairedAt: 1 }))
    await repo.add(sampleMachine({ id: 'b', pairedAt: 2 }))
    const fresh = new MachineRepo(db)
    const list = await fresh.init()
    expect(list.map((m) => m.id)).toEqual(['b', 'a'])
    expect(list[0]?.mkey).toBeInstanceOf(Uint8Array)
    expect(fresh.get('a')?.policy).toBe('auto')
  })

  it('forget deletes the machine and its snapshot and ignores late saves', async () => {
    const repo = new MachineRepo(db)
    const cache = new IdbSnapshotCache(db)
    await repo.add(sampleMachine({ id: 'a' }))
    await cache.save('a', { savedAt: 1, seq: 1, wsRev: 0, snapshot: { terminals: [] }, workspace: null, hello: null })
    await repo.forget('a')
    await cache.delete('a')
    await repo.save(sampleMachine({ id: 'a', name: 'late' }))
    await cache.save('a', { savedAt: 2, seq: 2, wsRev: 0, snapshot: null, workspace: null, hello: null })
    expect(await db.getAll('machines')).toEqual([])
    expect(await cache.load('a')).toBeNull()
  })

  it('re-adding a forgotten machine works again', async () => {
    const repo = new MachineRepo(db)
    const cache = new IdbSnapshotCache(db)
    await repo.forget('a')
    await cache.delete('a')
    await repo.add(sampleMachine({ id: 'a' }))
    cache.allow('a')
    await cache.save('a', { savedAt: 2, seq: 2, wsRev: 0, snapshot: null, workspace: null, hello: null })
    expect(repo.get('a')).toBeDefined()
    expect((await cache.load('a'))?.seq).toBe(2)
  })

  it('clearAll empties every store', async () => {
    await db.put('kv', 1, 'x')
    await new MachineRepo(db).add(sampleMachine())
    await new IdbSnapshotCache(db).save('m1', { savedAt: 1, seq: 1, wsRev: 0, snapshot: null, workspace: null, hello: null })
    await db.clearAll()
    expect(await db.getAll('kv')).toEqual([])
    expect(await db.getAll('machines')).toEqual([])
    expect(await db.getAll('snapshots')).toEqual([])
  })
})
