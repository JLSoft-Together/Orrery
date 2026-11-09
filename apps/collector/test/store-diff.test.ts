import { describe, expect, it } from 'vitest'
import type { Agent, ServerMessage } from '@ccm/shared'
import { MonitorStore } from '../src/store'

const agent = (over: Partial<Agent> = {}): Agent =>
  ({ id: 'a1', terminalId: 't1', role: 'main', name: 'x', type: 'main', status: 'working', ...over }) as Agent

function setup() {
  const store = new MonitorStore({ activityLimit: 10, batchMs: 1000 })
  const sent: ServerMessage[] = []
  store.subscribe((m) => sent.push(m))
  const events = () => {
    store.flush()
    return sent.flatMap((m) => m.events)
  }
  return { store, events }
}

describe('store agent diff and terminal index', () => {
  it('emits nothing for an identical upsert, even with undefined fields', () => {
    const { store, events } = setup()
    store.upsertAgent(agent({ model: undefined }))
    store.flush()
    const before = events().length
    store.upsertAgent(agent({ model: undefined }))
    expect(events().length).toBe(before)
  })

  it('patches only changed fields and nulls fields that became undefined', () => {
    const { store, events } = setup()
    store.upsertAgent(agent({ model: 'm' }))
    store.flush()
    store.upsertAgent(agent({ model: undefined, status: 'idle' }))
    const last = events().at(-1)
    expect(last).toEqual({ type: 'agent.updated', payload: { id: 'a1', status: 'idle', model: null } })
    expect('model' in (store.getAgent('a1') as object)).toBe(false)
  })

  it('agentsOf follows upsert, terminal move and removal', () => {
    const { store } = setup()
    store.upsertAgent(agent())
    store.upsertAgent(agent({ id: 'a2' }))
    store.upsertAgent(agent({ id: 'a3', terminalId: 't2' }))
    expect(store.agentsOf('t1').map((a) => a.id)).toEqual(['a1', 'a2'])
    store.upsertAgent(agent({ id: 'a2', terminalId: 't2' }))
    expect(store.agentsOf('t1').map((a) => a.id)).toEqual(['a1'])
    expect(store.agentsOf('t2').map((a) => a.id)).toEqual(['a3', 'a2'])
    store.removeAgent('a1')
    expect(store.agentsOf('t1')).toEqual([])
  })
})
