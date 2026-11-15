import { describe, expect, it } from 'vitest'
import { cloneTabs, createLayoutUndo, layoutSignature, samePaneSet, snapshotLayout, type TabShape } from './layoutUndo'

const before: TabShape[] = [{ key: 't1', root: { dir: 'row', ratio: 0.5, a: { pane: 'a' }, b: { pane: 'b' } }, focus: 'a', project: '/p' }]
const after: TabShape[] = [
  { key: 't1', root: { pane: 'a' }, focus: 'a', project: '/p' },
  { key: 't2', root: { pane: 'b' }, focus: 'b' },
]
const panes = [{ key: 'a' }, { key: 'b', project: '/q' }]

describe('snapshotLayout', () => {
  it('copies the tabs deeply and records pane projects', () => {
    const snap = snapshotLayout(before, panes, 'a')
    expect(snap.projects).toEqual({ a: null, b: '/q' })
    expect(snap.active).toBe('a')
    expect(snap.tabs[0]!.root).not.toBe(before[0]!.root)
    expect(snap.sig).toBe(layoutSignature(before))
  })

  it('drops runtime-only fields from the signature', () => {
    const extra = [{ ...before[0]!, folded: false }]
    expect(layoutSignature(extra)).toBe(layoutSignature(before))
  })

  it('compares pane sets', () => {
    const snap = snapshotLayout(before, panes, 'a')
    expect(samePaneSet(snap, ['b', 'a'])).toBe(true)
    expect(samePaneSet(snap, ['a'])).toBe(false)
    expect(samePaneSet(snap, ['a', 'b', 'c'])).toBe(false)
  })

  it('clones tabs for restore', () => {
    const snap = snapshotLayout(before, panes, 'a')
    const out = cloneTabs(snap)
    expect(out).toEqual(snap.tabs)
    expect(out[0]).not.toBe(snap.tabs[0])
  })
})

describe('createLayoutUndo', () => {
  it('undoes only while the layout is still the one it made', () => {
    const undo = createLayoutUndo()
    const snap = snapshotLayout(before, panes, 'a')
    const sig = layoutSignature(after)
    expect(undo.record(snap, sig)).toBe(true)
    expect(undo.canUndo(sig, ['a', 'b'])).toBe(true)
    expect(undo.canUndo(layoutSignature(before), ['a', 'b'])).toBe(false)
    expect(undo.canUndo(sig, ['a', 'b', 'c'])).toBe(false)
    expect(undo.take(sig, ['a', 'b'])).toBe(snap)
    expect(undo.canUndo(sig, ['a', 'b'])).toBe(false)
  })

  it('ignores a no-op', () => {
    const undo = createLayoutUndo()
    const snap = snapshotLayout(before, panes, 'a')
    expect(undo.record(snap, snap.sig)).toBe(false)
    expect(undo.pending()).toBeNull()
  })

  it('clear forgets the entry', () => {
    const undo = createLayoutUndo()
    undo.record(snapshotLayout(before, panes, 'a'), layoutSignature(after))
    undo.clear()
    expect(undo.take(layoutSignature(after), ['a', 'b'])).toBeNull()
  })
})
