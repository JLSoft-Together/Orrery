import { describe, expect, it } from 'vitest'
import { CELL_H, CELL_W, cellSize, fitView, groupSystems, MAX_SUNS, planetCap, sunOffsets, sunScale, idleMinutes, layoutSlots, MAX_PLANETS, peerPath, planetLayout, statusRank, sunRadius, toolFile, zoomAt } from './starMap'

const keys = (n: number) => Array.from({ length: n }, (_, i) => ({ id: `t${i}`, project: `p${i % 4}` }))

describe('layoutSlots', () => {
  it('places every system in its own cell without overlap', () => {
    const r = layoutSlots(keys(12), 900)
    expect(new Set(r.slots.values()).size).toBe(12)
    for (const a of r.placements) {
      for (const b of r.placements) {
        if (a.id === b.id) continue
        expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeGreaterThan(60)
      }
    }
    expect(r.width).toBe(r.cols * CELL_W)
    expect(r.height).toBe(r.rows * CELL_H)
  })

  it('is deterministic and keeps existing slots when a system is added', () => {
    const a = layoutSlots(keys(6), 900)
    expect(layoutSlots(keys(6), 900).slots).toEqual(a.slots)
    const b = layoutSlots(keys(7), 900, a.slots)
    for (const [id, slot] of a.slots) expect(b.slots.get(id)).toBe(slot)
  })

  it('handles empty input and zero width', () => {
    const r = layoutSlots([], 0)
    expect(r.placements).toEqual([])
    expect(r.cols).toBeGreaterThanOrEqual(1)
  })
})

describe('view maths', () => {
  it('fits the world inside the view and centres it', () => {
    const f = fitView({ width: 1600, height: 800 }, { width: 800, height: 600 })
    expect(f.scale).toBe(0.5)
    expect(f.x).toBe(0)
    expect(f.y).toBe(100)
  })
  it('never upscales past 1 when fitting', () => {
    expect(fitView({ width: 100, height: 100 }, { width: 800, height: 600 }).scale).toBe(1)
  })
  it('keeps the point under the cursor fixed while zooming', () => {
    const f = zoomAt({ scale: 1, x: 0, y: 0 }, 2, 100, 50)
    expect(f.scale).toBe(2)
    expect(f.x).toBe(-100)
    expect(f.y).toBe(-50)
  })
})

describe('planetLayout', () => {
  const subs = (n: number) => Array.from({ length: n }, (_, i) => ({ id: `s${i}`, status: 'working' as const }))
  it('uses one ring up to three and two rings above, capped with +N', () => {
    expect(planetLayout(subs(3), 24).rings).toHaveLength(1)
    const big = planetLayout(subs(9), 24)
    expect(big.rings).toHaveLength(2)
    expect(big.spots).toHaveLength(MAX_PLANETS)
    expect(big.more).toBe(3)
  })
  it('has no rings without sub-agents', () => {
    expect(planetLayout([], 24)).toEqual({ rings: [], spots: [], more: 0 })
  })
})

describe('helpers', () => {
  it('ranks needs first', () => {
    expect(['idle', 'needs', 'ended', 'running'].map((g) => statusRank(g as never)).sort()).toEqual([0, 1, 2, 3])
    expect(statusRank('needs')).toBeLessThan(statusRank('running'))
  })
  it('sizes suns by activity', () => {
    expect(sunRadius('running', 3, true)).toBeGreaterThan(sunRadius('idle', 0, false))
    expect(sunRadius('ended', 0, false)).toBeLessThan(sunRadius('idle', 0, false))
  })
  it('draws a curved peer path with a midpoint', () => {
    const p = peerPath({ x: 0, y: 0 }, { x: 100, y: 0 })
    expect(p.d.startsWith('M0 0 Q')).toBe(true)
    expect(p.mx).toBe(50)
    expect(p.my).not.toBe(0)
  })
  it('shows a file basename only for file tools with a bare path', () => {
    expect(toolFile('Edit', '/a/b/c.ts')).toBe('c.ts')
    expect(toolFile('Bash', 'ls')).toBeNull()
    expect(toolFile('Read', 'cat secret.txt')).toBeNull()
    expect(toolFile('Edit', undefined)).toBeNull()
  })
  it('counts idle minutes', () => {
    expect(idleMinutes(new Date(0).toISOString(), 5 * 60_000 + 1)).toBe(5)
    expect(idleMinutes(undefined, 0)).toBeNull()
  })
})

describe('project systems', () => {
  const item = (id: string, project: string, rank: number) => ({ id, project, rank })
  it('groups sessions by project, most urgent first, capped at three suns', () => {
    const g = groupSystems([item('a', 'p', 2), item('b', 'p', 0), item('c', 'q', 1), item('d', 'p', 1), item('e', 'p', 3)])
    expect(g.map((x) => x.project)).toEqual(['p', 'q'])
    expect(g[0]!.items.map((x) => x.id)).toEqual(['b', 'd', 'a'])
    expect(g[0]!.hidden).toBe(1)
    expect(g[0]!.total).toBe(4)
    expect(g[0]!.items).toHaveLength(MAX_SUNS)
    expect(g[1]!.hidden).toBe(0)
  })
  it('spreads suns without overlap and shrinks them with the count', () => {
    for (const n of [1, 2, 3]) {
      const o = sunOffsets(n)
      expect(o).toHaveLength(n)
      for (const a of o) for (const b of o) if (a !== b) expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeGreaterThan(100)
    }
    expect(sunScale(3)).toBeLessThan(sunScale(2))
    expect(sunScale(2)).toBeLessThan(sunScale(1))
    expect(planetCap(3)).toBeLessThan(planetCap(1))
  })
  it('grows the cell only when a system has several suns', () => {
    expect(cellSize(1).w).toBe(CELL_W)
    expect(cellSize(2).w).toBeGreaterThan(CELL_W)
    const r = layoutSlots([{ id: 'p', project: 'p', suns: 3 }, { id: 'q', project: 'q', suns: 1 }], 900)
    expect(r.cellW).toBe(cellSize(3).w)
    expect(r.width).toBe(r.cols * r.cellW)
    expect(r.height).toBe(r.rows * r.cellH)
  })
  it('keeps slots keyed by project across updates', () => {
    const a = layoutSlots([{ id: 'p', project: 'p', suns: 2 }, { id: 'q', project: 'q', suns: 1 }], 900)
    const b = layoutSlots([{ id: 'p', project: 'p', suns: 3 }, { id: 'q', project: 'q', suns: 1 }, { id: 'r', project: 'r', suns: 1 }], 900, a.slots)
    expect(b.slots.get('p')).toBe(a.slots.get('p'))
    expect(b.slots.get('q')).toBe(a.slots.get('q'))
  })
})

describe('groupSystems keys', () => {
  it('keeps two folders with the same name apart', () => {
    const g = groupSystems([
      { id: 'a', project: 'app', key: '/w/one/app', rank: 1 },
      { id: 'b', project: 'app', key: '/w/two/app', rank: 1 },
      { id: 'c', project: 'app', key: '/w/one/app', rank: 0 },
    ])
    expect(g.map((x) => [x.key, x.items.map((i) => i.id)])).toEqual([
      ['/w/one/app', ['c', 'a']],
      ['/w/two/app', ['b']],
    ])
  })
})
