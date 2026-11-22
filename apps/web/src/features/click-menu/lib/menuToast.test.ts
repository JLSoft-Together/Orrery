import { describe, expect, it } from 'vitest'
import { buildMenuToastPayload, itemForAnswer, menuSig, parseMenuToastAction, toastOptions } from './menuToast'
import type { TermMenu, TermMenuItem } from './termMenu'

const item = (index: number, label: string, extra: Partial<TermMenuItem> = {}): TermMenuItem => ({ index, label, row: index, selected: index === 1, ...extra })
const menu = (items: TermMenuItem[], extra: Partial<TermMenu> = {}): TermMenu => ({
  kind: 'ask',
  agent: 'claude',
  items,
  highlighted: 1,
  numbered: true,
  confident: true,
  firstRow: 0,
  lastRow: items.length,
  ...extra,
})

describe('toastOptions', () => {
  it('lists up to 3 numbered options', () => {
    expect(toastOptions(menu([item(1, 'Yes'), item(2, 'No')]))).toEqual([
      { index: 1, label: 'Yes' },
      { index: 2, label: 'No' },
    ])
    expect(toastOptions(menu([item(1, 'a'), item(2, 'b'), item(3, 'c')]))).toHaveLength(3)
  })

  it('gives null (Open only) for more than 3, multi select, unnumbered, Submit or Other rows', () => {
    expect(toastOptions(menu([1, 2, 3, 4].map((i) => item(i, String(i)))))).toBeNull()
    expect(toastOptions(menu([item(1, 'a'), item(2, 'b')], { kind: 'ask-multi' }))).toBeNull()
    expect(toastOptions(menu([item(1, 'a'), item(2, 'b')], { numbered: false }))).toBeNull()
    expect(toastOptions(menu([item(1, 'a'), item(2, 'Submit', { isSubmit: true })]))).toBeNull()
    expect(toastOptions(menu([item(1, 'a'), item(2, 'Other', { isOther: true })]))).toBeNull()
    expect(toastOptions(menu([item(1, 'a'), item(1, 'b')]))).toBeNull()
    expect(toastOptions(menu([]))).toBeNull()
  })
})

describe('menuSig', () => {
  it('changes with the layout or the question, not with the highlight', () => {
    const a = menu([item(1, 'Yes'), item(2, 'No')])
    const moved = menu([item(1, 'Yes', { selected: false }), item(2, 'No', { selected: true })])
    expect(menuSig(a, 'Q?')).toBe(menuSig(moved, 'Q?'))
    expect(menuSig(a, 'Q?')).not.toBe(menuSig(a, 'Other?'))
    expect(menuSig(a, 'Q?')).not.toBe(menuSig(menu([item(1, 'Yes'), item(2, 'Maybe')]), 'Q?'))
    expect(menuSig(a, 'Q?')).toMatch(/^[0-9a-f]{8}$/)
  })
})

describe('buildMenuToastPayload', () => {
  const rows = [{ paneKey: 'p1', menu: menu([item(1, 'Yes'), item(2, 'No')]), question: 'Q?', title: 'Q?', place: 'orrery', terminalId: 't1' }]

  it('builds entries with sig and options', () => {
    const p = buildMenuToastPayload({ enabled: true, open: 'Open', dismiss: 'Dismiss', rows })
    expect(p.entries[0]).toMatchObject({ paneKey: 'p1', terminalId: 't1', title: 'Q?', place: 'orrery', options: [{ index: 1, label: 'Yes' }, { index: 2, label: 'No' }] })
    expect(p.entries[0]?.sig).toBe(menuSig(rows[0]!.menu, 'Q?'))
  })

  it('sends no entries when disabled', () => {
    expect(buildMenuToastPayload({ enabled: false, open: 'Open', dismiss: 'Dismiss', rows }).entries).toEqual([])
  })
})

describe('answers from the toast', () => {
  const m = menu([item(1, 'Yes'), item(2, 'No')])
  const sig = menuSig(m, 'Q?')

  it('resolves the item of the current menu', () => {
    expect(itemForAnswer(m, 'Q?', { type: 'answer', paneKey: 'p', sig, index: 2 })?.label).toBe('No')
  })

  it('drops a stale signature, an unknown index or a menu that no longer has toast options', () => {
    expect(itemForAnswer(m, 'Q?', { type: 'answer', paneKey: 'p', sig: 'deadbeef', index: 1 })).toBeNull()
    expect(itemForAnswer(m, 'Q?', { type: 'answer', paneKey: 'p', sig, index: 3 })).toBeNull()
    const multi = menu([item(1, 'Yes'), item(2, 'No')], { kind: 'ask-multi' })
    expect(itemForAnswer(multi, 'Q?', { type: 'answer', paneKey: 'p', sig: menuSig(multi, 'Q?'), index: 1 })).toBeNull()
  })

  it('parses only well formed actions', () => {
    expect(parseMenuToastAction({ type: 'open', paneKey: 'p', sig: 's' })).toEqual({ type: 'open', paneKey: 'p', sig: 's' })
    expect(parseMenuToastAction({ type: 'answer', paneKey: 'p', sig: 's', index: 1 })).toEqual({ type: 'answer', paneKey: 'p', sig: 's', index: 1 })
    expect(parseMenuToastAction({ type: 'answer', paneKey: 'p', sig: 's', index: '1' })).toBeNull()
    expect(parseMenuToastAction({ type: 'answer', paneKey: 'p', sig: 's', index: 1.5 })).toBeNull()
    expect(parseMenuToastAction({ type: 'x', paneKey: 'p', sig: 's' })).toBeNull()
    expect(parseMenuToastAction(null)).toBeNull()
  })
})
