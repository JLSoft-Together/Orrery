import type { RailMode } from './railMode'

export const RAIL_COLLAPSED_KEY = 'ccm.next.railCollapsed'

export function parseRailCollapsed(raw: string | null | undefined): boolean {
  return raw === '1'
}

export function readRailCollapsed(): boolean {
  try {
    return parseRailCollapsed(localStorage.getItem(RAIL_COLLAPSED_KEY))
  } catch {
    return false
  }
}

export function writeRailCollapsed(collapsed: boolean): void {
  try {
    if (collapsed) localStorage.setItem(RAIL_COLLAPSED_KEY, '1')
    else localStorage.removeItem(RAIL_COLLAPSED_KEY)
  } catch {
    return
  }
}

export function effectiveRailMode(base: RailMode, collapsed: boolean): RailMode {
  return base === 'full' && collapsed ? 'icon' : base
}

export function canCollapseRail(base: RailMode): boolean {
  return base === 'full'
}

export function isRailToggleKey(e: Pick<KeyboardEvent, 'type' | 'code' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey' | 'isComposing'>, mac: boolean): boolean {
  if (e.type !== 'keydown' || e.isComposing || e.code !== 'KeyB' || !e.shiftKey || e.altKey) return false
  return mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey
}
