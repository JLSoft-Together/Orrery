import type { Agent, Provider, TerminalSession } from '@ccm/shared'
import { groupSessions, type SessionGroup } from './sessionGroups'

export interface CommandRow {
  agentId: string
  terminalId: string
  role: Agent['role']
  provider: Provider
  command: string
  tool: string
  startedAt: string | null
  startedMs: number | null
  waiting: boolean
}

const parse = (iso: string | undefined): number | null => {
  if (!iso) return null
  const n = Date.parse(iso)
  return Number.isFinite(n) ? n : null
}

export function runningCommands(agents: readonly Agent[], terminalOf: (id: string) => Pick<TerminalSession, 'status' | 'provider'> | undefined): CommandRow[] {
  const rows: CommandRow[] = []
  for (const a of agents) {
    if (!a.currentTool) continue
    if (a.status !== 'working' && a.status !== 'waiting') continue
    const term = terminalOf(a.terminalId)
    if (!term || term.status === 'stale') continue
    const startedMs = parse(a.currentToolStartedAt)
    rows.push({
      agentId: a.id,
      terminalId: a.terminalId,
      role: a.role,
      provider: a.provider ?? term.provider ?? 'claude',
      command: a.currentToolSummary || a.currentTool,
      tool: a.currentTool,
      startedAt: startedMs === null ? null : (a.currentToolStartedAt ?? null),
      startedMs,
      waiting: a.status === 'waiting' || (a.role === 'main' && term.status === 'waiting'),
    })
  }
  return rows.sort((x, y) => {
    if (x.startedMs === null && y.startedMs !== null) return 1
    if (y.startedMs === null && x.startedMs !== null) return -1
    return (x.startedMs ?? 0) - (y.startedMs ?? 0) || x.agentId.localeCompare(y.agentId)
  })
}

export function longestCommand(rows: readonly CommandRow[]): CommandRow | undefined {
  return rows.find((r) => r.startedMs !== null) ?? rows[0]
}

export function sameCommands(a: readonly CommandRow[] | undefined, b: readonly CommandRow[]): boolean {
  return (
    !!a &&
    a.length === b.length &&
    a.every((x, i) => {
      const y = b[i]!
      return x.agentId === y.agentId && x.command === y.command && x.tool === y.tool && x.startedMs === y.startedMs && x.waiting === y.waiting && x.terminalId === y.terminalId
    })
  )
}

function startOfDay(ms: number): number {
  const d = new Date(ms)
  d.setHours(0, 0, 0, 0)
  return d.getTime()
}

export function endedToday(s: Pick<TerminalSession, 'status' | 'endedAt' | 'lastActivityAt'>, nowMs: number): boolean {
  if (s.status !== 'stale') return true
  const at = parse(s.endedAt) ?? parse(s.lastActivityAt)
  return at === null || at >= startOfDay(nowMs)
}

export function monitorGroups(sorted: readonly Pick<TerminalSession, 'id' | 'status' | 'endedAt' | 'lastActivityAt'>[], nowMs: number): SessionGroup[] {
  return groupSessions(sorted.filter((s) => endedToday(s, nowMs)))
}

export const MONITOR_COLUMNS = ['needs', 'running', 'idle', 'ended'] as const

export function countOf(groups: readonly SessionGroup[], key: SessionGroup['key']): number {
  return groups.find((g) => g.key === key)?.ids.length ?? 0
}

export function frozenSort<T extends { id: string }>(list: readonly T[], order: readonly string[]): T[] {
  const pos = new Map(order.map((id, i) => [id, i]))
  return [...list].sort((a, b) => (pos.get(a.id) ?? Number.MAX_SAFE_INTEGER) - (pos.get(b.id) ?? Number.MAX_SAFE_INTEGER) || a.id.localeCompare(b.id))
}
