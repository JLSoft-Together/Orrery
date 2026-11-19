import { describe, expect, it } from 'vitest'
import { arcAt, layoutArcs, layoutRings, TAU } from './sunburst'

describe('layoutArcs', () => {
  it('returns nothing for empty or zero input', () => {
    expect(layoutArcs([], 0, TAU, 0)).toEqual([])
    expect(layoutArcs([0, 0], 0, TAU, 0)).toEqual([])
  })

  it('makes one item span the full circle', () => {
    const a = layoutArcs([5], 0, TAU, 0)[0]!
    expect(a.start).toBe(0)
    expect(a.end).toBeCloseTo(TAU, 10)
  })

  it('adds up to 360 degrees without gaps', () => {
    const arcs = layoutArcs([500, 300, 150, 50, 0], 0, TAU, 0)
    expect(arcs).toHaveLength(4)
    const sum = arcs.reduce((s, a) => s + (a.end - a.start), 0)
    expect(sum).toBeCloseTo(TAU, 10)
    for (let i = 1; i < arcs.length; i++) expect(arcs[i]!.start).toBeCloseTo(arcs[i - 1]!.end, 10)
  })
})

describe('layoutRings', () => {
  it('keeps ring 1 inside its parent arc and scales it by the filled share', () => {
    const arcs = layoutRings([60, 40], [[30, 10], []], [60, 40])
    const parent = arcs.find((a) => a.ring === 0 && a.index === 0)!
    const kids = arcs.filter((a) => a.ring === 1)
    expect(kids).toHaveLength(2)
    expect(kids[0]!.start).toBeCloseTo(parent.start, 10)
    const span = kids.reduce((s, a) => s + (a.end - a.start), 0)
    expect(span).toBeCloseTo((parent.end - parent.start) * (40 / 60), 10)
    for (const k of kids) expect(k.parent).toBe(0)
  })

  it('ignores missing or invalid child sizes', () => {
    const arcs = layoutRings([10, 10], [[], [0, 0]], [10, 10])
    expect(arcs.every((a) => a.ring === 0)).toBe(true)
  })
})

describe('arcAt', () => {
  const geo = { cx: 100, cy: 100, inner: 20, ringWidth: 30 }
  const arcs = layoutRings([1, 1], [[], []], [1, 1])

  it('misses the hole and outside the rings', () => {
    expect(arcAt(arcs, geo, 105, 100)).toBeNull()
    expect(arcAt(arcs, geo, 100 + 20 + 30 * 3, 100)).toBeNull()
  })

  it('finds the arc by angle and radius', () => {
    expect(arcAt(arcs, geo, 130, 100)?.index).toBe(0)
    expect(arcAt(arcs, geo, 70, 100)?.index).toBe(1)
  })
})
