import { describe, expect, it } from 'vitest'
import { applyMonitorEvent } from '../remote/monitorReducer'
import { createAppI18n } from '../i18n'
import { formatPct, formatReset, quotaByAgent, quotaOf } from './quotaModel'

const NOW = Date.parse('2026-11-19T10:00:00.000Z')

const limits = (five: unknown, seven: unknown, updatedAt = '2026-11-19T09:55:00.000Z') => ({ fiveHour: five, sevenDay: seven, updatedAt })

describe('quotaOf', () => {
  it('reads Claude Code and Codex windows from the monitor snapshot', () => {
    const snap = {
      limits: limits({ usedPct: 41.6, resetsAt: '2026-11-19T12:00:00.000Z' }, { usedPct: 8, resetsAt: '2026-11-24T00:00:00.000Z' }),
      codexLimits: limits({ usedPct: 100 }, undefined),
    }
    expect(quotaOf(snap, NOW)).toEqual([
      { agent: 'claude', window: '5h', pct: 42, resetAt: Date.parse('2026-11-19T12:00:00.000Z'), stale: false },
      { agent: 'claude', window: 'week', pct: 8, resetAt: Date.parse('2026-11-24T00:00:00.000Z'), stale: false },
      { agent: 'codex', window: '5h', pct: 100, resetAt: null, stale: false },
    ])
  })

  it('never invents a value: missing, null, non-numeric or non-finite windows are skipped', () => {
    expect(quotaOf(null, NOW)).toEqual([])
    expect(quotaOf({}, NOW)).toEqual([])
    expect(quotaOf({ limits: null, codexLimits: 'x' }, NOW)).toEqual([])
    expect(quotaOf({ limits: limits({ usedPct: '40' }, { usedPct: Number.NaN }) }, NOW)).toEqual([])
    expect(quotaOf({ limits: limits(null, [1]) }, NOW)).toEqual([])
  })

  it('clamps to 0..100 and marks reports older than 30 minutes stale', () => {
    const rows = quotaOf({ limits: limits({ usedPct: 140 }, { usedPct: -3 }, '2026-11-19T09:00:00.000Z') }, NOW)
    expect(rows.map((r) => r.pct)).toEqual([100, 0])
    expect(rows.every((r) => r.stale)).toBe(true)
  })

  it('follows limits.updated and codexLimits.updated events already sent to the phone', () => {
    const snap: Record<string, unknown> = {}
    applyMonitorEvent(snap, { type: 'limits.updated', payload: limits({ usedPct: 10 }, undefined) })
    applyMonitorEvent(snap, { type: 'codexLimits.updated', payload: limits(undefined, { usedPct: 20 }) })
    expect(quotaOf(snap, NOW).map((r) => [r.agent, r.window, r.pct])).toEqual([
      ['claude', '5h', 10],
      ['codex', 'week', 20],
    ])
  })

  it('groups rows per agent in order', () => {
    const rows = quotaOf({ limits: limits({ usedPct: 1 }, { usedPct: 2 }), codexLimits: limits({ usedPct: 3 }, undefined) }, NOW)
    expect(quotaByAgent(rows).map((g) => [g.agent, g.windows.length])).toEqual([
      ['claude', 2],
      ['codex', 1],
    ])
  })
})

describe('format', () => {
  it('formats percent by locale', () => {
    expect(formatPct(42, 'en')).toBe('42%')
    expect(formatPct(42, 'vi')).toMatch(/^42\s?%$/)
  })

  it('formats the reset with the duration units of the locale', () => {
    const en = createAppI18n('en').global.t
    const vi = createAppI18n('vi').global.t
    const at = NOW + 2 * 3_600_000
    expect(formatReset(at, NOW, 'en', en)).toMatch(/^resets in 2 h \(.+\)$/)
    expect(formatReset(at, NOW, 'vi', vi)).toMatch(/^đặt lại sau 2 giờ \(.+\)$/)
    expect(formatReset(NOW + 3 * 86_400_000, NOW, 'en', en)).toMatch(/^resets in 3 d \(\w+ .+\)$/)
  })

  it('says nothing when the reset is unknown or already past', () => {
    const en = createAppI18n('en').global.t
    expect(formatReset(null, NOW, 'en', en)).toBe('')
    expect(formatReset(NOW - 1, NOW, 'en', en)).toBe('')
  })
})
