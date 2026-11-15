import { describe, expect, it } from 'vitest'
import { boxes, leaves, type LayoutNode } from './splitLayout'
import { chunk, evenLayout, layoutFits, lineOf, presetAllowed, presetLayout, sameLayout } from './layoutPresets'

const keys = ['a', 'b', 'c', 'd', 'e']
const area = (root: LayoutNode): Map<string, { x: number; y: number; w: number; h: number }> => boxes(root)
const near = (a: number, b: number): boolean => Math.abs(a - b) < 1e-9

describe('chunk', () => {
  it('splits a list into balanced parts', () => {
    expect(chunk(keys, 2)).toEqual([['a', 'b', 'c'], ['d', 'e']])
    expect(chunk(keys, 3)).toEqual([['a', 'b'], ['c', 'd'], ['e']])
    expect(chunk(['a'], 3)).toEqual([['a']])
  })
})

describe('lineOf', () => {
  it('gives each item the same share', () => {
    const root = lineOf(['a', 'b', 'c'].map((pane) => ({ pane })), 'row')!
    const b = area(root)
    for (const k of ['a', 'b', 'c']) expect(near(b.get(k)!.w, 1 / 3)).toBe(true)
  })
})

describe('presetLayout', () => {
  it('keeps every pane and adds none', () => {
    for (const p of ['cols2', 'cols3', 'grid2', 'mainCol', 'topTwo'] as const) {
      const root = presetLayout(p, keys, 'c')!
      expect([...leaves(root)].sort()).toEqual(keys)
    }
  })

  it('2 columns: two columns of stacked panes', () => {
    const b = area(presetLayout('cols2', ['a', 'b', 'c', 'd'])!)
    expect(b.get('a')).toEqual({ x: 0, y: 0, w: 0.5, h: 0.5 })
    expect(b.get('d')).toEqual({ x: 0.5, y: 0.5, w: 0.5, h: 0.5 })
  })

  it('3 columns: equal widths', () => {
    const b = area(presetLayout('cols3', ['a', 'b', 'c'])!)
    for (const k of ['a', 'b', 'c']) expect(near(b.get(k)!.w, 1 / 3)).toBe(true)
  })

  it('2 x 2: rows of two', () => {
    const b = area(presetLayout('grid2', ['a', 'b', 'c', 'd'])!)
    expect(b.get('b')).toEqual({ x: 0.5, y: 0, w: 0.5, h: 0.5 })
    expect(b.get('c')).toEqual({ x: 0, y: 0.5, w: 0.5, h: 0.5 })
  })

  it('1 big + column: focus on the left', () => {
    const b = area(presetLayout('mainCol', ['a', 'b', 'c'], 'c')!)
    expect(b.get('c')).toEqual({ x: 0, y: 0, w: 0.6, h: 1 })
    expect(near(b.get('a')!.x, 0.6)).toBe(true)
  })

  it('top + 2 below: focus on top', () => {
    const b = area(presetLayout('topTwo', ['a', 'b', 'c'], 'b')!)
    expect(b.get('b')).toEqual({ x: 0, y: 0, w: 1, h: 0.5 })
    expect(b.get('a')).toEqual({ x: 0, y: 0.5, w: 0.5, h: 0.5 })
  })

  it('one pane is the zoom, not a tree', () => {
    expect(presetLayout('one', keys)).toBeNull()
    expect(presetLayout('cols2', ['a'])).toEqual({ pane: 'a' })
    expect(presetLayout('cols2', [])).toBeNull()
  })
})

describe('evenLayout', () => {
  it('gives every pane of a chain the same size', () => {
    const chain: LayoutNode = { dir: 'row', ratio: 0.8, a: { pane: 'a' }, b: { dir: 'row', ratio: 0.2, a: { pane: 'b' }, b: { pane: 'c' } } }
    const b = area(evenLayout(chain))
    for (const k of ['a', 'b', 'c']) expect(near(b.get(k)!.w, 1 / 3)).toBe(true)
  })

  it('keeps groups and their meta', () => {
    const root: LayoutNode = { dir: 'row', ratio: 0.7, a: { group: 'p', name: 'X', node: { pane: 'a' } }, b: { group: 'q', node: { pane: 'b' } } }
    const out = evenLayout(root)
    expect(out).toEqual({ dir: 'row', ratio: 0.5, a: { group: 'p', name: 'X', node: { pane: 'a' } }, b: { group: 'q', node: { pane: 'b' } } })
  })
})

describe('guards', () => {
  it('presets only for an ungrouped tab with two or more panes', () => {
    expect(presetAllowed({ pane: 'a' })).toBe(false)
    expect(presetAllowed({ dir: 'row', ratio: 0.5, a: { pane: 'a' }, b: { pane: 'b' } })).toBe(true)
    expect(presetAllowed({ group: 'p', node: { dir: 'row', ratio: 0.5, a: { pane: 'a' }, b: { pane: 'b' } } })).toBe(false)
  })

  it('layoutFits checks the minimum pane size', () => {
    const three = presetLayout('cols3', ['a', 'b', 'c'])!
    expect(layoutFits(three, { w: 1200, h: 800 })).toBe(true)
    expect(layoutFits(three, { w: 600, h: 800 })).toBe(false)
    expect(layoutFits(three, { w: 0, h: 0 })).toBe(true)
  })

  it('sameLayout compares structure', () => {
    expect(sameLayout({ pane: 'a' }, { pane: 'a' })).toBe(true)
    expect(sameLayout({ pane: 'a' }, { pane: 'b' })).toBe(false)
  })
})
