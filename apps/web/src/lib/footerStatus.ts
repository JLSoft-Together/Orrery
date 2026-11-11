import type { GitDiffStat, ServerRes } from '@ccm/shared'
import type { RailState } from './rail/railModel'

export interface FooterPane {
  key: string
  state: RailState | null
}

export interface FooterCounts {
  running: number
  waiting: number
  firstRunning: string | null
  firstWaiting: string | null
}

export const PORTS_MAX = 4

export function footerCounts(panes: readonly FooterPane[]): FooterCounts {
  const out: FooterCounts = { running: 0, waiting: 0, firstRunning: null, firstWaiting: null }
  for (const p of panes) {
    if (p.state === 'working' || p.state === 'subagents') {
      out.running++
      out.firstRunning ??= p.key
    } else if (p.state === 'asked') {
      out.waiting++
      out.firstWaiting ??= p.key
    }
  }
  return out
}

const normDir = (dir: string): string => dir.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

export function devPorts(servers: readonly Pick<ServerRes, 'kind' | 'cwd' | 'sessionId' | 'ports'>[], sessionIds: ReadonlySet<string>, dirs: readonly string[]): number[] {
  const wanted = new Set(dirs.filter(Boolean).map(normDir))
  const found = new Set<number>()
  for (const s of servers) {
    if (s.kind !== 'dev-server') continue
    const mine = (s.sessionId !== null && sessionIds.has(s.sessionId)) || (!!s.cwd && wanted.has(normDir(s.cwd)))
    if (!mine) continue
    for (const p of s.ports) if (Number.isInteger(p.port) && p.port > 0) found.add(p.port)
  }
  return [...found].sort((a, b) => a - b)
}

export function portsLabel(ports: readonly number[], max = PORTS_MAX): string {
  const shown = ports.slice(0, max).join(' · ')
  return ports.length > max ? `${shown} +${ports.length - max}` : shown
}

export function changedCount(diff: Pick<GitDiffStat, 'files' | 'untracked'> | undefined): number | null {
  if (!diff) return null
  const n = (Number.isFinite(diff.files) ? diff.files : 0) + (Number.isFinite(diff.untracked) ? diff.untracked : 0)
  return n >= 0 ? n : null
}

export function contextPair(tokens: number | undefined, limit: number | undefined, fmt: (n: number) => string): { used: string; limit: string | null } | null {
  if (tokens === undefined || !Number.isFinite(tokens) || tokens <= 0) return null
  const hasLimit = limit !== undefined && Number.isFinite(limit) && limit > 0
  return { used: fmt(tokens), limit: hasLimit ? fmt(limit) : null }
}
