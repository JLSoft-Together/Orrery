import { edgeAt, MIN_PANE, type Edge, type Rect } from '../splitLayout'

export type CompassZone = Edge | 'center'

export const COMPASS_TILE = 44
export const COMPASS_GAP = 4
export const DROP_MIN: number = MIN_PANE.w
export const DROP_GAP: number = 8

export interface TileBox {
  zone: CompassZone
  left: number
  top: number
  size: number
}

const SLOTS: Array<[CompassZone, number, number]> = [
  ['top', 0, -1],
  ['left', -1, 0],
  ['center', 0, 0],
  ['right', 1, 0],
  ['bottom', 0, 1],
]

export function compassCenter(r: Rect): { x: number; y: number } {
  return { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 }
}

export function compassTiles(r: Rect, tile = COMPASS_TILE, gap = COMPASS_GAP): TileBox[] {
  const c = compassCenter(r)
  const step = tile + gap
  return SLOTS.map(([zone, dx, dy]) => ({ zone, left: c.x + dx * step - tile / 2, top: c.y + dy * step - tile / 2, size: tile }))
}

export function tileAt(r: Rect, x: number, y: number, tile = COMPASS_TILE, gap = COMPASS_GAP): CompassZone | null {
  for (const b of compassTiles(r, tile, gap)) if (x >= b.left && x <= b.left + b.size && y >= b.top && y <= b.top + b.size) return b.zone
  return null
}

export function compassZoneAt(r: Rect, x: number, y: number, tile = COMPASS_TILE, gap = COMPASS_GAP): { zone: CompassZone; onTile: boolean } {
  const hit = tileAt(r, x, y, tile, gap)
  return hit ? { zone: hit, onTile: true } : { zone: edgeAt(r, x, y), onTile: false }
}

export interface DropFit {
  ok: boolean
  left: number
  min: number
}

export function dropFit(size: { width: number; height: number }, zone: CompassZone, min = DROP_MIN, gap = DROP_GAP): DropFit {
  if (zone === 'center') return { ok: true, left: Math.floor(Math.min(size.width, size.height)), min }
  const span = zone === 'left' || zone === 'right' ? size.width : size.height
  const left = Math.max(0, Math.floor((span - gap) / 2))
  return { ok: left >= min, left, min }
}

export function zoneRect(r: { left: number; top: number; width: number; height: number }, zone: CompassZone): { left: number; top: number; width: number; height: number } {
  if (zone === 'center') return { ...r }
  const half = zone === 'left' || zone === 'right' ? { ...r, width: r.width / 2 } : { ...r, height: r.height / 2 }
  if (zone === 'right') half.left += half.width
  if (zone === 'bottom') half.top += half.height
  return half
}

export const DOCK_PERCENT = 50
