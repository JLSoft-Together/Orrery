import { describe, expect, it } from 'vitest'
import { boxes, cleanGroups, dividers, dockGroup, dockSpot, dockTree, dropTargetAt, edgeAt, foldedKeys, foldLayout, GROUP_NAME_MAX, groupBoxes, groupOf, groupsOf, hasGroups, isLayout, metaOf, neighborGroup, setGroupMeta, leaves, moveGroup, movePane, neighbor, removeLeaf, setRatio, settleGroups, splitLeaf, swapPanes, takeGroup, ungrouped, type LayoutNode, type Rect } from './splitLayout'

// a | (b / c)
const tree: LayoutNode = splitLeaf(splitLeaf({ pane: 'a' }, 'a', 'b', 'row'), 'b', 'c', 'col')

describe('layout tree', () => {
  it('splits a leaf in place and keeps order', () => {
    expect(leaves(tree)).toEqual(['a', 'b', 'c'])
    expect(boxes(tree).get('c')).toEqual({ x: 0.5, y: 0.5, w: 0.5, h: 0.5 })
  })

  it('removing a leaf lets its sibling take the parent slot', () => {
    const t = removeLeaf(tree, 'b')!
    expect(leaves(t)).toEqual(['a', 'c'])
    expect(boxes(t).get('c')).toEqual({ x: 0.5, y: 0, w: 0.5, h: 1 })
    expect(removeLeaf({ pane: 'a' }, 'a')).toBeNull()
    expect(removeLeaf(tree, 'zz')).toBe(tree)
  })

  it('clamps ratios and addresses nested splits by path', () => {
    const t = setRatio(setRatio(tree, '', 0.99), 'b', 0.3)
    const d = dividers(t)
    expect(d.map((x) => [x.path, x.dir, x.ratio])).toEqual([
      ['', 'row', 0.85],
      ['b', 'col', 0.3],
    ])
    expect(boxes(t).get('a')!.w).toBeCloseTo(0.85)
  })

  it('moves focus to the adjacent pane', () => {
    expect(neighbor(tree, 'a', 'right')).toBe('b')
    expect(neighbor(tree, 'b', 'down')).toBe('c')
    expect(neighbor(tree, 'c', 'up')).toBe('b')
    expect(neighbor(tree, 'c', 'left')).toBe('a')
    expect(neighbor(tree, 'a', 'left')).toBeNull()
    expect(neighbor(tree, 'b', 'right')).toBeNull()
  })

  it('validates stored trees', () => {
    expect(isLayout(tree)).toBe(true)
    expect(isLayout({ dir: 'row', ratio: 0.5, a: { pane: 'a' } })).toBe(false)
    expect(isLayout(null)).toBe(false)
  })
})

describe('pane docking', () => {
  it('docks a pane on each edge of a leaf', () => {
    const one: LayoutNode = { pane: 'a' }
    expect(dockTree(one, 'a', { pane: 'x' }, 'left')).toEqual({ dir: 'row', ratio: 0.5, a: { pane: 'x' }, b: { pane: 'a' } })
    expect(dockTree(one, 'a', { pane: 'x' }, 'right')).toEqual({ dir: 'row', ratio: 0.5, a: { pane: 'a' }, b: { pane: 'x' } })
    expect(dockTree(one, 'a', { pane: 'x' }, 'top')).toEqual({ dir: 'col', ratio: 0.5, a: { pane: 'x' }, b: { pane: 'a' } })
    expect(dockTree(one, 'a', { pane: 'x' }, 'bottom')).toEqual({ dir: 'col', ratio: 0.5, a: { pane: 'a' }, b: { pane: 'x' } })
    expect(dockTree(tree, 'zz', { pane: 'x' }, 'left')).toBe(tree)
  })

  it('docks a whole tree next to a leaf', () => {
    const sub: LayoutNode = splitLeaf({ pane: 'x' }, 'x', 'y', 'col')
    const t = dockTree(tree, 'c', sub, 'right')
    expect(leaves(t)).toEqual(['a', 'b', 'c', 'x', 'y'])
    expect(boxes(t).get('y')).toEqual({ x: 0.75, y: 0.75, w: 0.25, h: 0.25 })
  })

  it('moves a pane to another edge without losing panes', () => {
    // a | (b / c) → c moves left of a: (c | a) | b
    const t = movePane(tree, 'c', 'a', 'left')
    expect(leaves(t)).toEqual(['c', 'a', 'b'])
    expect(boxes(t).get('b')).toEqual({ x: 0.5, y: 0, w: 0.5, h: 1 })
    expect(boxes(t).get('c')).toEqual({ x: 0, y: 0, w: 0.25, h: 1 })
    expect(movePane(tree, 'a', 'a', 'left')).toBe(tree)
    expect(movePane(tree, 'zz', 'a', 'left')).toBe(tree)
  })

  it('swaps two leaves and keeps the shape', () => {
    const t = swapPanes(tree, 'a', 'c')
    expect(leaves(t)).toEqual(['c', 'b', 'a'])
    expect(dividers(t)).toEqual(dividers(tree))
    expect(swapPanes(tree, 'a', 'zz')).toBe(tree)
  })

  it('picks the nearest edge by fraction of the size', () => {
    const r: Rect = { left: 0, top: 0, right: 400, bottom: 100 }
    expect(edgeAt(r, 10, 50)).toBe('left')
    expect(edgeAt(r, 390, 50)).toBe('right')
    expect(edgeAt(r, 200, 10)).toBe('top')
    expect(edgeAt(r, 200, 95)).toBe('bottom')
    // 60px from the left of a 400px pane is 15%; 20px from the top of 100px is 20%.
    expect(edgeAt(r, 60, 20)).toBe('left')
  })

  it('finds the pane under the pointer, never the source', () => {
    const rects = new Map<string, Rect>([
      ['a', { left: 0, top: 0, right: 100, bottom: 100 }],
      ['b', { left: 108, top: 0, right: 208, bottom: 100 }],
    ])
    expect(dropTargetAt(rects, 190, 50, 'a')).toEqual({ key: 'b', edge: 'right' })
    expect(dropTargetAt(rects, 50, 50, 'a')).toBeNull()
    expect(dropTargetAt(rects, 104, 50, 'a')).toBeNull()
  })
})

