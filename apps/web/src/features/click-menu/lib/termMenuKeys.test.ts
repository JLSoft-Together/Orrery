import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { parseTermMenu, type TermCursor, type TermMenu } from './termMenu'
import { KEY_DOWN, KEY_ENTER, KEY_SPACE, KEY_UP, menuLayoutKey, menuStateKey, planConfirmKeys, planHighlightKeys, planMenuKeys } from './termMenuKeys'

const DIR = join(__dirname, '__fixtures__', 'term-menu')

function menuOf(name: string): TermMenu {
  const lines = readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
  let cursor: TermCursor | undefined
  try {
    cursor = JSON.parse(readFileSync(join(DIR, `${name}.cursor.json`), 'utf8')) as TermCursor
  } catch {
    cursor = undefined
  }
  return parseTermMenu(lines, cursor)!
}

const plan = (menu: TermMenu, label: string) => planMenuKeys(menu, menu.items.find((i) => i.label === label)!)

describe('planMenuKeys', () => {
  it('sends the number for a numbered Claude menu', () => {
    const m = menuOf('claude-ask-single')
    expect(plan(m, 'Blue')).toEqual({ keys: '3', after: 'none' })
    expect(plan(m, 'Red')).toEqual({ keys: '1', after: 'none' })
  })

  it('sends the number for a numbered Claude picker', () => {
    const m = menuOf('claude-model')
    expect(m.items.length).toBeGreaterThan(1)
    const second = m.items[1]!
    expect(planMenuKeys(m, second)).toEqual({ keys: String(second.index), after: 'none' })
  })

  it('uses Up / Down distance then Enter when the menu has no numbers', () => {
    const m = menuOf('claude-trust-nonum')
    expect(m.numbered).toBe(false)
    const from = m.items.findIndex((i) => i.selected)
    const last = m.items[m.items.length - 1]!
    expect(planMenuKeys(m, last)).toEqual({ keys: KEY_DOWN.repeat(m.items.length - 1 - from) + KEY_ENTER, after: 'none' })
    const first = m.items[0]!
    expect(planMenuKeys(m, first)).toEqual({ keys: from === 0 ? KEY_ENTER : KEY_UP.repeat(from) + KEY_ENTER, after: 'none' })
  })

  it('moves up for an item above the highlight when the menu has no numbers', () => {
    const m = menuOf('claude-mcp-prompt')
    const from = m.items.findIndex((i) => i.selected)
    const target = m.items[from === 0 ? 1 : 0]!
    const p = planMenuKeys(m, target)!
    expect(p.keys.endsWith(KEY_ENTER)).toBe(true)
    expect(p.keys.startsWith(from === 0 ? KEY_DOWN : KEY_UP)).toBe(true)
  })

  it('ticks a multiSelect item with its number', () => {
    const m = menuOf('claude-ask-multi')
    expect(m.kind).toBe('ask-multi')
    expect(plan(m, 'Banana')).toEqual({ keys: '2', after: 'none' })
  })

  it('goes Down to Submit then asks to confirm after reading the screen', () => {
    const m = menuOf('claude-ask-multi')
    const submit = m.items.find((i) => i.isSubmit)!
    const from = m.items.findIndex((i) => i.selected)
    const to = m.items.indexOf(submit)
    expect(planMenuKeys(m, submit)).toEqual({ keys: KEY_DOWN.repeat(to - from), after: 'enter-on-submit' })
  })

  it('only moves the highlight to Other and hands focus to the terminal', () => {
    const m = menuOf('claude-ask-single')
    const other = m.items.find((i) => i.isOther)!
    const from = m.items.findIndex((i) => i.selected)
    expect(planMenuKeys(m, other)).toEqual({ keys: KEY_DOWN.repeat(m.items.indexOf(other) - from), after: 'focus' })
  })

  it('does not move when Other already has the highlight', () => {
    const m = menuOf('claude-ask-other-highlight')
    const other = m.items.find((i) => i.isOther)!
    expect(other.selected).toBe(true)
    expect(planMenuKeys(m, other)).toEqual({ keys: '', after: 'focus' })
  })

  it('sends the number for a numbered Codex menu', () => {
    const m = menuOf('codex-permissions')
    expect(m.agent).toBe('codex')
    const target = m.items[1]!
    expect(planMenuKeys(m, target)).toEqual({ keys: String(target.index), after: 'none' })
  })

  it('answers a Codex approval with its number', () => {
    const m = menuOf('codex-approval')
    expect(m.kind).toBe('codex-approval')
    expect(planMenuKeys(m, m.items[0]!)).toEqual({ keys: String(m.items[0]!.index), after: 'none' })
  })

  it('falls back to navigation for a two digit number, with Space in a multiSelect', () => {
    const base = menuOf('claude-ask-multi')
    const item = { ...base.items[1]!, index: 12 }
    const m: TermMenu = { ...base, items: [base.items[0]!, item, ...base.items.slice(2)] }
    expect(planMenuKeys(m, item)).toEqual({ keys: KEY_DOWN + KEY_SPACE, after: 'none' })
  })

  it('cannot navigate without a highlight', () => {
    const base = menuOf('claude-trust-nonum')
    const m: TermMenu = { ...base, items: base.items.map((i) => ({ ...i, selected: false })) }
    expect(planMenuKeys(m, m.items[1]!)).toBeNull()
  })
})

