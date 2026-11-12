import 'fake-indexeddb/auto'
import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Db } from '../storage/idb'
import { IdbSnapshotCache, MachineRepo } from '../storage/repos'
import { attachDesktop, createNetwork, fakeNoise, IDENTITY, sampleMachine, testConfig, type FakeNetwork, type FakeSocket } from '../test/fakes'
import { RemoteHub, type HubEvent } from './hub'

let db: Db
let net: FakeNetwork
let sockets: FakeSocket[]
let events: HubEvent[]

function makeHub(): RemoteHub {
  const hub = new RemoteHub({
    connection: { noise: fakeNoise, connector: net.connector, keys: { load: async () => IDENTITY }, config: testConfig(), now: () => Date.now() },
    registry: new MachineRepo(db),
    cache: new IdbSnapshotCache(db),
  })
  hub.events.on((e) => events.push(e))
  return hub
}

function desktop(): void {
  net.routes.set('wss://relay.example/p/m1', (socket) => {
    sockets.push(socket)
    attachDesktop(socket, {
      onJson: (message, s) => {
        if (message.t !== 'hello') return false
        s.serverJson({ re: message.id, ok: true, machine: 'm1', name: 'Desk', version: '1.2.3', minProto: 1 })
        s.serverJson({ t: 'snapshot', seq: 1, data: { terminals: [{ id: 't1', status: 'working' }] } })
        s.serverJson({ t: 'ws.snapshot', rev: 1, tabs: [], closedCount: 0 })
        return true
      },
    })
  })
}

const tick = (ms: number) => vi.advanceTimersByTimeAsync(ms)

beforeEach(async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
  db = await Db.open(new IDBFactory(), `hub-${Math.random()}`)
  net = createNetwork()
  sockets = []
  events = []
  await new MachineRepo(db).add(sampleMachine())
})

afterEach(() => {
  vi.useRealTimers()
})

describe('RemoteHub', () => {
  it('connects, receives snapshots and saves them to IndexedDB', async () => {
    desktop()
    const hub = makeHub()
    await hub.init()
    await tick(10)
    expect(hub.link('m1').status).toBe('ready')
    expect(hub.view('m1').snapshot?.terminals).toHaveLength(1)
    expect(hub.view('m1').hello?.version).toBe('1.2.3')
    await tick(2100)
    expect((await db.get<{ seq: number }>('snapshots', 'm1'))?.seq).toBe(1)
    hub.dispose()
  })

  it('shows the cached snapshot as stale right after opening, before any connection', async () => {
    desktop()
    const first = makeHub()
    await first.init()
    await tick(2100)
    first.dispose()
    net.routes.clear()
    const second = makeHub()
    await second.init()
    expect(second.view('m1').fromCache).toBe(true)
    expect(second.view('m1').stale).toBe(true)
    expect(second.view('m1').snapshot?.terminals).toHaveLength(1)
    second.dispose()
  })

  it('wipes everything about a machine when the desktop revokes it (close 4401)', async () => {
    desktop()
    const hub = makeHub()
    await hub.init()
    await tick(2100)
    expect(await db.get('snapshots', 'm1')).toBeDefined()
    sockets[0]?.serverClose(4401)
    await vi.waitFor(async () => {
      expect(hub.machines()).toEqual([])
      expect(await db.getAll('machines')).toEqual([])
      expect(await db.get('snapshots', 'm1')).toBeUndefined()
    })
    expect(events.find((e) => e.kind === 'revoked')).toMatchObject({ machineId: 'm1', name: 'Desk' })
    await tick(60_000)
    expect(sockets).toHaveLength(1)
    expect(await db.getAll('machines')).toEqual([])
    expect(await db.get('snapshots', 'm1')).toBeUndefined()
  })

  it('a pending debounced save cannot bring a revoked machine back', async () => {
    desktop()
    const hub = makeHub()
    await hub.init()
    await tick(10)
    sockets[0]?.serverClose(4401)
    await tick(2500)
    await vi.waitFor(async () => {
      expect(await db.get('snapshots', 'm1')).toBeUndefined()
      expect(await db.getAll('machines')).toEqual([])
    })
  })

  it('changing the route policy persists it and reconnects', async () => {
    desktop()
    const hub = makeHub()
    await hub.init()
    await tick(10)
    await hub.setPolicy('m1', 'lan')
    await tick(10)
    expect(hub.machines()[0]?.policy).toBe('lan')
    expect((await db.getAll<{ policy: string }>('machines'))[0]?.policy).toBe('lan')
    expect(hub.link('m1').status).toBe('offline')
    hub.dispose()
  })

  it('keeps a bounded connection log', async () => {
    desktop()
    const hub = makeHub()
    await hub.init()
    for (let i = 0; i < 300; i++) hub.note(`n${i}`)
    expect(hub.log.length).toBe(200)
    hub.dispose()
  })
})
