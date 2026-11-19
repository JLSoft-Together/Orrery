import { describe, expect, it } from 'vitest'
import type { RailCluster, RailRow, RailState } from '../rail/railModel'
import {
  attnKindOf,
  buildLiveRows,
  clampSnooze,
  collapsedProjects,
  compactElapsed,
  detailOf,
  freezeLayout,
  isSignalEdge,
  isSnoozedAt,
  layoutOf,
  needsLayout,
  needsOf,
  pruneSnoozed,
  snoozeChoices,
  snoozeUntil,
  sortNeeds,
  statusGroupOf,
  type LiveExtras,
  type LiveRow,
} from './railLive'

function live(key: string, state: RailState, since: number, over: Partial<LiveRow> = {}): LiveRow {
  return {
    key, paneKey: key, tabKey: `t-${key}`, clusterKey: 'c1', clusterName: 'App', tone: 0, agent: 'Claude', title: key, state,
    attn: attnKindOf(state, null), since, subagents: 0, model: null, ctxPct: null, tool: null, waitText: null, stateText: null,
    shell: false, focused: false, src: {} as RailRow, ...over,
  }
}

describe('taxonomy', () => {
  it('maps states to kinds', () => {
    expect(attnKindOf('asked', 'permission')).toBe('permission')
    expect(attnKindOf('asked', 'question')).toBe('question')
    expect(attnKindOf('asked', null)).toBe('question')
    expect(attnKindOf('failed', null)).toBe('error')
    expect(attnKindOf('done', null)).toBeNull()
    expect(attnKindOf('working', null)).toBeNull()
  })

  it('keeps all nine states reachable in groups', () => {
    const all: RailState[] = ['failed', 'asked', 'ended', 'working', 'subagents', 'done', 'starting', 'idle', 'parked']
    expect(all.map(statusGroupOf)).toEqual(['failed', null, 'idle', 'working', 'working', 'done', 'working', 'idle', 'idle'])
  })

  it('sorts permission and question before error, done never enters, oldest first', () => {
    const rows = [
      live('err-old', 'failed', 100),
      live('ask-new', 'asked', 900, { attn: 'question' }),
      live('perm-old', 'asked', 500, { attn: 'permission' }),
      live('done', 'done', 50),
      live('ask-none', 'asked', 0, { attn: 'question' }),
    ]
    expect(sortNeeds(rows).map((r) => r.key)).toEqual(['perm-old', 'ask-new', 'ask-none', 'err-old', 'done'])
    expect(needsOf(rows, () => false).map((r) => r.key)).toEqual(['perm-old', 'ask-new', 'ask-none', 'err-old'])
    expect(needsOf(rows, (k) => k === 'perm-old').map((r) => r.key)).not.toContain('perm-old')
  })
})

describe('snooze', () => {
  it('offers 5, 15, 60 and caps errors at 15', () => {
    expect(snoozeChoices('permission')).toEqual([5, 15, 60])
    expect(snoozeChoices('question')).toEqual([5, 15, 60])
    expect(snoozeChoices('error')).toEqual([5, 15])
    expect(clampSnooze('error', 60)).toBe(15)
    expect(clampSnooze('permission', 60)).toBe(60)
    expect(clampSnooze('permission', 500)).toBe(60)
  })

  it('expires at the deadline and prunes', () => {
    const until = snoozeUntil('error', 60, 1000)
    expect(until).toBe(1000 + 15 * 60_000)
    expect(isSnoozedAt({ a: until }, 'a', until - 1)).toBe(true)
    expect(isSnoozedAt({ a: until }, 'a', until)).toBe(false)
    expect(isSnoozedAt({}, 'a', 0)).toBe(false)
    expect(pruneSnoozed({ a: 5, b: 50, c: 60 }, 10, (k) => k !== 'c')).toEqual({ b: 50 })
  })
})

