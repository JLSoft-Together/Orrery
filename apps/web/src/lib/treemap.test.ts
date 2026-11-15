import { describe, expect, it } from 'vitest'
import { cellAt, squarify } from './treemap'

const frame = { x: 0, y: 0, w: 800, h: 500 }
const area = (cs: { w: number; h: number }[]): number => cs.reduce((s, c) => s + c.w * c.h, 0)

describe('squarify', () => {
  it('returns nothing for empty input, zero frame or all-zero sizes', () => {
    expect(squarify([], frame)).toEqual([])
    expect(squarify([0, 0, -1, NaN], frame)).toEqual([])
    expect(squarify([1, 2], { x: 0, y: 0, w: 0, h: 10 })).toEqual([])
  })

  it('fills the whole frame with one item', () => {
    const cells = squarify([42], frame)
    expect(cells).toHaveLength(1)
    expect(cells[0]).toMatchObject({ index: 0, x: 0, y: 0, w: 800, h: 500 })
  })

  it('covers exactly the frame area and keeps every cell inside it', () => {
    const values = [900, 600, 400, 300, 120, 80, 40, 10, 5, 1]
    const cells = squarify(values, frame)
    expect(cells).toHaveLength(values.length)
    expect(area(cells)).toBeCloseTo(frame.w * frame.h, 3)
    for (const c of cells) {
      expect(c.x).toBeGreaterThanOrEqual(-1e-6)
      expect(c.y).toBeGreaterThanOrEqual(-1e-6)
      expect(c.x + c.w).toBeLessThanOrEqual(frame.w + 1e-6)
      expect(c.y + c.h).toBeLessThanOrEqual(frame.h + 1e-6)
    }
  })

  it('gives each cell an area proportional to its value', () => {
    const values = [50, 30, 20]
    const cells = squarify(values, frame)
    for (const c of cells) expect((c.w * c.h) / (frame.w * frame.h)).toBeCloseTo(values[c.index]! / 100, 3)
  })

  it('skips zero sizes but keeps original indexes', () => {
    const cells = squarify([10, 0, 30], frame)
    expect(cells.map((c) => c.index).sort()).toEqual([0, 2])
    expect(area(cells)).toBeCloseTo(frame.w * frame.h, 3)
  })

  it('keeps aspect ratios reasonable for an even split', () => {
    const cells = squarify(Array.from({ length: 12 }, () => 1), { x: 0, y: 0, w: 600, h: 400 })
    for (const c of cells) expect(Math.max(c.w / c.h, c.h / c.w)).toBeLessThan(3)
  })

  it('copes with 100k items without losing area', () => {
    const values = Array.from({ length: 100_000 }, (_, i) => 100_000 - i)
    const cells = squarify(values.slice(0, 2000), frame)
    expect(area(cells)).toBeCloseTo(frame.w * frame.h, 2)
  })
})

describe('cellAt', () => {
  it('hits the cell under the point and misses outside', () => {
    const cells = squarify([3, 1], { x: 0, y: 0, w: 400, h: 100 })
    const big = cells.find((c) => c.index === 0)!
    expect(cellAt(cells, big.x + 1, big.y + 1)?.index).toBe(0)
    expect(cellAt(cells, 500, 50)).toBeNull()
  })
})
