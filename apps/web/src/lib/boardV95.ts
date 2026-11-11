import type { Provider, TerminalSession, ToolSpan } from '@ccm/shared'
import { dirKey } from '../stores/favorites'
import { baseName } from './preview'
import { NO_PROJECT } from './projectGroups'
import { folderName, projectOf } from './sidebarRows'
import { GROUP_ORDER, groupOf } from './sessionGroups'

export const DONE_VISIBLE = 3

export type BoardColumnKey = 'needs' | 'running' | 'idle' | 'ended'

export interface FileAction {
  access: 'edit' | 'read'
  file: string
}

export function fileAction(spans: Iterable<ToolSpan>, agentId: string | undefined): FileAction | null {
  if (!agentId) return null
  let best: ToolSpan | null = null
  let bestAt = -Infinity
  for (const s of spans) {
    if (s.agentId !== agentId || s.end || !s.path || !s.access) continue
    const at = Date.parse(s.start)
    const rank = Number.isFinite(at) ? at : 0
    if (rank >= bestAt) {
      best = s
      bestAt = rank
    }
  }
  if (!best?.path || !best.access) return null
  const file = baseName(best.path)
  return file ? { access: best.access, file } : null
}

export function isStrip(count: number, expanded: boolean): boolean {
  return count === 0 && !expanded
}

export function columnTemplate(strips: readonly boolean[], stripPx = 40): string {
  return strips.map((s) => (s ? `${stripPx}px` : 'minmax(0, 1fr)')).join(' ')
}

export function clockLabel(ms: number): string {
  const d = new Date(ms)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

export interface DoneRow {
  id: string
  at: number
  time: string
  title: string
  project: string
  files: number | null
}

type DoneSource = Pick<TerminalSession, 'id' | 'endedAt' | 'lastActivityAt' | 'diff'>

export function doneRow(t: DoneSource, title: string, project: string): DoneRow | null {
  const at = Date.parse(t.endedAt ?? t.lastActivityAt ?? '')
  if (!Number.isFinite(at)) return null
  const files = t.diff && t.diff.files > 0 ? t.diff.files : null
  return { id: t.id, at, time: clockLabel(at), title, project, files }
}

export function sortDone(rows: readonly DoneRow[]): DoneRow[] {
  return [...rows].sort((a, b) => b.at - a.at || a.id.localeCompare(b.id))
}

export function splitDone<T>(rows: readonly T[], expanded: boolean, limit = DONE_VISIBLE): { shown: T[]; more: number } {
  if (expanded || rows.length <= limit) return { shown: [...rows], more: 0 }
  return { shown: rows.slice(0, limit), more: rows.length - limit }
}

export type BoardGroupBy = 'status' | 'project' | 'agent'
export const BOARD_GROUP_BY: readonly BoardGroupBy[] = ['status', 'project', 'agent']

export function parseGroupBy(raw: string | null | undefined): BoardGroupBy {
  return raw === 'project' || raw === 'agent' ? raw : 'status'
}

export interface BoardLane {
  key: string
  name: string
  provider: Provider | null
  active: string[]
  ended: string[]
  needs: number
  latest: number
}

type LaneSource = Pick<TerminalSession, 'id' | 'status' | 'cwd' | 'provider' | 'lastActivityAt'>

const PROVIDER_ORDER: Provider[] = ['claude', 'codex', 'deepseek']

export function laneSessions(rows: readonly LaneSource[], by: 'project' | 'agent'): BoardLane[] {
  const lanes = new Map<string, BoardLane>()
  const rank = new Map<string, number>()
  for (const s of rows) {
    const provider: Provider = s.provider ?? 'claude'
    let key: string
    let name = ''
    if (by === 'agent') key = provider
    else {
      const dir = projectOf(s.cwd)
      key = dir ? dirKey(dir) : NO_PROJECT
      name = dir ? folderName(dir) : ''
    }
    let lane = lanes.get(key)
    if (!lane) {
      lane = { key, name, provider: by === 'agent' ? provider : null, active: [], ended: [], needs: 0, latest: 0 }
      lanes.set(key, lane)
    }
    const group = groupOf(s.status)
    rank.set(s.id, GROUP_ORDER.indexOf(group))
    if (group === 'ended') lane.ended.push(s.id)
    else lane.active.push(s.id)
    if (group === 'needs') lane.needs++
    const at = Date.parse(s.lastActivityAt ?? '')
    if (Number.isFinite(at) && at > lane.latest) lane.latest = at
  }
  const out = [...lanes.values()]
  for (const lane of out) lane.active.sort((a, b) => rank.get(a)! - rank.get(b)!)
  if (by === 'agent') return out.sort((a, b) => PROVIDER_ORDER.indexOf(a.provider!) - PROVIDER_ORDER.indexOf(b.provider!))
  return out.sort(
    (a, b) => Number(b.needs > 0) - Number(a.needs > 0) || b.latest - a.latest || a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
  )
}
