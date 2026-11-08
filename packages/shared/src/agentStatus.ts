/** Workflow status convention: an agent workflow writes `<folder>/.orrery/status.json`, Orrery shows it on the pane (docs §11ap). */
export const AGENT_STATUS_DIR = '.orrery'
export const AGENT_STATUS_FILE = 'status.json'
export const AGENT_STATUS_MAX_BYTES = 4096
export const AGENT_STATUS_PHASE_MAX = 40
export const AGENT_STATUS_NOTE_MAX = 80
/** Not modified for this long: shown dimmed. */
export const AGENT_STATUS_STALE_MS = 30 * 60_000

export interface WorkflowStatus {
  phase: string
  /** 0–1 */
  progress?: number
  note?: string
  /** mtime of the file, ms since epoch. */
  updatedAt: number
}

// C0/C1 controls, bidi embeddings / isolates / marks, zero-width and BOM, line / paragraph separators.
const UNSAFE = /[\u0000-\u001f\u007f-\u009f\u061c\u200b-\u200f\u2028-\u202e\u2060-\u206f\ufeff]/g

export function cleanStatusText(raw: unknown, max: number): string {
  if (typeof raw !== 'string') return ''
  const text = raw.replace(UNSAFE, ' ').replace(/\s+/g, ' ').trim()
  return Array.from(text).slice(0, max).join('').trim()
}

/** null = not a usable status (bad JSON, no phase). Never throws. */
export function parseAgentStatus(text: string, updatedAt: number): WorkflowStatus | null {
  let data: unknown
  try {
    data = JSON.parse(text.charCodeAt(0) === 0xfeff ? text.slice(1) : text)
  } catch {
    return null
  }
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null
  const o = data as Record<string, unknown>
  const phase = cleanStatusText(o.phase, AGENT_STATUS_PHASE_MAX)
  if (!phase) return null
  const note = cleanStatusText(o.note, AGENT_STATUS_NOTE_MAX)
  const progress = typeof o.progress === 'number' && Number.isFinite(o.progress) ? Math.min(1, Math.max(0, o.progress)) : undefined
  return { phase, ...(progress === undefined ? {} : { progress }), ...(note ? { note } : {}), updatedAt }
}

export const isStatusStale = (s: Pick<WorkflowStatus, 'updatedAt'>, nowMs: number): boolean => nowMs - s.updatedAt >= AGENT_STATUS_STALE_MS

/** Page → main watch request: `dir` null stops watching `key`. */
export interface WorkflowStatusEvent {
  key: string
  status: WorkflowStatus | null
}

export function cleanAgentStatus(raw: unknown): WorkflowStatus | null {
  if (!raw || typeof raw !== 'object') return null
  const o = raw as Record<string, unknown>
  if (typeof o.updatedAt !== 'number' || !Number.isFinite(o.updatedAt)) return null
  return parseAgentStatus(JSON.stringify({ phase: o.phase, progress: o.progress, note: o.note }), o.updatedAt)
}
