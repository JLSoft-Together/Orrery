import { describe, expect, it } from 'vitest'
import { INBOX_GROUPS as SHARED_GROUPS, INBOX_REASONS as SHARED_REASONS } from '../../../../packages/shared/src/inbox'
import { en } from '../i18n/en'
import { vi } from '../i18n/vi'
import { INBOX_GROUPS, inboxCount, inboxRows, inboxSections } from './inboxModel'

const pane = (key: string, extra: Record<string, unknown> = {}) => ({ key, kind: 'term', title: key, rail: 'working', terminalId: `t-${key}`, sessionId: `s-${key}`, ...extra })
const doc = (panes: unknown[], inbox?: true) => ({ rev: 1, ...(inbox ? { inbox } : {}), tabs: [{ key: 'tab1', panes }] })

describe('inbox model', () => {
  it('uses the shared group order', () => {
    expect([...INBOX_GROUPS]).toEqual([...SHARED_GROUPS])
    expect(INBOX_GROUPS).toEqual(['menu', 'permission', 'question', 'clash', 'error', 'note', 'done'])
  })

  it('has text for every group and reason in both languages', () => {
    for (const messages of [en, vi]) {
      for (const g of SHARED_GROUPS) expect(messages.inbox.group[g]).toBeTruthy()
      for (const r of SHARED_REASONS) expect(messages.inbox.reason[r]).toBeTruthy()
    }
  })

  it('builds rows from pane.inbox when the snapshot flag is set, sorted by group then oldest first', () => {
    const rows = inboxRows(
      doc(
        [
          pane('d', { inbox: { reason: 'done', since: '2026-11-20T08:00:00Z' } }),
          pane('c', { inbox: { reason: 'clash', since: '2026-11-20T08:05:00Z', file: 'src/a.ts', peer: 'web' } }),
          pane('e2', { inbox: { reason: 'stuck', since: '2026-11-20T08:10:00Z' } }),
          pane('e1', { inbox: { reason: 'failed', since: '2026-11-20T08:01:00Z' } }),
          pane('m', { inbox: { reason: 'menu' } }),
          pane('x', { inbox: { reason: 'nonsense' } }),
          pane('none'),
        ],
        true,
      ),
    )
    expect(rows.map((r) => r.paneKey)).toEqual(['m', 'c', 'e1', 'e2', 'd'])
    expect(rows[1]).toMatchObject({ reason: 'clash', group: 'clash', file: 'src/a.ts', peer: 'web', terminalId: 't-c' })
    expect(inboxSections(rows).map((s) => s.group)).toEqual(['menu', 'clash', 'error', 'done'])
    expect(inboxCount(doc([pane('m', { inbox: { reason: 'menu' } })], true))).toBe(1)
  })

  it('ignores attention when the flag is set', () => {
    expect(inboxRows(doc([pane('a', { attention: 'waiting' })], true))).toEqual([])
  })

  it('falls back to attention on an old desktop', () => {
    const rows = inboxRows(doc([pane('a', { attention: 'done' }), pane('b', { attention: 'waiting' }), pane('c', { inbox: { reason: 'menu' } })]))
    expect(rows.map((r) => [r.paneKey, r.reason, r.group])).toEqual([
      ['b', 'question', 'question'],
      ['a', 'done', 'done'],
    ])
  })

  it('is empty without a workspace', () => {
    expect(inboxRows(null)).toEqual([])
    expect(inboxCount(null)).toBe(0)
  })
})
