import type { Agent, AgentStatus } from '@ccm/shared'
import type { CheckpointFile } from './preview'

export const ORBIT_MAX_DOTS = 5

export interface OrbitDot {
  id: string
  status: AgentStatus
  angle: number
}

export interface OrbitEntry {
  id: string
  name: string
  type: string
  tool: string | null
  since: string | undefined
  status: AgentStatus
}

type SubLike = Pick<Agent, 'id' | 'role' | 'status' | 'name' | 'type' | 'currentTool' | 'currentToolStartedAt' | 'startedAt'>

const isLive = (a: SubLike): boolean => a.role === 'subagent' && (a.status === 'working' || a.status === 'waiting')

export function liveSubagents<T extends SubLike>(agents: readonly T[] | undefined): T[] {
  return agents?.filter(isLive) ?? []
}

export function orbitEntries(agents: readonly SubLike[] | undefined): OrbitEntry[] {
  return liveSubagents(agents).map((a) => ({
    id: a.id,
    name: a.name ?? '',
    type: a.type ?? '',
    tool: a.currentTool ?? null,
    since: a.currentTool ? (a.currentToolStartedAt ?? a.startedAt) : a.startedAt,
    status: a.status,
  }))
}

export function orbitDots(entries: readonly Pick<OrbitEntry, 'id' | 'status'>[]): OrbitDot[] {
  const shown = entries.slice(0, ORBIT_MAX_DOTS)
  return shown.map((e, i) => ({ id: e.id, status: e.status, angle: Math.round((360 / shown.length) * i) }))
}

export function orbitTone(entries: readonly Pick<OrbitEntry, 'status'>[]): 'working' | 'waiting' {
  return entries.some((e) => e.status === 'working') ? 'working' : 'waiting'
}

export type MeterTone = 'ok' | 'warn' | 'over'

export interface MeterState {
  ratio: number
  fill: number
  now: number
  max: number
  tone: MeterTone
}

export const METER_WARN_AT = 0.8

export function meterState(tokens: number | undefined, limit: number | undefined): MeterState | null {
  if (tokens === undefined || limit === undefined) return null
  if (!Number.isFinite(tokens) || !Number.isFinite(limit) || tokens <= 0 || limit <= 0) return null
  const ratio = tokens / limit
  return { ratio, fill: Math.min(1, ratio), now: Math.round(tokens), max: Math.round(limit), tone: ratio > 1 ? 'over' : ratio >= METER_WARN_AT ? 'warn' : 'ok' }
}

export interface TurnSummary {
  files: number
  added: number
  removed: number
  top: CheckpointFile[]
  more: number
}

export const TURN_TOP = 3

export function turnSummary(files: readonly CheckpointFile[]): TurnSummary | null {
  if (!files.length) return null
  let added = 0
  let removed = 0
  for (const f of files) {
    added += f.added ?? 0
    removed += f.removed ?? 0
  }
  const top = files.slice(0, TURN_TOP)
  return { files: files.length, added, removed, top: [...top], more: files.length - top.length }
}

export function newestTurnAfter<T extends { id: string; at: number; label: string }>(list: readonly T[], sinceMs: number, graceMs = 2000): T | null {
  if (sinceMs <= 0) return null
  return list.find((c) => c.label.startsWith('after:') && c.at >= sinceMs - graceMs) ?? null
}
