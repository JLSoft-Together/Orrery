import { describe, expect, it } from 'vitest'
import { en } from '../i18n/en'
import { vi } from '../i18n/vi'
import type { InboxRow } from './inboxModel'
import { SNOOZE_MS, hideKey, hideRow, isHidden, pruneHidden, swipeDecision, unhideRow, visibleRows } from './inboxLocal'

const row = (paneKey: string, reason: InboxRow['reason'] = 'done', since?: string): InboxRow => ({ paneKey, title: paneKey, provider: 'claude', reason, group: 'done', terminalId: null, ...(since ? { since } : {}) })

describe('inbox swipe decision', () => {
  it('left is seen, right is snooze, both past 72px', () => {
    expect(swipeDecision(200, -80, 4)).toBe('seen')
    expect(swipeDecision(200, 80, 4)).toBe('snooze')
  })
  it('ignores short, mostly vertical and iOS back-edge swipes', () => {
    expect(swipeDecision(200, -60, 0)).toBeNull()
    expect(swipeDecision(200, -90, 80)).toBeNull()
    expect(swipeDecision(10, 120, 0)).toBeNull()
  })
})

describe('inbox local hide', () => {
  const now = 1_000_000
  it('seen hides until the row changes, snooze hides for 15 minutes', () => {
    const a = row('a', 'done', 't1')
    const b = row('b')
    let map = hideRow({}, hideKey(a), 'seen', now)
    map = hideRow(map, hideKey(b), 'snooze', now)
    expect(visibleRows([a, b], map, now + 1000)).toEqual([])
    expect(visibleRows([a, b], map, now + SNOOZE_MS + 1)).toEqual([b])
    expect(visibleRows([row('a', 'done', 't2')], map, now)).toHaveLength(1)
    expect(visibleRows([row('a', 'question', 't1')], map, now)).toHaveLength(1)
  })
  it('undo restores and prune drops stale keys', () => {
    const a = row('a')
    const map = hideRow({}, hideKey(a), 'seen', now)
    expect(isHidden(unhideRow(map, hideKey(a)), hideKey(a), now)).toBe(false)
    expect(pruneHidden(map, [], now)).toEqual({})
    expect(pruneHidden(map, [a], now)).toEqual(map)
    expect(pruneHidden(hideRow({}, hideKey(a), 'snooze', now), [a], now + SNOOZE_MS + 1)).toEqual({})
  })
  it('has text in both languages', () => {
    for (const m of [en, vi]) for (const k of ['seen', 'snooze', 'more', 'undo', 'seenToast', 'snoozeToast'] as const) expect(m.inbox[k]).toBeTruthy()
  })
})
