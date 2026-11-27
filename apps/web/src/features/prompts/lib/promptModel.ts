import { maskSecrets } from '@ccm/shared'

export const PROMPT_FORMAT = 'orrery-saved-prompts'
export const SCOPE_ALL = 'all'
export const MAX_ITEMS = 200
export const MAX_TITLE = 80
export const MAX_BODY = 4000
export const MAX_SCOPE = 4096

export interface SavedPrompt {
  id: string
  title: string
  body: string
  scope: string
  updatedAt: number
}

export interface PromptDraft {
  id?: string
  title: string
  body: string
  scope: string
}

export type PromptError = 'invalid-op' | 'invalid-id' | 'invalid-title' | 'invalid-body' | 'invalid-scope' | 'too-many' | 'not-found' | 'io' | 'unavailable'

export const PROMPT_ERRORS: readonly PromptError[] = ['invalid-op', 'invalid-id', 'invalid-title', 'invalid-body', 'invalid-scope', 'too-many', 'not-found', 'io', 'unavailable']

export const toPromptError = (v: unknown): PromptError => (PROMPT_ERRORS as readonly unknown[]).includes(v) ? (v as PromptError) : 'io'

const isAbsolute = (s: string): boolean => /^[A-Za-z]:[\\/]/.test(s) || s.startsWith('/') || s.startsWith('\\\\')

export function isPrompt(v: unknown): v is SavedPrompt {
  if (!v || typeof v !== 'object') return false
  const p = v as Record<string, unknown>
  return typeof p.id === 'string' && typeof p.title === 'string' && typeof p.body === 'string' && typeof p.scope === 'string' && typeof p.updatedAt === 'number'
}

const CONTROL = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F-\u009F]/
const CONTROL_ALL = new RegExp(CONTROL.source, 'g')

export const hasControlChars = (s: string): boolean => CONTROL.test(s)

export const stripControlChars = (s: string): string => s.replace(CONTROL_ALL, '')

export const normalizeBody = (body: string): string => body.replace(/\r\n?/g, '\n')

export function fitsDraft(d: Pick<PromptDraft, 'title' | 'body' | 'scope'>): PromptError | null {
  const title = d.title.trim()
  if (!title || title.length > MAX_TITLE) return 'invalid-title'
  if (!d.body.trim() || d.body.length > MAX_BODY || hasControlChars(d.body)) return 'invalid-body'
  if (d.scope !== SCOPE_ALL && (d.scope.length > MAX_SCOPE || !isAbsolute(d.scope))) return 'invalid-scope'
  return null
}

export const looksSecret = (body: string): boolean => maskSecrets(body, '', 'linux') !== body

export function autoTitle(text: string): string {
  const first = text.split(/\r?\n/).find((l) => l.trim()) ?? ''
  const flat = first.replace(/\s+/g, ' ').trim()
  return flat.length > 60 ? `${flat.slice(0, 59)}…` : flat
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean)
  return parts[parts.length - 1] ?? path
}

const normPath = (p: string): string => {
  const slashed = p.replace(/\\/g, '/').replace(/\/+$/, '')
  return /^[A-Za-z]:/.test(slashed) ? slashed.toLowerCase() : slashed
}

export function scopeMatches(scope: string, cwd: string | undefined): boolean {
  if (scope === SCOPE_ALL) return true
  if (!cwd) return false
  const a = normPath(scope)
  const b = normPath(cwd)
  return b === a || b.startsWith(`${a}/`)
}

export function exportText(items: readonly SavedPrompt[], at = new Date()): string {
  return JSON.stringify({ format: PROMPT_FORMAT, version: 1, exportedAt: at.toISOString(), prompts: items.map(({ title, body, scope }) => ({ title, body, scope })) }, null, 2)
}

export interface ImportRow {
  title: string
  body: string
  scope: string
}

export function parseImport(raw: unknown): { rows: ImportRow[]; skipped: number } | null {
  const holder = raw && typeof raw === 'object' ? (raw as { prompts?: unknown; items?: unknown }) : null
  const list = Array.isArray(raw) ? raw : (holder?.prompts ?? holder?.items)
  if (!Array.isArray(list)) return null
  const rows: ImportRow[] = []
  let skipped = 0
  for (const r of list) {
    const p = (r && typeof r === 'object' ? r : {}) as Record<string, unknown>
    const title = typeof p.title === 'string' ? p.title.trim() : ''
    const body = typeof p.body === 'string' ? normalizeBody(p.body) : ''
    const scope = typeof p.scope === 'string' ? p.scope : SCOPE_ALL
    if (fitsDraft({ title, body, scope }) || rows.length >= MAX_ITEMS) skipped++
    else rows.push({ title, body, scope })
  }
  return { rows, skipped }
}
