export type RailMode = 'full' | 'icon' | 'overlay'

export const RAIL_FULL_FROM = 1200
export const RAIL_ICON_FROM = 1024
export const RAIL_WIDE_FROM = 1440
export const RAIL_HYSTERESIS = 24
export const RAIL_ICON_W = 56
export const RAIL_OVERLAY_W = 248
export const RAIL_MIN = 224
export const RAIL_MAX = 360
export const RAIL_WIDTH_KEY = 'ccm.next.railWidth'

export function railModeFor(viewport: number): RailMode {
  if (viewport >= RAIL_FULL_FROM) return 'full'
  if (viewport >= RAIL_ICON_FROM) return 'icon'
  return 'overlay'
}

export function nextRailMode(prev: RailMode, viewport: number): RailMode {
  if (!Number.isFinite(viewport) || viewport <= 0) return prev
  let mode = prev
  for (let i = 0; i < 2; i++) {
    if (mode === 'overlay' && viewport >= RAIL_ICON_FROM + RAIL_HYSTERESIS) mode = 'icon'
    else if (mode === 'icon' && viewport >= RAIL_FULL_FROM + RAIL_HYSTERESIS) mode = 'full'
    else if (mode === 'full' && viewport < RAIL_FULL_FROM - RAIL_HYSTERESIS) mode = 'icon'
    else if (mode === 'icon' && viewport < RAIL_ICON_FROM - RAIL_HYSTERESIS) mode = 'overlay'
    else break
  }
  return mode
}

export function defaultRailWidth(viewport: number): number {
  return viewport >= RAIL_WIDE_FROM ? 264 : 248
}

export function clampRailWidth(px: number): number {
  if (!Number.isFinite(px)) return RAIL_MIN
  return Math.round(Math.min(RAIL_MAX, Math.max(RAIL_MIN, px)))
}

export function parseRailWidth(raw: string | null | undefined): number | null {
  if (!raw) return null
  const n = Number(raw)
  return Number.isFinite(n) && n > 0 ? clampRailWidth(n) : null
}

export function railWidth(mode: RailMode, viewport: number, custom: number | null): number {
  if (mode === 'icon') return RAIL_ICON_W
  if (mode === 'overlay') return RAIL_OVERLAY_W
  return custom === null ? defaultRailWidth(viewport) : clampRailWidth(custom)
}

export function readRailWidth(): number | null {
  try {
    return parseRailWidth(localStorage.getItem(RAIL_WIDTH_KEY))
  } catch {
    return null
  }
}

export function writeRailWidth(px: number | null): void {
  try {
    if (px === null) localStorage.removeItem(RAIL_WIDTH_KEY)
    else localStorage.setItem(RAIL_WIDTH_KEY, String(clampRailWidth(px)))
  } catch {
    return
  }
}
