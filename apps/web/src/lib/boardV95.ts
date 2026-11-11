import type { TerminalSession, ToolSpan } from '@ccm/shared'
import { baseName } from './preview'

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
