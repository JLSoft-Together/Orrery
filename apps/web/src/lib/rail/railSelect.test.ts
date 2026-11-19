import { describe, expect, it } from 'vitest'
import { isSeparator } from '../menu'
import { clickSelect, extendSelect, NO_SELECTION, pruneSelection, selectionFacts, selectionMenu, selectionPanes } from './railSelect'

const order = ['a', 'b', 'c', 'd', 'e']
const t = (k: string, p?: Record<string, unknown>) => (p ? `${k}:${JSON.stringify(p)}` : k)

describe('clickSelect', () => {
  it('plain click clears', () => {
    expect(clickSelect({ keys: ['a', 'b'], anchor: 'a' }, 'c', order, { toggle: false, range: false }, 'a')).toEqual(NO_SELECTION)
  })

  it('first toggle seeds with the focused row', () => {
    expect(clickSelect(NO_SELECTION, 'c', order, { toggle: true, range: false }, 'a')).toEqual({ keys: ['a', 'c'], anchor: 'c' })
  })

  it('toggle on the focused row alone does not seed it twice', () => {
    expect(clickSelect(NO_SELECTION, 'a', order, { toggle: true, range: false }, 'a')).toEqual({ keys: ['a'], anchor: 'a' })
  })

  it('toggle removes a selected row and empties to none', () => {
    const one = clickSelect({ keys: ['a', 'c'], anchor: 'c' }, 'c', order, { toggle: true, range: false }, null)
    expect(one.keys).toEqual(['a'])
    expect(clickSelect(one, 'a', order, { toggle: true, range: false }, null)).toEqual(NO_SELECTION)
  })

  it('shift selects the range from the focused row', () => {
    expect(clickSelect(NO_SELECTION, 'd', order, { toggle: false, range: true }, 'b')).toEqual({ keys: ['b', 'c', 'd'], anchor: 'b' })
  })

  it('shift replaces the range around the same anchor; ctrl+shift adds it', () => {
    const sel = { keys: ['b', 'c', 'd'], anchor: 'b' }
    expect(clickSelect(sel, 'a', order, { toggle: false, range: true }, 'd').keys).toEqual(['a', 'b'])
    expect(clickSelect({ keys: ['e'], anchor: 'e' }, 'c', order, { toggle: true, range: true }, null).keys).toEqual(['e', 'c', 'd'])
  })
})

describe('extendSelect', () => {
  it('grows and shrinks from the anchor', () => {
    const one = extendSelect(NO_SELECTION, 'b', 'c', order)
    expect(one).toEqual({ keys: ['b', 'c'], anchor: 'b' })
    const two = extendSelect(one, 'c', 'd', order)
    expect(two.keys).toEqual(['b', 'c', 'd'])
    expect(extendSelect(two, 'd', 'a', order).keys).toEqual(['a', 'b'])
  })
})

describe('pruneSelection', () => {
  it('drops rows that went away and keeps the same object when nothing did', () => {
    const sel = { keys: ['a', 'b'], anchor: 'a' }
    expect(pruneSelection(sel, () => true)).toBe(sel)
    expect(pruneSelection(sel, (k) => k === 'b')).toEqual({ keys: ['b'], anchor: 'b' })
    expect(pruneSelection(sel, () => false)).toEqual(NO_SELECTION)
  })
})

describe('selectionPanes', () => {
  it('expands tab rows to their panes, without duplicates', () => {
    const rows: Record<string, { paneKey: string; tabKey: string; tabRow: boolean }> = {
      p1: { paneKey: 'p1', tabKey: 't1', tabRow: false },
      'tab:t2': { paneKey: 'p2', tabKey: 't2', tabRow: true },
    }
    const leaves: Record<string, string[]> = { t1: ['p1'], t2: ['p2', 'p3', 'p1'] }
    expect(selectionPanes(['p1', 'tab:t2', 'gone'], (k) => rows[k], (t) => leaves[t] ?? [])).toEqual(['p1', 'p2', 'p3'])
  })
})

describe('selectionFacts', () => {
  const tabs: Record<string, { key: string; size: number }> = { p1: { key: 't1', size: 1 }, p2: { key: 't2', size: 2 }, p3: { key: 't2', size: 2 } }
  it('group needs two tabs, ungroup a split tab', () => {
    expect(selectionFacts(['p1', 'p2'], (k) => tabs[k])).toEqual({ group: true, ungroup: true })
    expect(selectionFacts(['p2', 'p3'], (k) => tabs[k])).toEqual({ group: false, ungroup: true })
    expect(selectionFacts(['p1'], (k) => tabs[k])).toEqual({ group: false, ungroup: false })
  })
})

describe('selectionMenu', () => {
  it('lists group, ungroup, close N and clear', () => {
    const entries = selectionMenu({ count: 3, group: false, ungroup: true }, t)
    const actions = entries.filter((e) => !isSeparator(e)) as Array<{ id: string; label: string; disabled?: boolean }>
    expect(actions.map((a) => a.id)).toEqual(['sel-group', 'sel-ungroup', 'sel-close', 'sel-clear'])
    expect(actions[0]!.disabled).toBe(true)
    expect(actions[1]!.disabled).toBe(false)
    expect(actions[2]!.label).toBe('railSelect.close:{"n":3}')
  })
})
