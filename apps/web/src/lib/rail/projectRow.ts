import type { RailCluster } from './railModel'

export const PROJECT_TONES = 8

export function projectTone(key: string): number {
  let h = 0
  for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0
  return h % PROJECT_TONES
}

export function monogram(name: string): string {
  const ch = [...name.trim()].find((c) => /[\p{L}\p{N}]/u.test(c))
  return ch ? ch.toUpperCase() : '?'
}

export function runCounts(cluster: Pick<RailCluster, 'checkouts'>): { running: number; waiting: number } {
  let running = 0
  let waiting = 0
  for (const k of cluster.checkouts) {
    for (const r of k.rows) {
      if (r.state === 'working' || r.state === 'subagents') running++
      else if (r.state === 'asked') waiting++
    }
  }
  return { running, waiting }
}

export function lastActive(cluster: Pick<RailCluster, 'checkouts'>): number | null {
  let best = 0
  for (const k of cluster.checkouts) for (const r of k.rows) if (r.since > best) best = r.since
  return best > 0 ? best : null
}

export type Ago = { key: 'now' | 'minutes' | 'hours' | 'yesterday' | 'days'; n?: number }

export function agoOf(at: number, now: number): Ago {
  const sec = Math.max(0, Math.round((now - at) / 1000))
  if (sec < 60) return { key: 'now' }
  if (sec < 3600) return { key: 'minutes', n: Math.floor(sec / 60) }
  if (sec < 86400) return { key: 'hours', n: Math.floor(sec / 3600) }
  const days = Math.floor(sec / 86400)
  return days === 1 ? { key: 'yesterday' } : { key: 'days', n: days }
}
