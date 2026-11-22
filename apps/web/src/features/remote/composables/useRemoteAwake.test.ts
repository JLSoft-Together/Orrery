import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import type { RemoteDevice, RemoteStatus } from '@ccm/shared'
import type { AwakeLocks } from './useRemoteAwake'

const device = (id: string, state: RemoteDevice['state']): RemoteDevice => ({ deviceId: id, name: 'Pixel', platform: 'android', pairedAt: 1, lastSeen: 1, state })
const status = (devices: RemoteDevice[]): RemoteStatus => ({ enabled: true, listening: true, addresses: [], port: 4320, relay: 'off', keepAwake: true, devices })

function fakeLocks() {
  const held: { name: string; done: boolean }[] = []
  const locks: AwakeLocks = {
    request: (name, callback) => {
      const entry = { name, done: false }
      held.push(entry)
      return callback().then(() => {
        entry.done = true
      })
    },
  }
  return { locks, live: () => held.filter((h) => !h.done), all: held }
}

describe('remote awake lock', () => {
  beforeEach(() => {
    vi.resetModules()
    setActivePinia(createPinia())
  })

  async function boot(locks: AwakeLocks | undefined) {
    const mod = await import('./useRemoteAwake')
    const store = await import('../stores/remote')
    mod.installRemoteAwake(locks)
    return { remote: store.useRemoteStore(), name: mod.REMOTE_AWAKE_LOCK }
  }

  it('holds one lock only while a phone is connected', async () => {
    const f = fakeLocks()
    const { remote, name } = await boot(f.locks)
    await nextTick()
    expect(f.all).toHaveLength(0)
    remote.status = status([device('d1', 'idle')])
    await nextTick()
    expect(f.all).toHaveLength(0)
    remote.status = status([device('d1', 'connected')])
    await nextTick()
    expect(f.live().map((h) => h.name)).toEqual([name])
    remote.status = status([device('d1', 'connected'), device('d2', 'connected')])
    await nextTick()
    expect(f.all).toHaveLength(1)
    remote.status = status([device('d1', 'idle'), device('d2', 'idle')])
    await nextTick()
    await Promise.resolve()
    expect(f.live()).toHaveLength(0)
    remote.status = status([device('d2', 'connected')])
    await nextTick()
    expect(f.all).toHaveLength(2)
    expect(f.live()).toHaveLength(1)
  })

  it('does nothing without the Web Locks API', async () => {
    const { remote } = await boot(undefined)
    remote.status = status([device('d1', 'connected')])
    await nextTick()
    expect(remote.connected).toBe(1)
  })
})
