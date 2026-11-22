export interface TermSize {
  cols: number
  rows: number
}

export const BASE_FONT = 13
export const MIN_FONT = 9
export const CELL_RATIO = 0.602

export function deskFont(width: number, cols: number, base = BASE_FONT, min = MIN_FONT): number {
  if (cols < 1 || width <= 0) return base
  return Math.max(min, Math.min(base, Math.floor(width / (cols * CELL_RATIO))))
}

export function fitScale(natural: number, available: number): number {
  if (natural <= 0 || available <= 0 || natural <= available) return 1
  return Math.max(0.05, Math.floor((available / natural) * 1000) / 1000)
}


export interface Box {
  w: number
  h: number
}

export interface Point {
  x: number
  y: number
}

export interface ZoomView {
  zoom: number
  x: number
  y: number
}

export const ZOOM_READ = 1.5
export const ZOOM_MAX = 6
export const IDENTITY: ZoomView = { zoom: 1, x: 0, y: 0 }

export function maxZoom(base: number): number {
  if (base <= 0) return 1
  return Math.max(1, Math.min(ZOOM_MAX, ZOOM_READ / base))
}

function clampAxis(value: number, content: number, box: number): number {
  if (content <= box) return 0
  return Math.min(0, Math.max(box - content, value))
}

export function clampView(view: ZoomView, base: number, content: Box, box: Box): ZoomView {
  const zoom = Math.min(maxZoom(base), Math.max(1, view.zoom))
  const s = base * zoom
  return { zoom, x: clampAxis(view.x, content.w * s, box.w), y: zoom === 1 ? 0 : clampAxis(view.y, content.h * s, box.h) }
}

export function pinchView(start: ZoomView, from: { dist: number; mid: Point }, to: { dist: number; mid: Point }, base: number, content: Box, box: Box): ZoomView {
  if (from.dist <= 0) return start
  const s0 = base * start.zoom
  const px = (from.mid.x - start.x) / s0
  const py = (from.mid.y - start.y) / s0
  const zoom = Math.min(maxZoom(base), Math.max(1, start.zoom * (to.dist / from.dist)))
  const s = base * zoom
  return clampView({ zoom, x: to.mid.x - px * s, y: to.mid.y - py * s }, base, content, box)
}

export function panView(start: ZoomView, delta: Point, base: number, content: Box, box: Box): ZoomView {
  return clampView({ zoom: start.zoom, x: start.x + delta.x, y: start.y + delta.y }, base, content, box)
}
