import { describe, expect, it } from 'vitest'
import { assignSlots, computeSky, hashName, planetAngle, ringCount, splitDimmed, systemExtent, type SkySystem } from './skyLayout'

const sys = (terminalId: string, project: string, subs = 0): SkySystem => ({
  terminalId,
  project,
  mainId: `m-${terminalId}`,
  subs: Array.from({ length: subs }, (_, i) => `${terminalId}-s${i}`),
})

describe('skyLayout', () => {
  it('hashes names to the same number every time', () => {
    expect(hashName('Orrery')).toBe(hashName('Orrery'))
    expect(hashName('Orrery')).not.toBe(hashName('Habblaud'))
  })

  it('keeps a project slot independent of input order', () => {
    const a = assignSlots(['app', 'api', 'web', 'docs'])
    const b = assignSlots(['docs', 'web', 'api', 'app'])
    expect([...a.entries()].sort()).toEqual([...b.entries()].sort())
    expect(new Set(a.values()).size).toBe(4)
  })

  it('places the same systems at the same points regardless of order', () => {
    const list = [sys('t1', 'app', 2), sys('t2', 'api', 0), sys('t3', 'web', 7)]
    const a = computeSky({ systems: list, dots: [] })
    const b = computeSky({ systems: [...list].reverse(), dots: [] })
    for (const [id, p] of a.positions) expect(b.positions.get(id)).toEqual(p)
  })

  it('puts subagents on rings around the sun, 6 on the first ring', () => {
    expect(ringCount(0)).toBe(0)
    expect(ringCount(6)).toBe(1)
    expect(ringCount(7)).toBe(2)
    const r = computeSky({ systems: [sys('t1', 'app', 7)], dots: [] })
    expect(r.orbits).toHaveLength(2)
    expect(systemExtent(7).w).toBeGreaterThan(systemExtent(6).w)
    expect(r.positions.size).toBe(8)
  })

  it('collapses idle and ended sessions into dots only above 8 sessions', () => {
    const few = Array.from({ length: 8 }, (_, i) => ({ id: `s${i}`, status: 'idle' as const }))
    expect(splitDimmed(few).dimmed).toEqual([])
    const many = [...few, { id: 'w', status: 'working' as const }, { id: 'n', status: 'waiting' as const }, { id: 'e', status: 'stale' as const }]
    const r = splitDimmed(many)
    expect(r.full).toEqual(['w', 'n'])
    expect(r.dimmed).toHaveLength(9)
  })

  it('lays dots below the systems', () => {
    const r = computeSky({ systems: [sys('t1', 'app')], dots: [{ id: 'a', project: 'x' }, { id: 'b', project: 'y' }] })
    expect(r.dots.size).toBe(2)
    for (const p of r.dots.values()) expect(p.y).toBeGreaterThan(r.suns.get('t1')!.y)
  })

  it('nudges a planet only when motion is allowed', () => {
    expect(planetAngle(0, 4, 0, 3, true)).toBe(planetAngle(0, 4, 0, 0, true))
    expect(planetAngle(0, 4, 0, 3, false)).toBeGreaterThan(planetAngle(0, 4, 0, 0, false))
    expect(planetAngle(0, 4, 0, 4, false)).toBe(planetAngle(0, 4, 0, 0, false))
    const still = computeSky({ systems: [sys('t1', 'app', 2)], dots: [], nudges: { 't1-s0': 2 }, reduced: true })
    const base = computeSky({ systems: [sys('t1', 'app', 2)], dots: [], reduced: true })
    expect(still.positions.get('t1-s0')).toEqual(base.positions.get('t1-s0'))
    const moved = computeSky({ systems: [sys('t1', 'app', 2)], dots: [], nudges: { 't1-s0': 2 } })
    expect(moved.positions.get('t1-s0')).not.toEqual(base.positions.get('t1-s0'))
  })
})
