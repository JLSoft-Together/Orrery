import { describe, expect, it } from 'vitest'
import { CELL_H, CELL_W, fitView, idleMinutes, layoutSlots, MAX_PLANETS, peerPath, planetLayout, statusRank, sunRadius, toolFile, zoomAt } from './starMap'

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
