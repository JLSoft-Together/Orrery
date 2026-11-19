import { isNextPaletteId, NEXT_PALETTE_DEFAULT, nextSchemeFor, type NextPaletteId } from '@ccm/mist-tokens'

export type UiGeneration = 'classic' | 'next'
export type NextMotion = 'full' | 'reduced' | 'off'
export type NextGlass = 'full' | 'reduced'
export type RailGroup = 'project' | 'status'

export const UI_GENERATION_KEY = 'ccm.uiGeneration'
export const UI_MIGRATION_KEY = 'ccm.uiMigration'
export const UI_MIGRATION = '4.1-next'
export const NEXT_KEYS = {
  theme: 'ccm.nextTheme',
  railGroup: 'ccm.next.railGroup',
  explorerPinned: 'ccm.next.explorerPinned',
  explorerWidth: 'ccm.next.explorerWidth',
  reduceGlass: 'ccm.next.reduceGlass',
  clearTerm: 'ccm.next.clearTerm',
  motion: 'ccm.next.motion',
  custom: 'ccm.next.custom',
} as const

export const isUiGeneration = (v: unknown): v is UiGeneration => v === 'classic' || v === 'next'
export const isNextMotion = (v: unknown): v is NextMotion => v === 'full' || v === 'reduced' || v === 'off'
export const isRailGroup = (v: unknown): v is RailGroup => v === 'project' || v === 'status'

export const parseUiGeneration = (v: string | null): UiGeneration => (v === 'classic' ? 'classic' : 'next')
export const parseNextTheme = (v: string | null): NextPaletteId => (isNextPaletteId(v) ? v : NEXT_PALETTE_DEFAULT)

export function startUi(stored: string | null, migration: string | null): { ui: UiGeneration; migrate: boolean } {
  if (migration !== UI_MIGRATION) return { ui: 'next', migrate: true }
  return { ui: parseUiGeneration(stored), migrate: false }
}

export function effectiveMotion(stored: string | null, prefersReduced: boolean): NextMotion {
  return isNextMotion(stored) ? stored : prefersReduced ? 'reduced' : 'full'
}

export function effectiveGlass(reduceGlass: boolean, prefersReducedTransparency: boolean): NextGlass {
  return reduceGlass || prefersReducedTransparency ? 'reduced' : 'full'
}

export interface NextStart {
  ui: UiGeneration
  theme: NextPaletteId
  dark: boolean
  glass: NextGlass
  motion: NextMotion
}

export function startNext(ui: string | null, theme: string | null, classicDark: boolean, reduceGlass: string | null, reducedTransparency: boolean, motion: string | null, reducedMotion: boolean): NextStart {
  const gen = parseUiGeneration(ui)
  const id = parseNextTheme(theme)
  const dark = gen === 'next' ? nextSchemeFor(id, classicDark ? 'dark' : 'light') === 'dark' : classicDark
  return { ui: gen, theme: id, dark, glass: effectiveGlass(reduceGlass === '1', reducedTransparency), motion: effectiveMotion(motion, reducedMotion) }
}
