import type { Provider, ToolAccess, ToolSpan } from '@ccm/shared'
import { pathKey } from './changesView'

export const AFTER_TURN_MS = 120_000
export const TURN_FALLBACK_MS = 600_000
export const TURN_SLACK_MS = 5_000
export const MAX_FILES = 2000
export const MAX_AGENTS_PER_FILE = 6

export interface TouchTerminal {
  provider?: Provider
  name: string
  working: boolean
  turnStart?: number
  idleSince?: number
}

export interface AgentTouch {
  terminalId: string
  provider?: Provider
  name: string
  kind: ToolAccess
  at: number
}

export type TouchState = 'edit' | 'read' | 'conflict'

export interface FileTouchInfo {
  state: TouchState
  touches: AgentTouch[]
  editors: number
}

export interface DirTouchInfo {
  state: TouchState
}

export interface TouchIndex {
  files: Map<string, FileTouchInfo>
  dirs: Map<string, DirTouchInfo>
}

const RANK: Record<TouchState, number> = { read: 0, edit: 1, conflict: 2 }

export function isFresh(span: ToolSpan, term: TouchTerminal, now: number): boolean {
  const at = Date.parse(span.start)
  if (Number.isNaN(at)) return false
  if (term.working) return at >= (term.turnStart ?? now - TURN_FALLBACK_MS) - TURN_SLACK_MS
  if (term.idleSince === undefined || now - term.idleSince > AFTER_TURN_MS) return false
  return at >= (term.turnStart ?? term.idleSince - TURN_FALLBACK_MS) - TURN_SLACK_MS
}

function ancestors(key: string): string[] {
  const out: string[] = []
  let i = key.lastIndexOf('/')
  while (i > 0) {
    const dir = key.slice(0, i)
    out.push(dir)
    i = dir.lastIndexOf('/')
  }
  return out
}

export function buildTouchIndex(spans: readonly ToolSpan[], terms: ReadonlyMap<string, TouchTerminal>, now: number, keyOf: (p: string) => string = pathKey): TouchIndex {
  const perFile = new Map<string, Map<string, AgentTouch>>()
  for (const span of spans) {
    if (!span.path || (span.access !== 'edit' && span.access !== 'read')) continue
    const term = terms.get(span.terminalId)
    if (!term || !isFresh(span, term, now)) continue
    const key = keyOf(span.path)
    let byTerm = perFile.get(key)
    if (!byTerm) {
      if (perFile.size >= MAX_FILES) continue
      byTerm = new Map()
      perFile.set(key, byTerm)
    }
    const at = Date.parse(span.start)
    const prev = byTerm.get(span.terminalId)
    if (!prev && byTerm.size >= MAX_AGENTS_PER_FILE) continue
    const kind = prev?.kind === 'edit' || span.access === 'edit' ? 'edit' : 'read'
    const same = prev?.kind === kind
    const when = same ? Math.max(prev.at, at) : kind === span.access ? at : (prev?.at ?? at)
    byTerm.set(span.terminalId, { terminalId: span.terminalId, provider: term.provider, name: term.name, kind, at: when })
  }
  const files = new Map<string, FileTouchInfo>()
  const dirs = new Map<string, DirTouchInfo>()
  for (const [key, byTerm] of perFile) {
    const touches = [...byTerm.values()].sort((a, b) => Number(b.kind === 'edit') - Number(a.kind === 'edit') || b.at - a.at)
    const editors = touches.filter((t) => t.kind === 'edit').length
    const state: TouchState = editors >= 2 ? 'conflict' : editors === 1 ? 'edit' : 'read'
    files.set(key, { state, touches, editors })
    for (const dir of ancestors(key)) {
      const cur = dirs.get(dir)
      if (!cur || RANK[state] > RANK[cur.state]) dirs.set(dir, { state })
    }
  }
  return { files, dirs }
}

export const emptyTouchIndex = (): TouchIndex => ({ files: new Map(), dirs: new Map() })
