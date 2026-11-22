import type { ToolSpan } from '@ccm/shared'
import { TURN_FALLBACK_MS, TURN_SLACK_MS } from './agentTouches'
import { absoluteIn, pathKey } from '../../git/lib/changesView'
import type { ChangeEntry } from '../../../core/lib/preview'

export const MAX_TURN_PATHS = 2000
export const UNKNOWN_FILTER = '~unknown'
export const TURN_EDITS_KEY = 'ccm.turnEdits'

export interface TurnEdits {
  terminalId: string
  name: string
  turnStart: number
  turnEnd?: number
  paths: Set<string>
}

export const turnStartOf = (turnStart: number | undefined, at: number): number => turnStart ?? at - TURN_FALLBACK_MS

export function editPaths(spans: readonly ToolSpan[], from: number, to?: number, keyOf: (p: string) => string = pathKey): Set<string> {
  const out = new Set<string>()
  for (const s of spans) {
    if (s.access !== 'edit' || !s.path) continue
    const at = Date.parse(s.start)
    if (Number.isNaN(at) || at < from - TURN_SLACK_MS) continue
    if (to !== undefined && at > to + TURN_SLACK_MS) continue
    out.add(keyOf(s.path))
  }
  return out
}

export function addPaths(turn: TurnEdits, paths: Iterable<string>, max = MAX_TURN_PATHS): TurnEdits {
  let next: Set<string> | null = null
  for (const p of paths) {
    if ((next ?? turn.paths).has(p) || (next ?? turn.paths).size >= max) continue
    next ??= new Set(turn.paths)
    next.add(p)
  }
  return next ? { ...turn, paths: next } : turn
}

export function mergeTurn(old: TurnEdits | undefined, next: TurnEdits, max = MAX_TURN_PATHS): TurnEdits {
  if (!old) return next
  const merged = addPaths({ ...old, name: next.name || old.name }, next.paths, max)
  return { ...merged, turnStart: Math.min(old.turnStart, next.turnStart), turnEnd: Math.max(old.turnEnd ?? 0, next.turnEnd ?? 0) || undefined }
}

export interface SessionGroup {
  terminalId: string
  name: string
  entries: ChangeEntry[]
}

export interface TurnSplit {
  sessions: SessionGroup[]
  unknown: ChangeEntry[]
  owners: Map<string, string[]>
}

export function splitBySession(root: string, entries: readonly ChangeEntry[], turns: readonly TurnEdits[], keyOf: (p: string) => string = pathKey): TurnSplit {
  const groups = new Map<string, SessionGroup>()
  const owners = new Map<string, string[]>()
  const unknown: ChangeEntry[] = []
  for (const e of entries) {
    const keys = [keyOf(absoluteIn(root, e.path)), ...(e.origPath ? [keyOf(absoluteIn(root, e.origPath))] : [])]
    const ids: string[] = []
    for (const turn of turns) {
      if (!keys.some((k) => turn.paths.has(k))) continue
      ids.push(turn.terminalId)
      let g = groups.get(turn.terminalId)
      if (!g) {
        g = { terminalId: turn.terminalId, name: turn.name, entries: [] }
        groups.set(turn.terminalId, g)
      }
      g.entries.push(e)
    }
    if (ids.length) owners.set(e.path, ids)
    else unknown.push(e)
  }
  return { sessions: [...groups.values()], unknown, owners }
}

export function filterEntries(split: TurnSplit, entries: readonly ChangeEntry[], filter: string | null | undefined): ChangeEntry[] {
  if (!filter) return [...entries]
  if (filter === UNKNOWN_FILTER) return split.unknown
  return split.sessions.find((s) => s.terminalId === filter)?.entries ?? [...entries]
}

export function validFilter(split: TurnSplit, filter: string | null | undefined): string | null {
  if (!filter) return null
  if (filter === UNKNOWN_FILTER) return split.unknown.length ? filter : null
  return split.sessions.some((s) => s.terminalId === filter) ? filter : null
}

interface SavedTurn {
  terminalId: string
  name: string
  turnStart: number
  turnEnd?: number
  paths: string[]
}

export interface SavedTurns {
  current: TurnEdits[]
  last: TurnEdits[]
}

const toSaved = (t: TurnEdits): SavedTurn => ({ terminalId: t.terminalId, name: t.name, turnStart: t.turnStart, turnEnd: t.turnEnd, paths: [...t.paths] })

export const serializeTurns = (current: Iterable<TurnEdits>, last: Iterable<TurnEdits>): string => JSON.stringify({ v: 1, current: [...current].map(toSaved), last: [...last].map(toSaved) })

function fromSaved(x: unknown): TurnEdits | null {
  if (!x || typeof x !== 'object') return null
  const s = x as Partial<SavedTurn>
  if (typeof s.terminalId !== 'string' || typeof s.turnStart !== 'number' || !Array.isArray(s.paths)) return null
  const paths = new Set(s.paths.filter((p): p is string => typeof p === 'string').slice(0, MAX_TURN_PATHS))
  return { terminalId: s.terminalId, name: typeof s.name === 'string' ? s.name : '', turnStart: s.turnStart, turnEnd: typeof s.turnEnd === 'number' ? s.turnEnd : undefined, paths }
}

export function parseTurns(raw: string | null): SavedTurns {
  try {
    const data = raw ? (JSON.parse(raw) as { v?: number; current?: unknown[]; last?: unknown[] }) : null
    if (!data || data.v !== 1) return { current: [], last: [] }
    const list = (xs: unknown): TurnEdits[] => (Array.isArray(xs) ? xs.flatMap((x) => fromSaved(x) ?? []) : [])
    return { current: list(data.current), last: list(data.last) }
  } catch {
    return { current: [], last: [] }
  }
}
