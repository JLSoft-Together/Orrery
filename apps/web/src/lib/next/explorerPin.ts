export const EXPLORER_DRAWER_W = 288
export const PIN_MIN_CANVAS = 560

export const EXPLORER_MIN_W = 220
export const EXPLORER_MAX_W = 640
export const EXPLORER_PIN_GAP = 8

export function clampExplorerWidth(width: number, room?: number): number {
  const ceiling = room === undefined ? EXPLORER_MAX_W : Math.max(EXPLORER_MIN_W, Math.min(EXPLORER_MAX_W, room))
  if (!Number.isFinite(width)) return EXPLORER_DRAWER_W
  return Math.round(Math.min(ceiling, Math.max(EXPLORER_MIN_W, width)))
}

export function parseExplorerWidth(raw: string | null): number {
  if (raw === null || raw.trim() === '') return EXPLORER_DRAWER_W
  return clampExplorerWidth(Number(raw))
}

export function pinnedRoom(canvasFree: number): number {
  return canvasFree - PIN_MIN_CANVAS - EXPLORER_PIN_GAP
}

export function dragWidth(startWidth: number, startX: number, x: number, room?: number): number {
  return clampExplorerWidth(startWidth + (startX - x), room)
}

export function fitPinnedWidth(canvasTotal: number, desired: number): number | null {
  const room = pinnedRoom(canvasTotal)
  if (room < EXPLORER_MIN_W) return null
  return clampExplorerWidth(desired, room)
}
