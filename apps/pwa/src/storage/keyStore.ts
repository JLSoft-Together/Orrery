import { randomHex } from '../remote/bytes'
import type { DeviceIdentity, KeyStore } from '../remote/connection'
import type { NoiseSuite } from '../remote/noise'
import type { Db } from './idb'

interface SealedIdentity {
  deviceId: string
  iv: Uint8Array
  cipher: Uint8Array
}

export interface Unlocker {
  unlock(): Promise<CryptoKey>
}

const WRAP_KEY = 'wrapKey'
const IDENTITY = 'identity'

export function createLocalUnlocker(db: Db): Unlocker {
  let pending: Promise<CryptoKey> | null = null
  return {
    unlock() {
      pending ??= (async () => {
        const stored = await db.get<CryptoKey>('kv', WRAP_KEY)
        if (stored) return stored
        const key = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt'])
        await db.put('kv', key, WRAP_KEY)
        return key
      })().catch((error) => {
        pending = null
        throw error
      })
      return pending
    },
  }
}

export interface KeyStoreOptions {
  db: Db
  noise: () => Promise<NoiseSuite>
  unlocker?: Unlocker
}

export function createKeyStore(options: KeyStoreOptions): KeyStore & { reset(): Promise<void> } {
  const unlocker = options.unlocker ?? createLocalUnlocker(options.db)
  let creating: Promise<void> | null = null

  async function create(): Promise<void> {
    const noise = await options.noise()
    const wrapKey = await unlocker.unlock()
    const privateKey = noise.generatePrivateKey()
    const iv = crypto.getRandomValues(new Uint8Array(12))
    const cipher = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, wrapKey, privateKey as Uint8Array<ArrayBuffer>))
    privateKey.fill(0)
    const sealed: SealedIdentity = { deviceId: `pwa-${randomHex(8)}`, iv, cipher }
    await options.db.put('kv', sealed, IDENTITY)
  }

  return {
    async load(): Promise<DeviceIdentity> {
      let sealed = await options.db.get<SealedIdentity>('kv', IDENTITY)
      if (!sealed) {
        creating ??= create().finally(() => {
          creating = null
        })
        await creating
        sealed = await options.db.get<SealedIdentity>('kv', IDENTITY)
      }
      if (!sealed) throw new Error('identity missing')
      const wrapKey = await unlocker.unlock()
      const privateKey = new Uint8Array(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: sealed.iv as Uint8Array<ArrayBuffer> }, wrapKey, sealed.cipher as Uint8Array<ArrayBuffer>))
      return { privateKey, deviceId: sealed.deviceId }
    },
    async reset(): Promise<void> {
      await options.db.delete('kv', IDENTITY)
      await options.db.delete('kv', WRAP_KEY)
    },
  }
}