describe('freeze while pointer is in the rail', () => {
  const rows = [live('a', 'working', 3), live('b', 'asked', 2), live('c', 'idle', 1)]

  it('groups by state and recency', () => {
    const l = layoutOf(rows, 'status', ['c1'])
    expect(l.map((s) => s.id)).toEqual(['s:working', 's:idle'])
    expect(needsLayout(needsOf(rows, () => false))[0]!.keys).toEqual(['b'])
  })

  it('does not move rows between sections while held, adds new ones, drops gone ones', () => {
    const prev = layoutOf(rows, 'status', ['c1'])
    const after = [live('a', 'failed', 9), live('c', 'working', 8), live('d', 'idle', 7)]
    const next = layoutOf(after, 'status', ['c1'])
    const held = freezeLayout(prev, next, true)
    expect(held.find((s) => s.id === 's:working')!.keys).toEqual(['a'])
    expect(held.find((s) => s.id === 's:idle')!.keys).toEqual(['c', 'd'])
    expect(held.flatMap((s) => s.keys)).not.toContain('b')
  })

  it('applies the new order on release', () => {
    const prev = layoutOf(rows, 'status', ['c1'])
    const after = [live('a', 'failed', 9)]
    const next = layoutOf(after, 'status', ['c1'])
    expect(freezeLayout(prev, next, false)).toEqual(next)
    expect(freezeLayout(null, next, true)).toEqual(next)
  })
})

describe('rows and labels', () => {
  const row = (state: RailState, kind: RailRow['kind']): RailRow => ({ key: `k-${kind}`, paneKey: 'p', tabKey: 't', kind, agent: 'Claude', state, since: 5, subagents: 0, focused: false }) as RailRow
  const cluster = (rows: RailRow[], live = true): RailCluster => ({ key: 'c1', name: 'App', live, checkouts: [{ rows }] }) as unknown as RailCluster
  const extras: LiveExtras = { model: 'Opus 5', ctxPct: 40, tool: 'Bash', waitReason: 'permission', waitText: 'Cần duyệt', stateText: 'Đang chạy', title: 'T' }

  it('hides model for shell and run rows, keeps empty model empty', () => {
    const rows = buildLiveRows([cluster([row('working', 'agent'), row('idle', 'shell'), row('idle', 'other')])], () => extras, () => 3)
    expect(rows.map((r) => r.model)).toEqual(['Opus 5', null, null])
    expect(rows.every((r) => r.tone === 3)).toBe(true)
    expect(buildLiveRows([cluster([row('idle', 'agent')])], () => ({ ...extras, model: null }), () => 0)[0]!.model).toBeNull()
    expect(buildLiveRows([cluster([row('idle', 'agent')], false)], () => extras, () => 0)).toEqual([])
  })

  it('picks one detail', () => {
    expect(detailOf({ state: 'working', tool: 'Bash', waitText: null, stateText: 'Đang chạy' })).toBe('Bash')
    expect(detailOf({ state: 'working', tool: null, waitText: null, stateText: 'Đang chạy' })).toBe('Đang chạy')
    expect(detailOf({ state: 'asked', tool: null, waitText: 'Cần duyệt', stateText: 'Cần bạn' })).toBe('Cần duyệt')
  })

  it('formats elapsed time', () => {
    expect(compactElapsed(400)).toBe('')
    expect(compactElapsed(42_000)).toBe('42s')
    expect(compactElapsed(134_000)).toBe('2m 14s')
    expect(compactElapsed(25 * 60_000)).toBe('25m')
    expect(compactElapsed(65 * 60_000)).toBe('1h 05m')
    expect(compactElapsed(50 * 3600_000)).toBe('2d')
  })

  it('summarises collapsed projects', () => {
    const rows = [live('a', 'asked', 1, { attn: 'permission' }), live('b', 'failed', 2), live('c', 'working', 3), live('d', 'asked', 4, { attn: 'question', clusterKey: 'c2', clusterName: 'Two' })]
    const out = collapsedProjects(rows, ['c1', 'c2', 'c3'], (k) => k === 'a')
    expect(out.map((p) => [p.clusterKey, p.needs, p.failed])).toEqual([['c1', 0, 1], ['c2', 1, 0]])
  })

  it('signals only on a real change into asked or failed', () => {
    expect(isSignalEdge(undefined, 'asked')).toBe(false)
    expect(isSignalEdge('working', 'asked')).toBe(true)
    expect(isSignalEdge('asked', 'asked')).toBe(false)
    expect(isSignalEdge('working', 'failed')).toBe(true)
    expect(isSignalEdge('working', 'done')).toBe(false)
  })
})
