import type { RailState } from '../rail/railModel'
import type { LiveRow } from './railLive'

export type RowStatus = 'working' | 'waiting' | 'idle' | 'error'
export type ElapsedUnit = 's' | 'm' | 'h' | 'd'

export function rowStatusOf(state: RailState): RowStatus {
  switch (state) {
    case 'failed':
      return 'error'
    case 'asked':
      return 'waiting'
    case 'working':
    case 'subagents':
    case 'starting':
      return 'working'
    default:
      return 'idle'
  }
}

export function elapsedParts(ms: number): { unit: ElapsedUnit; n: number } | null {
  const s = Math.floor(Math.max(0, ms) / 1000)
  if (s < 1) return null
  if (s < 60) return { unit: 's', n: s }
  const m = Math.floor(s / 60)
  if (m < 60) return { unit: 'm', n: m }
  const h = Math.floor(m / 60)
  if (h < 24) return { unit: 'h', n: h }
  return { unit: 'd', n: Math.floor(h / 24) }
}

export function rowToolLine(r: Pick<LiveRow, 'state' | 'tool' | 'waitText' | 'stateText'>): string | null {
  if (r.state === 'working') return r.tool
  if (r.state === 'asked') return r.waitText ?? r.stateText
  if (r.state === 'failed') return r.stateText
  return null
}
