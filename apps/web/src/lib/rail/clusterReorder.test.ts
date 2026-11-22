import { describe, expect, it } from 'vitest'
import { freezeLayout, layoutOf } from '../next/railLive'
import { dropIndex, insideBox, lineEdge, reorderSlot, sectionRange } from './clusterReorder'

const boxes = [
  { top: 0, bottom: 40 },
  { top: 40, bottom: 80 },
  { top: 80, bottom: 120 },
]

describe('reorderSlot', () => {
  it('picks the slot above the first box whose middle is below the pointer', () => {
    expect(reorderSlot(boxes, 5)).toBe(0)
    expect(reorderSlot(boxes, 25)).toBe(1)
    expect(reorderSlot(boxes, 70)).toBe(2)
    expect(reorderSlot(boxes, 500)).toBe(3)
  })

  it('clamps to the section range', () => {
    expect(reorderSlot(boxes, 5, [1, 3])).toBe(1)
    expect(reorderSlot(boxes, 500, [0, 2])).toBe(2)
  })
})

describe('dropIndex', () => {
  it('maps a slot to the target index of the dragged cluster', () => {
    expect(dropIndex(2, 0)).toBe(0)
    expect(dropIndex(0, 3)).toBe(2)
    expect(dropIndex(0, 2)).toBe(1)
  })

  it('returns null when the drop does not move anything or is outside the list', () => {
    expect(dropIndex(1, 1)).toBeNull()
    expect(dropIndex(1, 2)).toBeNull()
    expect(dropIndex(1, -1)).toBeNull()
  })
})

describe('sectionRange', () => {
  it('spans the run of the dragged cluster section', () => {
    const s = [0, 0, 1, 1, 1, 2]
    expect(sectionRange(s, 0)).toEqual([0, 2])
    expect(sectionRange(s, 3)).toEqual([2, 5])
    expect(sectionRange(s, 5)).toEqual([5, 6])
    expect(sectionRange(s, 9)).toBeNull()
  })
})

describe('lineEdge and insideBox', () => {
  it('puts the line above the slot box or below the last one', () => {
    expect(lineEdge(boxes, 1)).toBe(37)
    expect(lineEdge(boxes, 3)).toBe(123)
    expect(lineEdge([], 0)).toBeNull()
  })

  it('tells a drop in the list from a drop outside it', () => {
    const list = { left: 0, right: 260, top: 0, bottom: 600 }
    expect(insideBox(list, 100, 300)).toBe(true)
    expect(insideBox(list, 400, 300)).toBe(false)
  })
})

describe('UI next rail layout after a reorder', () => {
  const rows = [
    { key: 'a1', clusterKey: 'a' },
    { key: 'b1', clusterKey: 'b' },
  ] as unknown as Parameters<typeof layoutOf>[0]

  it('a held layout keeps the old project order, a forced sync shows the new one', () => {
    const before = freezeLayout(null, layoutOf(rows, 'project', ['a', 'b']), false)
    const fresh = layoutOf(rows, 'project', ['b', 'a'])
    expect(freezeLayout(before, fresh, true).map((s) => s.id)).toEqual(['p:a', 'p:b'])
    expect(freezeLayout(null, fresh, true).map((s) => s.id)).toEqual(['p:b', 'p:a'])
  })
})