describe('menu keys', () => {
  it('layout key ignores the highlight, state key does not', () => {
    const a = menuOf('claude-ask-single')
    const b = menuOf('claude-ask-other-highlight')
    expect(menuStateKey(a)).not.toBe(menuStateKey(b))
    expect(menuLayoutKey(a)).toBe(menuLayoutKey(a))
  })
})

describe('planHighlightKeys / planConfirmKeys', () => {
  const at = (m: TermMenu, label: string) => m.items.find((i) => i.label === label)!

  it('click moves the highlight with arrows, even on a numbered menu', () => {
    const m = menuOf('claude-ask-single')
    const from = m.items.findIndex((i) => i.selected)
    const to = m.items.indexOf(at(m, 'Blue'))
    expect(planHighlightKeys(m, at(m, 'Blue'))).toBe((to > from ? KEY_DOWN : KEY_UP).repeat(Math.abs(to - from)))
    expect(planHighlightKeys(m, m.items[from]!)).toBe('')
  })

  it('uses the predicted highlight when given', () => {
    const m = menuOf('claude-ask-single')
    expect(planHighlightKeys(m, at(m, 'Blue'), m.items.indexOf(at(m, 'Blue')))).toBe('')
    expect(planHighlightKeys(m, at(m, 'Red'), m.items.indexOf(at(m, 'Blue')))).toBe(KEY_UP.repeat(2))
  })

  it('double click sends the number on a numbered menu', () => {
    const m = menuOf('claude-ask-single')
    expect(planConfirmKeys(m, at(m, 'Blue'))).toEqual({ keys: '3', after: 'none' })
  })

  it('double click on an unnumbered menu is arrows then Enter, or only Enter once on the row', () => {
    const m = menuOf('claude-trust-nonum')
    const last = m.items[m.items.length - 1]!
    const from = m.items.findIndex((i) => i.selected)
    const n = m.items.length - 1 - from
    expect(planConfirmKeys(m, last)).toEqual({ keys: KEY_DOWN.repeat(n) + KEY_ENTER, after: 'none' })
    expect(planConfirmKeys(m, last, m.items.length - 1)).toEqual({ keys: KEY_ENTER, after: 'none' })
  })

  it('double click does nothing on multiSelect, Submit and Other', () => {
    const m = menuOf('claude-ask-multi')
    for (const i of m.items) expect(planConfirmKeys(m, i)).toBeNull()
  })

  it('Codex numbered menu confirms with the number', () => {
    const m = menuOf('codex-permissions')
    expect(planConfirmKeys(m, m.items[1]!)).toEqual({ keys: '2', after: 'none' })
  })
})
