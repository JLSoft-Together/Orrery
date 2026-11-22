import { describe, expect, it } from 'vitest'
import { badgeCount, hasFresh, needIds, shouldFlash } from './badge'
import { GROUP_WINDOW_MS, groupCounts, groupLead, groupNames, groupTag, isMerged, joinGroup, type GroupState } from './grouping'

const m = (id: string, waiting = false) => ({ id, name: id, waiting })

describe('notification grouping', () => {
  it('keeps one event single and merges those inside the 8 second window', () => {
    const a = joinGroup(null, 1000, m('a'))
    expect(isMerged(a)).toBe(false)
    const b = joinGroup(a, 1000 + GROUP_WINDOW_MS, m('b'))
    expect(isMerged(b)).toBe(true)
    expect(b.startedAt).toBe(1000)
    expect(groupTag(b)).toBe(groupTag(a))
  })

  it('starts a new group after the window', () => {
    const a = joinGroup(null, 0, m('a'))
    const b = joinGroup(a, GROUP_WINDOW_MS + 1, m('b'))
    expect(b.members.map((x) => x.id)).toEqual(['b'])
    expect(groupTag(b)).not.toBe(groupTag(a))
  })

  it('counts a session once and lets a waiting one lead', () => {
    let g: GroupState | null = null
    g = joinGroup(g, 0, m('a'))
    g = joinGroup(g, 10, m('b'))
    g = joinGroup(g, 20, m('c', true))
    g = joinGroup(g, 30, m('a', true))
    expect(groupCounts(g)).toEqual({ done: 1, waiting: 2 })
    expect(groupLead(g)?.id).toBe('c')
  })

  it('lists up to 4 names and counts the rest', () => {
    let g: GroupState | null = null
    for (const id of ['a', 'b', 'c', 'd', 'e', 'f']) g = joinGroup(g, 0, m(id))
    expect(groupNames(g as GroupState)).toEqual({ shown: 'a, b, c, d', more: 2 })
  })
})

describe('taskbar badge', () => {
  const items = [
    { id: '1', reason: 'menu' as const },
    { id: '2', reason: 'permission' as const },
    { id: '3', reason: 'question' as const },
    { id: '4', reason: 'done' as const },
    { id: '5', reason: 'failed' as const },
    { id: '6', reason: 'clash' as const },
  ]

  it('counts only the groups that wait for an answer', () => {
    expect(needIds(items)).toEqual(['1', '2', '3'])
    expect(badgeCount(items)).toBe(3)
    expect(badgeCount([])).toBe(0)
  })

  it('detects an item that was not there before', () => {
    expect(hasFresh(new Set(['1', '2']), ['1', '2', '3'])).toBe(true)
    expect(hasFresh(new Set(['1', '2', '3']), ['1', '2'])).toBe(false)
  })

  it('flashes only for a new item, in an unfocused window, outside quiet hours, with the switch on', () => {
    const base = { enabled: true, fresh: true, focused: false, quiet: false }
    expect(shouldFlash(base)).toBe(true)
    expect(shouldFlash({ ...base, enabled: false })).toBe(false)
    expect(shouldFlash({ ...base, fresh: false })).toBe(false)
    expect(shouldFlash({ ...base, focused: true })).toBe(false)
    expect(shouldFlash({ ...base, quiet: true })).toBe(false)
  })
})
