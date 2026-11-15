export type ChipTone = 'working' | 'waiting' | 'error' | 'done' | 'idle' | 'meta' | 'accent' | 'warning'

const BY_STATUS: Record<string, ChipTone> = {
  working: 'working',
  waiting: 'waiting',
  error: 'error',
  completed: 'done',
  idle: 'idle',
  unknown: 'idle',
  cancelled: 'meta',
  stale: 'meta',
}

export function chipTone(status: string | undefined): ChipTone {
  return BY_STATUS[status ?? ''] ?? 'idle'
}

export const GAUGE_HIGH = 65

export function gaugeLevel(pct: number): 'ok' | 'high' {
  return pct >= GAUGE_HIGH ? 'high' : 'ok'
}