describe('dock spot', () => {
  it('splits the roomiest pane along its longer side, the focused one on a tie', () => {
    expect(dockSpot(tree, 'c', { w: 1600, h: 900 })).toEqual({ key: 'a', dir: 'col' })
    expect(dockSpot(tree, 'c', { w: 1600, h: 900 })).toEqual(dockSpot(tree, 'b', { w: 1600, h: 900 }))
    expect(dockSpot(splitLeaf({ pane: 'a' }, 'a', 'b', 'row'), 'b', { w: 1600, h: 900 })).toEqual({ key: 'b', dir: 'col' })
    expect(dockSpot({ pane: 'a' }, 'a', { w: 1600, h: 900 })).toEqual({ key: 'a', dir: 'row' })
    expect(dockSpot({ pane: 'a' }, 'a', { w: 0, h: 0 })).toEqual({ key: 'a', dir: 'row' })
  })

  it('falls back to the other side, then to the roomiest pane, then gives up', () => {
    // c is 250 × 150: too small both ways; a (250 × 300) is taller than wide, so it halves downward.
    expect(dockSpot(tree, 'c', { w: 500, h: 300 })).toEqual({ key: 'a', dir: 'col' })
    expect(dockSpot({ pane: 'a' }, 'a', { w: 300, h: 600 })).toEqual({ key: 'a', dir: 'col' })
    expect(dockSpot({ pane: 'a' }, 'a', { w: 400, h: 250 })).toBeNull()
  })
})

