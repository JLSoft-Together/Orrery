import { CUSTOM_VAR_KEYS } from './customTheme'
import { MAU_VAR_KEYS, mauTheme } from './mau'

export const RANDOM_PALETTE_VERSION = 1
export const RANDOM_PINS_MAX = 6

export interface RandomPaletteState {
  v: typeof RANDOM_PALETTE_VERSION
  seed: number
  held: boolean
  pins: number[]
  upcoming: number | null
}

const isSeed = (x: unknown): x is number => typeof x === 'number' && Number.isInteger(x) && x >= 0 && x <= 0xffffffff

export function parseRandomPalette(raw: string | null): RandomPaletteState | null {
  if (!raw) return null
  let data: unknown
  try {
    data = JSON.parse(raw)
  } catch {
    return null
  }
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null
  const o = data as Record<string, unknown>
  if (o.v !== RANDOM_PALETTE_VERSION || !isSeed(o.seed)) return null
  const pins = Array.isArray(o.pins) ? [...new Set(o.pins.filter(isSeed))].slice(0, RANDOM_PINS_MAX) : []
  return { v: RANDOM_PALETTE_VERSION, seed: o.seed, held: o.held === true, pins, upcoming: isSeed(o.upcoming) ? o.upcoming : null }
}

export function serializeRandomPalette(state: RandomPaletteState): string {
  const now = mauTheme(state.seed).css
  const upcoming = state.upcoming === null ? null : mauTheme(state.upcoming).css
  return JSON.stringify({ ...state, css: now, upcomingCss: upcoming })
}

export function freshRandomPalette(nextSeed: () => number): RandomPaletteState {
  return { v: RANDOM_PALETTE_VERSION, seed: nextSeed(), held: false, pins: [], upcoming: nextSeed() }
}

export function launchRandomPalette(stored: RandomPaletteState | null, nextSeed: () => number): RandomPaletteState {
  if (!stored) return freshRandomPalette(nextSeed)
  if (stored.held) return { ...stored, upcoming: stored.upcoming ?? nextSeed() }
  return { ...stored, seed: stored.upcoming ?? nextSeed(), upcoming: nextSeed() }
}

export function reshuffleRandomPalette(state: RandomPaletteState, nextSeed: () => number): RandomPaletteState {
  return { ...state, seed: nextSeed(), held: false, upcoming: nextSeed() }
}

export function holdRandomPalette(state: RandomPaletteState): RandomPaletteState {
  return { ...state, held: true, pins: [state.seed, ...state.pins.filter((p) => p !== state.seed)].slice(0, RANDOM_PINS_MAX) }
}

export function pickRandomPin(state: RandomPaletteState, seed: number): RandomPaletteState {
  return state.pins.includes(seed) ? { ...state, seed, held: true } : state
}

export function dropRandomPin(state: RandomPaletteState, seed: number): RandomPaletteState {
  const pins = state.pins.filter((p) => p !== seed)
  return { ...state, pins, held: state.held && pins.includes(state.seed) }
}

const INLINE_KEYS: readonly string[] = [...new Set([...CUSTOM_VAR_KEYS, ...MAU_VAR_KEYS])]

export function applyInlineVars(style: CSSStyleDeclaration, vars: Readonly<Record<string, string>> | null, reducedGlass: boolean): void {
  for (const key of INLINE_KEYS) style.removeProperty(key)
  if (!vars) return
  for (const [key, value] of Object.entries(vars)) {
    if (key === '--ccm-glass' && reducedGlass) continue
    style.setProperty(key, value)
  }
}
