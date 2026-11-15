export const EXPLORER_DRAWER_W = 288
export const PIN_MIN_VIEWPORT = 1440
export const PIN_MIN_CANVAS = 720

export interface PinInput {
  viewport: number
  canvas: number
  pinned: boolean
  drawerWidth?: number
}

export type PinVerdict = { ok: true } | { ok: false; reason: 'viewport' | 'canvas' }

export function canvasWithoutDrawer(input: PinInput): number {
  const w = input.drawerWidth ?? EXPLORER_DRAWER_W
  return input.pinned ? input.canvas + w : input.canvas
}

export function pinVerdict(input: PinInput): PinVerdict {
  if (input.viewport < PIN_MIN_VIEWPORT) return { ok: false, reason: 'viewport' }
  const w = input.drawerWidth ?? EXPLORER_DRAWER_W
  if (canvasWithoutDrawer(input) - w < PIN_MIN_CANVAS) return { ok: false, reason: 'canvas' }
  return { ok: true }
}

export function effectivePinned(input: PinInput): boolean {
  return input.pinned && pinVerdict(input).ok
}
