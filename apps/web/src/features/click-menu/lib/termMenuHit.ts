import type { TermMenu, TermMenuItem } from './termMenu'

export interface ScreenBox {
  left: number
  top: number
  width: number
  height: number
}

export function rowAt(box: ScreenBox, rows: number, clientX: number, clientY: number): number | null {
  if (rows <= 0 || box.width <= 0 || box.height <= 0) return null
  if (clientX < box.left || clientX >= box.left + box.width) return null
  if (clientY < box.top || clientY >= box.top + box.height) return null
  return Math.min(rows - 1, Math.floor(((clientY - box.top) / box.height) * rows))
}

export function itemAtRow(menu: TermMenu | null, row: number | null): TermMenuItem | null {
  if (!menu || row === null) return null
  return menu.items.find((i) => i.row === row) ?? null
}

export function rowBox(box: ScreenBox, rows: number, row: number): ScreenBox {
  const h = box.height / rows
  return { left: box.left, top: box.top + row * h, width: box.width, height: h }
}

export interface MouseGate {
  button: number
  shiftKey: boolean
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
}

export function isPlainClick(e: MouseGate): boolean {
  return e.button === 0 && !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey
}
