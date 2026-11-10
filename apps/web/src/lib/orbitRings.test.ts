import { describe, expect, it } from 'vitest'
import { assignSectors, computeRings, hashName, navigate, ORBIT_DOT_LIMIT, ringOf, sectorCount, splitDimmed, subAngle, type RingSession } from './orbitRings'

const s = (id: string, project: string, status: RingSession['status'], subs: RingSession['subs'] = []): RingSession => ({ id, project, status, subs })
const dims = { width: 900, height: 640 }

describe('orbitRings', () => {
  it('puts each state on its own ring', () => {
    expect(ringOf('waiting')).toBe(0)
    expect(ringOf('working')).toBe(1)
    expect(ringOf('idle')).toBe(2)
    expect(ringOf('stale')).toBe(2)
    const out = computeRings({ ...dims, sessions: [s('a', 'p', 'waiting'), s('b', 'p', 'working'), s('c', 'p', 'idle')] })
    const dist = (id: string) => {
      const n = out.nodes.find((x) => x.id === id)!
      return Math.hypot(n.x - out.cx, n.y - out.cy)
    }
    expect(dist('a')).toBeCloseTo(out.radii[0]!, 5)
    expect(dist('b')).toBeCloseTo(out.radii[1]!, 5)
    expect(dist('c')).toBeCloseTo(out.radii[2]!, 5)
    expect(out.nodes.map((n) => n.id)).toEqual(['a', 'b', 'c'])
  })

  it('keeps a project sector fixed by name hash, whatever else is open', () => {
    const cells = sectorCount(3)
    const names = ['Orrery', 'Habblaud', 'ThemeStore', 'jin-workflow', 'ControlCenter', 'Notes']
    const free = names.filter((n) => n !== 'Orrery' && hashName(n) % cells !== hashName('Orrery') % cells).slice(0, 2)
    const solo = computeRings({ ...dims, sessions: [s('a', 'Orrery', 'working')] })
    const withOthers = computeRings({ ...dims, sessions: [s('a', 'Orrery', 'working'), s('b', free[0]!, 'idle'), s('c', free[1]!, 'waiting')] })
    const angleOf = (r: ReturnType<typeof computeRings>) => r.sectors.find((x) => x.project === 'Orrery')!.angle
    expect(angleOf(withOthers)).toBe(angleOf(solo))
    expect(solo.sectors[0]!.slot).toBe(hashName('Orrery') % cells)
    const a = assignSectors(['x', 'y', 'z'])
    expect(new Set(a.values()).size).toBe(3)
    expect(assignSectors(['z', 'y', 'x'])).toEqual(a)
  })

  it('grows the sector count in steps of 12 so slots stay put', () => {
    expect(sectorCount(1)).toBe(12)
    expect(sectorCount(12)).toBe(12)
    expect(sectorCount(13)).toBe(24)
  })

  it('collapses idle and done sessions to dim dots past 8 sessions', () => {
    const list = Array.from({ length: ORBIT_DOT_LIMIT + 2 }, (_, i) => ({ id: `s${i}`, status: (i < 2 ? 'working' : 'idle') as RingSession['status'] }))
    const split = splitDimmed(list)
    expect(split.full).toEqual(['s0', 's1'])
    expect(split.dimmed).toHaveLength(8)
    expect(splitDimmed(list.slice(0, 8)).dimmed).toEqual([])
    const out = computeRings({ ...dims, sessions: list.map((x) => s(x.id, 'p', x.status, [{ id: `${x.id}-sub`, status: 'working' }])) })
    expect(out.nodes.filter((n) => n.dot)).toHaveLength(8)
    expect(out.nodes.filter((n) => n.dot).every((n) => n.subs.length === 0)).toBe(true)
    expect(out.nodes.filter((n) => !n.dot).every((n) => n.subs.length === 1)).toBe(true)
  })

  it('places sub-agent dots beside the node and nudges them on a tool call unless motion is reduced', () => {
    const sess = [s('a', 'p', 'working', [{ id: 'x', status: 'working' }, { id: 'y', status: 'waiting' }])]
    const base = computeRings({ ...dims, sessions: sess })
    const nudged = computeRings({ ...dims, sessions: sess, nudges: { x: 2 } })
    const still = computeRings({ ...dims, sessions: sess, nudges: { x: 2 }, reduced: true })
    const pick = (r: ReturnType<typeof computeRings>) => r.nodes[0]!.subs.find((q) => q.id === 'x')!
    expect(Math.hypot(pick(base).x - base.nodes[0]!.x, pick(base).y - base.nodes[0]!.y)).toBeCloseTo(29, 5)
    expect(pick(nudged).x).not.toBeCloseTo(pick(base).x, 3)
    expect(pick(still).x).toBeCloseTo(pick(base).x, 8)
    expect(subAngle('a', 0, 2, 4)).toBeCloseTo(subAngle('a', 0, 2, 0), 8)
  })

  it('caps shown sub-agents and counts the rest', () => {
    const subs = Array.from({ length: 9 }, (_, i) => ({ id: `u${i}`, status: 'working' as const }))
    const n = computeRings({ ...dims, sessions: [s('a', 'p', 'working', subs)] }).nodes[0]!
    expect(n.subs).toHaveLength(6)
    expect(n.moreSubs).toBe(3)
  })

  it('does not stack sessions of one project and ring on the same spot', () => {
    const many = Array.from({ length: 7 }, (_, i) => s(`w${i}`, 'p', 'working'))
    const out = computeRings({ ...dims, sessions: many })
    const keys = new Set(out.nodes.map((n) => `${Math.round(n.x)}:${Math.round(n.y)}`))
    expect(keys.size).toBe(7)
    for (const n of out.nodes) expect(Number.isFinite(n.x + n.y)).toBe(true)
  })

  it('moves the selection toward the arrow and stays put at the edge', () => {
    const nodes = [
      { id: 'l', x: 100, y: 200 },
      { id: 'm', x: 200, y: 200 },
      { id: 'r', x: 300, y: 210 },
      { id: 'u', x: 205, y: 100 },
    ]
    expect(navigate(nodes, 'm', 'ArrowRight')).toBe('r')
    expect(navigate(nodes, 'm', 'ArrowLeft')).toBe('l')
    expect(navigate(nodes, 'm', 'ArrowUp')).toBe('u')
    expect(navigate(nodes, 'u', 'ArrowUp')).toBe('u')
    expect(navigate(nodes, 'r', 'ArrowRight')).toBe('r')
    expect(navigate(nodes, null, 'ArrowRight')).toBe('l')
    expect(navigate([], null, 'ArrowRight')).toBeNull()
  })
})
