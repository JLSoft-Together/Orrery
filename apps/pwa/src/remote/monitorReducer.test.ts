import { describe, expect, it } from 'vitest'
import { ACTIVITY_CAP, applyMonitorEvent, type MonitorSnapshot } from './monitorReducer'

const snap = (): MonitorSnapshot => ({ terminals: [{ id: 't1', status: 'idle', title: 'a' }], agents: [], usage: { buckets: [1], plan: 'x' } })

describe('applyMonitorEvent', () => {
  it('upserts, merges and removes terminals', () => {
    const s = snap()
    applyMonitorEvent(s, { type: 'terminal.created', payload: { id: 't2', status: 'working' } })
    applyMonitorEvent(s, { type: 'terminal.created', payload: { id: 't2', status: 'waiting' } })
    applyMonitorEvent(s, { type: 'terminal.updated', payload: { id: 't1', status: 'working' } })
    expect(s.terminals).toEqual([
      { id: 't1', status: 'working', title: 'a' },
      { id: 't2', status: 'waiting' },
    ])
    applyMonitorEvent(s, { type: 'terminal.removed', payload: { id: 't1' } })
    expect((s.terminals as unknown[]).length).toBe(1)
  })

  it('ignores updates for unknown ids and collections', () => {
    const s = snap()
    applyMonitorEvent(s, { type: 'terminal.updated', payload: { id: 'zz', status: 'x' } })
    applyMonitorEvent(s, { type: 'link.updated', payload: { id: 'zz' } })
    expect(s.terminals).toHaveLength(1)
    expect(s.peerLinks).toBeUndefined()
  })

  it('creates the collection on first create', () => {
    const s = snap()
    applyMonitorEvent(s, { type: 'link.created', payload: { id: 'l1' } })
    expect(s.peerLinks).toEqual([{ id: 'l1' }])
  })

  it('caps the activity list', () => {
    const s: MonitorSnapshot = { activity: [] }
    for (let i = 0; i < ACTIVITY_CAP + 25; i++) applyMonitorEvent(s, { type: 'activity', payload: { n: i } })
    const list = s.activity as Array<{ n: number }>
    expect(list).toHaveLength(ACTIVITY_CAP)
    expect(list[0]?.n).toBe(25)
  })

  it('applies whole-value and field events', () => {
    const s = snap()
    applyMonitorEvent(s, { type: 'usage.updated', payload: { buckets: [2, 3] } })
    applyMonitorEvent(s, { type: 'favorites.updated', payload: { favorites: ['f'] } })
    applyMonitorEvent(s, { type: 'limits.updated', payload: { a: 1 } })
    applyMonitorEvent(s, { type: 'sessions.loading', payload: { loading: true } })
    applyMonitorEvent(s, { type: 'usage.reset', payload: { buckets: [] } })
    expect(s.favorites).toEqual(['f'])
    expect(s.limits).toEqual({ a: 1 })
    expect(s.loading).toBe(true)
    expect(s.usage).toEqual({ buckets: [] })
  })

  it('ignores junk and prototype-ish event names', () => {
    const s = snap()
    applyMonitorEvent(s, null)
    applyMonitorEvent(s, { type: 5 })
    applyMonitorEvent(s, { type: 'constructor.created', payload: { id: 'x' } })
    applyMonitorEvent(s, { type: 'toString', payload: 1 })
    applyMonitorEvent(s, { type: 'terminal.created', payload: 'str' })
    expect(Object.keys(s).sort()).toEqual(['agents', 'terminals', 'usage'])
  })
})
