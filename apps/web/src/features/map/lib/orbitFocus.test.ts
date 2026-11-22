import { describe, expect, it } from 'vitest'
import type { ToolSpan } from '@ccm/shared'
import { arcLength, arcPath, computeFocus, feedRows, railOrder, stepSystem, TRAILS_PER_AGENT } from './orbitFocus'

const T0 = Date.parse('2026-11-11T10:00:00Z')
const span = (id: string, agentId: string, startSec: number, endSec?: number, outcome?: ToolSpan['outcome']): ToolSpan => ({
  id,
  terminalId: 't',
  agentId,
  tool: 'Bash',
  start: new Date(T0 + startSec * 1000).toISOString(),
  end: endSec === undefined ? undefined : new Date(T0 + endSec * 1000).toISOString(),
  outcome,
})

describe('orbitFocus', () => {
  it('makes trails longer for longer tool calls, within bounds', () => {
    expect(arcLength(0)).toBeGreaterThanOrEqual(0.08)
    expect(arcLength(1000)).toBeLessThan(arcLength(60_000))
    expect(arcLength(10 * 3600_000)).toBeLessThanOrEqual(0.7)
  })

  it('lays trails of one planet behind it, newest nearest, never overlapping', () => {
    const spans = [span('a', 'p1', 0, 2, 'ok'), span('b', 'p1', 10, 40, 'ok'), span('c', 'p1', 50)]
    const out = computeFocus({ terminalId: 't', mainId: 'm', subs: [{ id: 'p1', status: 'working' }], spans, nowMs: T0 + 60_000, width: 700, height: 560 })
    const planet = out.planets[0]!
    const trails = out.trails.filter((x) => x.agentId === 'p1')
    expect(trails.map((x) => x.id)).toEqual(['c', 'b', 'a'])
    expect(trails[0]!.running).toBe(true)
    expect(trails[0]!.a1).toBeLessThan(planet.angle)
    for (let i = 1; i < trails.length; i++) expect(trails[i]!.a1).toBeLessThan(trails[i - 1]!.a0)
    expect(trails.every((x) => x.r === planet.r)).toBe(true)
  })

  it('keeps at most a few trails per agent and puts main spans on the inner ring', () => {
    const spans = Array.from({ length: 10 }, (_, i) => span(`s${i}`, 'm', i * 5, i * 5 + 1, 'ok'))
    const out = computeFocus({ terminalId: 't', mainId: 'm', subs: [], spans, nowMs: T0 + 100_000, width: 700, height: 560 })
    expect(out.trails).toHaveLength(TRAILS_PER_AGENT)
    expect(out.trails.every((x) => x.r === out.mainR)).toBe(true)
    expect(out.mainR).toBeLessThan(out.rings[0]!)
  })

  it('is stable per agent id and nudges a planet only when motion is allowed', () => {
    const base = { terminalId: 't', mainId: 'm', subs: [{ id: 'p1', status: 'working' as const }], spans: [], nowMs: T0, width: 700, height: 560 }
    const a = computeFocus(base).planets[0]!
    expect(computeFocus(base).planets[0]!.angle).toBe(a.angle)
    expect(computeFocus({ ...base, nudges: { p1: 1 } }).planets[0]!.angle).not.toBe(a.angle)
    expect(computeFocus({ ...base, nudges: { p1: 1 }, reduced: true }).planets[0]!.angle).toBe(a.angle)
  })

  it('builds an arc path from two angles', () => {
    expect(arcPath(100, 100, 50, 0, Math.PI / 2)).toBe('M150 100 A50 50 0 0 1 100 150')
    expect(arcPath(0, 0, 10, 0, Math.PI * 1.5)).toContain(' 0 1 1 ')
  })

  it('lists tool calls newest first with state, never arguments', () => {
    const live = new Set(['m'])
    const rows = feedRows([span('a', 'm', 0, 2, 'ok'), span('b', 'm', 10), span('c', 'gone', 20), span('d', 'm', 5, 6, 'error')], live)
    expect(rows.map((r) => r.id)).toEqual(['c', 'b', 'd', 'a'])
    expect(rows.map((r) => r.state)).toEqual(['interrupted', 'running', 'error', 'ok'])
    expect(rows[3]!.ms).toBe(2000)
    expect(feedRows(Array.from({ length: 80 }, (_, i) => span(`x${i}`, 'm', i, i + 1)), live)).toHaveLength(60)
    expect(Object.keys(rows[0]!)).not.toContain('summary')
  })

  it('orders the rail like the queue and wraps when stepping', () => {
    const ids = railOrder([{ ids: ['n1', 'n2'] }, { ids: ['r1'] }])
    expect(ids).toEqual(['n1', 'n2', 'r1'])
    expect(stepSystem(ids, 'r1', 1)).toBe('n1')
    expect(stepSystem(ids, 'n1', -1)).toBe('r1')
    expect(stepSystem(ids, 'zz', 1)).toBe('n1')
    expect(stepSystem([], 'a', 1)).toBe('a')
  })
})
