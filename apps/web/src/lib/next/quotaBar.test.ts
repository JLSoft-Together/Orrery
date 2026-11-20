import { describe, expect, it } from 'vitest'
import type { PlanLimits } from '@ccm/shared'
import { quotaAgents, resetLabel, resetRelative } from './quotaBar'

const NOW = new Date(2026, 10, 19, 10, 0, 0).getTime()
const fresh = new Date(NOW - 60_000).toISOString()

const limits = (over: Partial<PlanLimits>): PlanLimits => ({ updatedAt: fresh, ...over })

describe('quotaAgents', () => {
  it('lists one cluster per agent that has real numbers', () => {
    const list = quotaAgents(limits({ fiveHour: { usedPct: 42.4 }, sevenDay: { usedPct: 91 } }), limits({ fiveHour: { usedPct: 5 } }), NOW)
    expect(list.map((a) => a.id)).toEqual(['claude', 'codex'])
    expect(list[0]!.name).toBe('Claude Code')
    expect(list[0]!.windows.map((w) => [w.key, w.used, w.tone])).toEqual([
      ['5h', 42, 'ok'],
      ['7d', 91, 'critical'],
    ])
    expect(list[1]!.windows).toHaveLength(1)
  })

  it('drops agents without a source or without finite numbers', () => {
    expect(quotaAgents(null, undefined, NOW)).toEqual([])
    expect(quotaAgents(limits({}), limits({ fiveHour: { usedPct: Number.NaN } }), NOW)).toEqual([])
    expect(quotaAgents(null, limits({ sevenDay: { usedPct: 10 } }), NOW).map((a) => a.id)).toEqual(['codex'])
  })

  it('clamps and flags stale reports', () => {
    const old = new Date(NOW - 31 * 60_000).toISOString()
    const [a] = quotaAgents({ updatedAt: old, fiveHour: { usedPct: 140 } }, null, NOW)
    expect(a!.stale).toBe(true)
    expect(a!.windows[0]!.used).toBe(100)
    expect(quotaAgents(limits({ fiveHour: { usedPct: 1 } }), null, NOW)[0]!.stale).toBe(false)
  })

  it('reads the reset time', () => {
    const at = new Date(NOW + 3_600_000).toISOString()
    const [a] = quotaAgents(limits({ fiveHour: { usedPct: 1, resetsAt: at } }), null, NOW)
    expect(a!.windows[0]!.resetsAt).toBe(Date.parse(at))
  })
})

describe('resetLabel', () => {
  it('shows the clock for today and the weekday for later', () => {
    const today = resetLabel(new Date(2026, 10, 19, 14, 30).getTime(), NOW, 'en-GB')
    expect(today).toBe('14:30')
    const later = resetLabel(new Date(2026, 10, 21, 9, 0).getTime(), NOW, 'en-GB')
    expect(later).toMatch(/^\D+ 09:00$/)
  })
  it('is empty without a future reset', () => {
    expect(resetLabel(null, NOW, 'en')).toBe('')
    expect(resetLabel(NOW - 1, NOW, 'en')).toBe('')
  })
})

describe('resetRelative', () => {
  const en = { d: 'd', h: 'h', min: 'min', lessThanMin: 'under 1 min' }
  const vi = { d: 'ngày', h: 'giờ', min: 'phút', lessThanMin: 'dưới 1 phút' }
  const at = (ms: number): number => NOW + ms

  it('gives hours and minutes', () => {
    expect(resetRelative(at(2 * 3_600_000 + 5 * 60_000), NOW, en)).toBe('2 h 5 min')
    expect(resetRelative(at(2 * 3_600_000 + 5 * 60_000), NOW, vi)).toBe('2 giờ 5 phút')
  })

  it('gives minutes only, whole hours and days without minutes', () => {
    expect(resetRelative(at(45 * 60_000), NOW, en)).toBe('45 min')
    expect(resetRelative(at(3 * 3_600_000), NOW, en)).toBe('3 h')
    expect(resetRelative(at(26 * 3_600_000 + 30 * 60_000), NOW, en)).toBe('1 d 2 h')
  })

  it('rounds seconds up and handles missing or past times', () => {
    expect(resetRelative(at(20_000), NOW, en)).toBe('1 min')
    expect(resetRelative(null, NOW, en)).toBe('')
    expect(resetRelative(at(-1), NOW, en)).toBe('')
  })
})
