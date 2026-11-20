import { describe, expect, it } from 'vitest'
import type { Pane, Tab } from '../stores/workspace'
import { menuSig } from './menuToast'
import { pickForPhone, remoteMenuOf } from './remoteMenu'
import { buildSummary } from './remoteSummary'
import type { TermMenu } from './termMenu'

const menu = (over: Partial<TermMenu> = {}): TermMenu => ({
  kind: 'permission',
  agent: 'claude',
  items: [
    { index: 1, label: 'Yes', row: 4, selected: true },
    { index: 2, label: "Yes, and don't ask again", row: 5, selected: false },
    { index: 3, label: 'Type something', row: 6, selected: false, isOther: true },
  ],
  highlighted: 1,
  numbered: true,
  confident: false,
  firstRow: 4,
  lastRow: 6,
  ...over,
})

const entry = (m = menu()) => ({ menu: m, question: 'Do you want to proceed?', since: 1000 })

describe('remoteMenuOf', () => {
  it('maps the queue entry to the phone shape with the toast signature and no rows', () => {
    const r = remoteMenuOf(entry())!
    expect(r).toEqual({
      sig: menuSig(menu(), 'Do you want to proceed?'),
      kind: 'permission',
      agent: 'claude',
      question: 'Do you want to proceed?',
      items: [{ index: 1, label: 'Yes', selected: true }, { index: 2, label: "Yes, and don't ask again" }, { index: 3, label: 'Type something', other: true }],
      numbered: true,
      confident: false,
      since: 1000,
    })
  })

  it('sends only confident menus or permission prompts, and caps size', () => {
    expect(remoteMenuOf(entry(menu({ kind: 'ask', confident: false })))).toBeUndefined()
    expect(remoteMenuOf(entry(menu({ kind: 'ask', confident: true })))).toBeDefined()
    expect(remoteMenuOf(null)).toBeUndefined()
    const many = menu({ items: Array.from({ length: 20 }, (_, i) => ({ index: i + 1, label: 'x'.repeat(300), row: i, selected: false })) })
    const r = remoteMenuOf(entry(many))!
    expect(r.items).toHaveLength(12)
    expect(r.items[0]!.label).toHaveLength(120)
  })

  it('lands in the summary for term panes only', () => {
    const tabs: Tab[] = [{ key: 't', root: { pane: 'p' }, focus: 'p' }]
    const panes: Pane[] = [{ key: 'p', preset: 'claude', cwd: 'C:\a', state: 'running' }]
    const facts = () => ({ title: 'x', rail: 'asked', busy: false, subagents: 0 })
    const s = buildSummary({ tabs, panes, closedCount: 0, facts, menu: () => remoteMenuOf(entry()) }, 1)
    expect(s.tabs[0]!.panes[0]!.menu?.items).toHaveLength(3)
    const none = buildSummary({ tabs, panes, closedCount: 0, facts }, 1)
    expect(none.tabs[0]!.panes[0]).not.toHaveProperty('menu')
  })
})

describe('pickForPhone', () => {
  const sig = menuSig(menu(), 'Do you want to proceed?')
  it('checks signature, index and Other', () => {
    expect(pickForPhone(entry(), 'deadbeef', 1)).toEqual({ error: 'busy' })
    expect(pickForPhone(entry(), sig, 9)).toEqual({ error: 'notFound' })
    expect(pickForPhone(entry(), sig, 3)).toEqual({ error: 'invalid' })
    expect(pickForPhone(null, sig, 1)).toEqual({ error: 'notFound' })
    expect(pickForPhone(entry(), sig, 2)).toMatchObject({ item: { index: 2 } })
  })
})
