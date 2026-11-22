import { describe, expect, it } from 'vitest'
import { compassTiles, compassZoneAt, dropFit, tileAt, zoneRect, COMPASS_TILE } from './dropCompass'

const R = { left: 100, top: 50, right: 500, bottom: 450 }

describe('compass tiles', () => {
  it('places five 44px tiles around the pane centre', () => {
    const tiles = compassTiles(R)
    expect(tiles.map((t) => t.zone)).toEqual(['top', 'left', 'center', 'right', 'bottom'])
    const c = tiles.find((t) => t.zone === 'center')!
    expect(c.size).toBe(COMPASS_TILE)
    expect(c.left + c.size / 2).toBe(300)
    expect(c.top + c.size / 2).toBe(250)
    const right = tiles.find((t) => t.zone === 'right')!
    expect(right.left - c.left).toBe(48)
    const top = tiles.find((t) => t.zone === 'top')!
    expect(c.top - top.top).toBe(48)
  })

  it('finds the tile under the pointer', () => {
    expect(tileAt(R, 300, 250)).toBe('center')
    expect(tileAt(R, 348, 250)).toBe('right')
    expect(tileAt(R, 252, 250)).toBe('left')
    expect(tileAt(R, 300, 202)).toBe('top')
    expect(tileAt(R, 300, 298)).toBe('bottom')
    expect(tileAt(R, 324, 226)).toBeNull()
  })

  it('falls back to the nearest edge off the tiles', () => {
    expect(compassZoneAt(R, 480, 250)).toEqual({ zone: 'right', onTile: false })
    expect(compassZoneAt(R, 300, 60)).toEqual({ zone: 'top', onTile: false })
    expect(compassZoneAt(R, 348, 250)).toEqual({ zone: 'right', onTile: true })
    expect(compassZoneAt(R, 300, 250)).toEqual({ zone: 'center', onTile: true })
  })
})

describe('dropFit', () => {
  it('needs 220px on each half after the gap', () => {
    expect(dropFit({ width: 448, height: 300 }, 'right')).toEqual({ ok: true, left: 220, min: 220 })
    expect(dropFit({ width: 446, height: 300 }, 'left')).toEqual({ ok: false, left: 219, min: 220 })
    expect(dropFit({ width: 900, height: 412 }, 'bottom')).toEqual({ ok: false, left: 202, min: 220 })
    expect(dropFit({ width: 100, height: 100 }, 'center').ok).toBe(true)
  })
})

describe('zoneRect', () => {
  const r = { left: 10, top: 20, width: 400, height: 200 }
  it('halves the pane on the dropped side', () => {
    expect(zoneRect(r, 'left')).toEqual({ left: 10, top: 20, width: 200, height: 200 })
    expect(zoneRect(r, 'right')).toEqual({ left: 210, top: 20, width: 200, height: 200 })
    expect(zoneRect(r, 'top')).toEqual({ left: 10, top: 20, width: 400, height: 100 })
    expect(zoneRect(r, 'bottom')).toEqual({ left: 10, top: 120, width: 400, height: 100 })
    expect(zoneRect(r, 'center')).toEqual(r)
  })
})
