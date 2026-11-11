import { describe, expect, it } from 'vitest'
import { clampSlot, fitTabs, groupKeys, groupStarts, withinGroup } from './tabGroups'

const g = (k: string): string => k[0]!

describe('groupKeys', () => {
  it('keeps groups together in first-seen order and each group stable', () => {
    expect(groupKeys(['a1', 'b1', 'a2', 'c1', 'b2'], g)).toEqual(['a1', 'a2', 'b1', 'b2', 'c1'])
  })
  it('leaves an already grouped list alone', () => {
    expect(groupKeys(['a1', 'a2', 'b1'], g)).toEqual(['a1', 'a2', 'b1'])
  })
})

describe('groupStarts', () => {
  it('marks the first tab of every group but the first', () => {
    expect([...groupStarts(['a1', 'a2', 'b1', 'c1', 'c2'], g)]).toEqual(['b1', 'c1'])
  })
  it('is empty for one group', () => {
    expect(groupStarts(['a1', 'a2'], g).size).toBe(0)
  })
})

describe('clampSlot', () => {
  const order = ['a1', 'a2', 'b1', 'b2', 'b3', 'c1']
  it('keeps a slot inside the own group', () => {
    expect(clampSlot(order, g, 'b1', 3)).toBe(3)
  })
  it('snaps a slot before the group to the group start', () => {
    expect(clampSlot(order, g, 'b2', 0)).toBe(2)
  })
  it('snaps a slot past the group to the group end', () => {
    expect(clampSlot(order, g, 'b1', 6)).toBe(5)
  })
})

describe('withinGroup', () => {
  const order = ['a1', 'a2', 'b1', 'b2', 'b3', 'c1']
  it('returns the group list and a slot relative to it', () => {
    expect(withinGroup(order, g, 'b1', 5)).toEqual({ list: ['b1', 'b2', 'b3'], slot: 3 })
    expect(withinGroup(order, g, 'b3', 0)).toEqual({ list: ['b1', 'b2', 'b3'], slot: 0 })
    expect(withinGroup(order, g, 'b3', 3)).toEqual({ list: ['b1', 'b2', 'b3'], slot: 1 })
  })
})

describe('fitTabs', () => {
  const keys = ['a', 'b', 'c', 'd', 'e']
  const widths = new Map(keys.map((k) => [k, 100] as const))
  it('shows all when they fit', () => {
    expect(fitTabs({ keys, widths, active: 'a', avail: 600 })).toEqual({ shown: keys, hidden: [] })
  })
  it('collapses the tail and keeps order', () => {
    const r = fitTabs({ keys, widths, active: 'a', avail: 360, moreWidth: 40 })
    expect(r.shown).toEqual(['a', 'b', 'c'])
    expect(r.hidden).toEqual(['d', 'e'])
  })
  it('always keeps the active tab even when it is last', () => {
    const r = fitTabs({ keys, widths, active: 'e', avail: 360, moreWidth: 40 })
    expect(r.shown).toEqual(['a', 'b', 'e'])
    expect(r.hidden).toEqual(['c', 'd'])
  })
  it('uses the fallback width for tabs never measured', () => {
    const r = fitTabs({ keys, widths: new Map(), active: 'a', avail: 400, moreWidth: 40, fallback: 120 })
    expect(r.shown.length).toBe(2)
  })
  it('does nothing before the strip is measured', () => {
    expect(fitTabs({ keys, widths, active: 'a', avail: 0 }).hidden).toEqual([])
  })
})
