import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MachineData, type CacheEntry, type SnapshotCache } from './machineData'

class MemoryCache implements SnapshotCache {
  entries = new Map<string, CacheEntry>()
  async load(id: string) {
    return this.entries.get(id) ?? null
  }
  async save(id: string, entry: CacheEntry) {
    this.entries.set(id, entry)
  }
  async delete(id: string) {
    this.entries.delete(id)
  }
}

let calls: string[]
let data: MachineData
let cache: MemoryCache

beforeEach(() => {
  calls = []
  cache = new MemoryCache()
  data = new MachineData({
    machineId: 'm1',
    cache,
    now: () => 5000,
    commander: async (type) => {
      calls.push(type)
      return {}
    },
  })
})

const snapshot = (seq = 1) => ({ t: 'snapshot', seq, data: { terminals: [{ id: 't1', status: 'idle' }] } })

describe('MachineData monitor', () => {
  it('takes a snapshot and marks it fresh', () => {
    data.handleEvent(snapshot())
    expect(data.view.stale).toBe(false)
    expect(data.view.seq).toBe(1)
    expect(data.view.updatedAt).toBe(5000)
  })

  it('applies the next seq and ignores old ones', () => {
    data.handleEvent(snapshot(1))
    data.handleEvent({ t: 'events', seq: 2, events: [{ type: 'terminal.updated', payload: { id: 't1', status: 'working' } }] })
    expect((data.view.snapshot?.terminals as Array<{ status: string }>)[0]?.status).toBe('working')
    data.handleEvent({ t: 'events', seq: 2, events: [{ type: 'terminal.removed', payload: { id: 't1' } }] })
    expect(data.view.snapshot?.terminals).toHaveLength(1)
    expect(calls).toEqual([])
  })

  it('asks for a resync once on a seq gap', () => {
    data.handleEvent(snapshot(1))
    data.handleEvent({ t: 'events', seq: 5, events: [] })
    data.handleEvent({ t: 'events', seq: 6, events: [] })
    expect(calls).toEqual(['resync'])
    data.handleEvent(snapshot(6))
    data.handleEvent({ t: 'events', seq: 9, events: [] })
    expect(calls).toEqual(['resync', 'resync'])
  })

  it('does not mutate the previous snapshot object identity', () => {
    data.handleEvent(snapshot(1))
    const before = data.view.snapshot
    data.handleEvent({ t: 'events', seq: 2, events: [{ type: 'agent.created', payload: { id: 'a1' } }] })
    expect(data.view.snapshot).not.toBe(before)
  })

  it('marks the view stale when the link drops', () => {
    data.handleEvent(snapshot())
    data.onLink({ status: 'reconnecting', reason: 'none', attempt: 1 })
    expect(data.view.stale).toBe(true)
  })
})

describe('MachineData workspace', () => {
  const ws = { t: 'ws.snapshot', rev: 3, tabs: [{ key: 'k', panes: [{ key: 'p', title: 'one' }] }], closedCount: 0 }

  it('stores the snapshot without the event type', () => {
    data.handleEvent(ws)
    expect(data.view.wsRev).toBe(3)
    expect(data.view.workspace?.t).toBeUndefined()
    expect(data.view.workspace?.closedCount).toBe(0)
  })

  it('applies a patch whose base matches and keeps rev', () => {
    data.handleEvent(ws)
    data.handleEvent({ t: 'ws.patch', base: 3, rev: 4, ops: [{ op: 'replace', path: '/tabs/0/panes/0/title', value: 'two' }] })
    expect(data.view.wsRev).toBe(4)
    expect(data.view.workspace?.rev).toBe(4)
    expect((data.view.workspace?.tabs as Array<{ panes: Array<{ title: string }> }>)[0]?.panes[0]?.title).toBe('two')
    expect(calls).toEqual([])
  })

  it('refetches when base differs, the patch is broken, or there is no snapshot', () => {
    data.handleEvent({ t: 'ws.patch', base: 1, rev: 2, ops: [] })
    expect(calls).toEqual(['ws.get'])
    data.handleEvent(ws)
    data.handleEvent({ t: 'ws.patch', base: 9, rev: 10, ops: [] })
    data.handleEvent(ws)
    data.handleEvent({ t: 'ws.patch', base: 3, rev: 4, ops: [{ op: 'remove', path: '/nope' }] })
    expect(calls).toEqual(['ws.get', 'ws.get', 'ws.get'])
    expect(data.view.wsRev).toBe(3)
  })

  it('ignores stale patches', () => {
    data.handleEvent(ws)
    data.handleEvent({ t: 'ws.patch', base: 2, rev: 3, ops: [{ op: 'remove', path: '/tabs/0' }] })
    expect(data.view.workspace?.tabs).toHaveLength(1)
    expect(calls).toEqual([])
  })
})

describe('MachineData cache and confirms', () => {
  it('saves after the debounce and restores as stale from cache', async () => {
    vi.useFakeTimers()
    data.handleEvent(snapshot(4))
    await vi.advanceTimersByTimeAsync(2100)
    vi.useRealTimers()
    expect(cache.entries.get('m1')?.seq).toBe(4)
    const fresh = new MachineData({ machineId: 'm1', cache, commander: async () => ({}) })
    await fresh.restoreFromCache()
    expect(fresh.view.stale).toBe(true)
    expect(fresh.view.fromCache).toBe(true)
    expect(fresh.view.snapshot?.terminals).toHaveLength(1)
    fresh.handleEvent({ t: 'events', seq: 5, events: [] })
    expect(fresh.view.fromCache).toBe(true)
  })

  it('a live snapshot replaces the cached one', async () => {
    cache.entries.set('m1', { savedAt: 1, seq: 1, wsRev: 0, snapshot: { terminals: [] }, workspace: null, hello: null })
    await data.restoreFromCache()
    expect(data.view.fromCache).toBe(true)
    data.handleEvent(snapshot(2))
    expect(data.view.fromCache).toBe(false)
    expect(data.view.stale).toBe(false)
  })

  it('tracks confirm asks and answers them', async () => {
    data.handleEvent({ t: 'confirm.ask', cid: 'c1', title: 'Delete?', ok: 'Yes', cancel: 'No' })
    expect(data.view.confirms.map((c) => c.cid)).toEqual(['c1'])
    await data.answerConfirm('c1', true)
    expect(data.view.confirms).toEqual([])
    expect(calls).toEqual(['confirm.answer'])
  })
})