describe('project groups', () => {
  const A = 'C:/a'
  const B = 'D:/b'
  const proj: Record<string, string> = { a1: A, a2: A, b1: B, b2: B, c1: 'E:/c' }
  const of = (k: string): string => proj[k]!

  it('keeps a single-project tree plain and names its project', () => {
    const t = splitLeaf({ pane: 'a1' }, 'a1', 'a2', 'row')
    const res = settleGroups(t, of)
    expect(res.root).toBe(t)
    expect(res.project).toBe(A)
    expect(hasGroups(res.root)).toBe(false)
  })

  it('turns scattered panes of two projects into one group each, keeping their order', () => {
    const t: LayoutNode = { dir: 'row', ratio: 0.5, a: { dir: 'col', ratio: 0.5, a: { pane: 'a1' }, b: { pane: 'b1' } }, b: { dir: 'col', ratio: 0.5, a: { pane: 'a2' }, b: { pane: 'b2' } } }
    const { root, project } = settleGroups(t, of)
    expect(project).toBeUndefined()
    expect(groupsOf(root)).toEqual([
      { project: A, keys: ['a1', 'a2'] },
      { project: B, keys: ['b1', 'b2'] },
    ])
    expect(leaves(root)).toHaveLength(4)
  })

  it('unwraps the last group left and drops empty groups', () => {
    const t = dockGroup(splitLeaf({ pane: 'a1' }, 'a1', 'a2', 'col'), A, 'a1', B, { pane: 'b1' }, 'right')
    expect(groupsOf(t).map((g) => g.project)).toEqual([A, B])
    const gone = removeLeaf(t, 'b1')!
    expect(groupsOf(gone).map((g) => g.project)).toEqual([A])
    const res = settleGroups(gone, of)
    expect(hasGroups(res.root)).toBe(false)
    expect(res.project).toBe(A)
    expect(removeLeaf({ group: A, node: { pane: 'a1' } }, 'a1')).toBeNull()
  })

  it('docks a new group beside the group of the near pane, or into the project group already there', () => {
    let t = dockGroup({ pane: 'a1' }, A, 'a1', B, { pane: 'b1' }, 'left')
    expect(groupsOf(t).map((g) => g.project)).toEqual([B, A])
    t = dockGroup(t, A, 'a1', B, { pane: 'b2' }, 'right')
    expect(groupsOf(t)).toEqual([
      { project: B, keys: ['b1', 'b2'] },
      { project: A, keys: ['a1'] },
    ])
    t = dockGroup(t, A, 'a1', 'E:/c', { pane: 'c1' }, 'bottom')
    expect(groupOf(t, 'c1')).toBe('E:/c')
    expect(groupBoxes(t).get('E:/c')).toEqual({ x: 0.5, y: 0.5, w: 0.5, h: 0.5 })
  })

  it('splits inside a group, tags its dividers and reaches its ratios by path', () => {
    let t = dockGroup({ pane: 'a1' }, A, 'a1', B, { pane: 'b1' }, 'right')
    t = splitLeaf(t, 'b1', 'b2', 'col')
    expect(groupOf(t, 'b2')).toBe(B)
    const d = dividers(t)
    expect(d.map((x) => [x.path, x.group])).toEqual([
      ['', undefined],
      ['b', B],
    ])
    t = setRatio(t, 'b', 0.3)
    expect(boxes(t).get('b1')!.h).toBeCloseTo(0.3)
    expect(isLayout(t)).toBe(true)
    expect(isLayout({ group: A, node: { nope: 1 } })).toBe(false)
  })

  it('moves and takes whole groups', () => {
    let t = dockGroup({ pane: 'a1' }, A, 'a1', B, { pane: 'b1' }, 'right')
    t = dockGroup(t, A, 'b1', 'E:/c', { pane: 'c1' }, 'right')
    expect(groupsOf(moveGroup(t, 'E:/c', A, 'left')).map((g) => g.project)).toEqual(['E:/c', A, B])
    expect(moveGroup(t, B, B, 'left')).toBe(t)
    const took = takeGroup(t, B)!
    expect(leaves(took.node)).toEqual(['b1'])
    expect(groupsOf(took.rest!).map((g) => g.project)).toEqual([A, 'E:/c'])
    expect(leaves(ungrouped(t))).toEqual(['a1', 'b1', 'c1'])
    expect(hasGroups(ungrouped(t))).toBe(false)
  })

  it('merges a second group of the same project into the first', () => {
    const t: LayoutNode = { dir: 'row', ratio: 0.5, a: { group: A, node: { pane: 'a1' } }, b: { dir: 'row', ratio: 0.5, a: { group: B, node: { pane: 'b1' } }, b: { group: A, node: { pane: 'a2' } } } }
    const { root } = settleGroups(t, of)
    expect(groupsOf(root)).toEqual([
      { project: A, keys: ['a1', 'a2'] },
      { project: B, keys: ['b1'] },
    ])
  })
})

describe('project group options', () => {
  const row = (): LayoutNode => dockGroup(dockGroup({ pane: 'a1' }, 'A', 'a1', 'B', { pane: 'b1' }, 'right'), 'A', 'b1', 'C', { pane: 'c1' }, 'right')

  it('keeps name, colour and fold through settle and leaf moves, and cleans bad values', () => {
    let t = setGroupMeta(row(), 'B', { name: '  API  ', tone: 3, folded: true })
    expect(groupsOf(t).find((g) => g.project === 'B')).toMatchObject({ name: 'API', tone: 3, folded: true })
    t = splitLeaf(t, 'b1', 'b2', 'col')
    t = settleGroups(t, () => 'x').root
    expect(groupsOf(t).find((g) => g.project === 'B')).toMatchObject({ name: 'API', tone: 3, folded: true })
    t = setGroupMeta(t, 'B', { tone: undefined, folded: false })
    expect(groupsOf(t).find((g) => g.project === 'B')).toEqual({ project: 'B', keys: ['b1', 'b2'], name: 'API' })
    expect(metaOf({ name: 'x'.repeat(80), tone: 99, folded: false })).toEqual({ name: 'x'.repeat(GROUP_NAME_MAX) })
    expect(groupsOf(cleanGroups({ dir: 'row', ratio: 0.5, a: { group: 'A', node: { pane: 'a1' }, tone: -1 } as LayoutNode, b: { group: 'B', node: { pane: 'b1' } } })).map((g) => g.tone)).toEqual([undefined, undefined])
  })

  it('collapses a group to a fixed strip and locks its divider', () => {
    const t = setGroupMeta(row(), 'A', { folded: true })
    const { root, locked } = foldLayout(t, { w: 1000, h: 600 }, { w: 36, h: 34 })
    expect(locked).toEqual([''])
    expect(groupBoxes(root).get('A')!.w).toBeCloseTo(0.036)
    expect(foldedKeys(t)).toEqual(new Set(['a1']))
    expect(foldLayout(t, { w: 0, h: 0 }, { w: 36, h: 34 }).root).toBe(t)
  })

  it('finds the neighbour group left and right', () => {
    const t = row()
    expect(neighborGroup(t, 'B', 'right')).toBe('C')
    expect(neighborGroup(t, 'B', 'left')).toBe('A')
    expect(neighborGroup(t, 'A', 'left')).toBeNull()
    expect(neighborGroup({ pane: 'a1' }, 'A', 'left')).toBeNull()
  })
})
