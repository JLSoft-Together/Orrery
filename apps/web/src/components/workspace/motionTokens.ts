import { motionMs } from '../../lib/motion'

export type DurToken = 'press' | 'fast' | 'base' | 'slow' | 'emph'
export type EaseToken = 'out-quint' | 'spring' | 'exit'

const DUR_FALLBACK: Record<DurToken, number> = { press: 100, fast: 150, base: 220, slow: 300, emph: 520 }
const tokens = new Map<string, string>()

function token(name: string): string {
  const known = tokens.get(name)
  if (known !== undefined) return known
  const value = typeof document === 'undefined' ? '' : getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  if (value) tokens.set(name, value)
  return value
}

function parseDuration(value: string): number | null {
  const m = /^(\d*\.?\d+)(ms|s)$/.exec(value.trim())
  return m ? Number(m[1]) * (m[2] === 's' ? 1000 : 1) : null
}

export function ease(name: EaseToken): string {
  return token(`--ease-${name}`) || 'ease-out'
}

export function dur(name: DurToken, factor = 1): number {
  return motionMs(Math.round((parseDuration(token(`--ccm-dur-${name}`)) ?? DUR_FALLBACK[name]) * factor))
}
