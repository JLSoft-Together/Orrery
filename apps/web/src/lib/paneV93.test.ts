import { describe, expect, it } from 'vitest'
import { meterState, newestTurnAfter, orbitDots, orbitEntries, orbitTone, turnSummary } from './paneV93'

const sub = (id: string, status: 'working' | 'waiting' | 'completed' | 'idle', extra = {}) => ({ id, role: 'subagent' as const, status, ...extra })

describe('orbit', () => {
  it('keeps only live subagents', () => {
    const list = orbitEntries([{ id: 'm', role: 'main', status: 'working' }, sub('a', 'working', { currentTool: 'Read' }), sub('b', 'completed'), sub('c', 'idle'), sub('d', 'waiting')])
    expect(list.map((e) => e.id)).toEqual(['a', 'd'])
    expect(list[0]!.tool).toBe('Read')
    expect(list[1]!.tool).toBeNull()
  })

  it('caps the dots at 5 and spaces them evenly', () => {
    const entries = Array.from({ length: 8 }, (_, i) => ({ id: `s${i}`, status: 'working' as const }))
    const dots = orbitDots(entries)
    expect(dots).toHaveLength(5)
    expect(dots.map((d) => d.angle)).toEqual([0, 72, 144, 216, 288])
    expect(orbitDots([])).toEqual([])
  })

  it('tone is working while any subagent works', () => {
    expect(orbitTone([{ status: 'waiting' }, { status: 'working' }])).toBe('working')
    expect(orbitTone([{ status: 'waiting' }])).toBe('waiting')
  })
})

describe('meterState', () => {
  it('hides without real numbers', () => {
    expect(meterState(undefined, 500000)).toBeNull()
    expect(meterState(0, 500000)).toBeNull()
    expect(meterState(100, 0)).toBeNull()
    expect(meterState(Number.NaN, 5)).toBeNull()
    expect(meterState(100, undefined)).toBeNull()
  })

  it('tones by ratio', () => {
    expect(meterState(399_999, 500_000)?.tone).toBe('ok')
    expect(meterState(400_000, 500_000)?.tone).toBe('warn')
    expect(meterState(500_000, 500_000)?.tone).toBe('warn')
    expect(meterState(500_001, 500_000)?.tone).toBe('over')
  })

  it('clamps the fill but keeps the ratio', () => {
    const m = meterState(750_000, 500_000)!
    expect(m.fill).toBe(1)
    expect(m.ratio).toBe(1.5)
  })
})

describe('turnSummary', () => {
  it('is null for no files', () => {
    expect(turnSummary([])).toBeNull()
  })

  it('sums and lists the first three', () => {
    const files = ['a', 'b', 'c', 'd', 'e'].map((p, i) => ({ path: p, status: 'M' as const, added: i + 1, removed: 1 }))
    const s = turnSummary(files)!
    expect(s.files).toBe(5)
    expect(s.added).toBe(15)
    expect(s.removed).toBe(5)
    expect(s.top.map((f) => f.path)).toEqual(['a', 'b', 'c'])
    expect(s.more).toBe(2)
  })

  it('treats missing counts as zero', () => {
    expect(turnSummary([{ path: 'x', status: 'A' }])).toMatchObject({ added: 0, removed: 0, more: 0 })
  })
})

describe('newestTurnAfter', () => {
  const list = [
    { id: '3', at: 5000, label: 'after:t' },
    { id: '2', at: 4000, label: 'before:t' },
    { id: '1', at: 1000, label: 'after:old' },
  ]
  it('finds the newest after: since the turn ended', () => {
    expect(newestTurnAfter(list, 4500)?.id).toBe('3')
  })
  it('ignores older turns and unset time', () => {
    expect(newestTurnAfter(list, 9000)).toBeNull()
    expect(newestTurnAfter(list, 0)).toBeNull()
  })
})
