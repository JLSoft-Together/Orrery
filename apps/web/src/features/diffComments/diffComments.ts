import type { DiffFile } from '../git/lib/diff'

export type CommentSide = 'new' | 'old'

export interface DiffComment {
  id: string
  path: string
  side: CommentSide
  from: number
  to: number
  code: string
  text: string
  stale: boolean
  at: number
}

export interface CommentDraft {
  root: string
  path: string
  side: CommentSide
  from: number
  to: number
  text: string
  editId?: string
  where: 'inline' | 'bar'
}

export interface PromptLabels {
  head: string
  removed: string
  more: (n: number) => string
}

export const COMMENTS_MAX = 100
export const SNIPPET_LINES = 4
export const SNIPPET_COLS = 200
export const PROMPT_MAX = 6000

export function sideLines(file: DiffFile, side: CommentSide): Map<number, string> {
  const out = new Map<number, string>()
  for (const h of file.hunks) {
    for (const l of h.lines) {
      if (side === 'new' && l.new !== undefined && (l.type === 'add' || l.type === 'ctx')) out.set(l.new, l.text)
      if (side === 'old' && l.old !== undefined && (l.type === 'del' || l.type === 'ctx')) out.set(l.old, l.text)
    }
  }
  return out
}

export function normRange(a: number, b: number): [number, number] {
  return a <= b ? [a, b] : [b, a]
}

const clip = (s: string): string => (s.length > SNIPPET_COLS ? `${s.slice(0, SNIPPET_COLS)}…` : s)

function snippetFrom(lines: Map<number, string>, from: number, to: number): string | null {
  const out: string[] = []
  for (let n = from; n <= to; n++) {
    const text = lines.get(n)
    if (text === undefined) return null
    if (out.length < SNIPPET_LINES) out.push(clip(text))
  }
  if (to - from + 1 > SNIPPET_LINES) out.push('…')
  return out.join('\n')
}

export function snippetOf(file: DiffFile, side: CommentSide, from: number, to: number): string {
  return snippetFrom(sideLines(file, side), from, to) ?? ''
}

export function rangeLabel(c: Pick<DiffComment, 'path' | 'from' | 'to'>): string {
  return c.from === c.to ? `${c.path}:${c.from}` : `${c.path}:${c.from}-${c.to}`
}

export function addComment(list: readonly DiffComment[], c: DiffComment): DiffComment[] {
  const next = [...list, c]
  return next.length > COMMENTS_MAX ? next.slice(next.length - COMMENTS_MAX) : next
}

export function reconcile(list: readonly DiffComment[], path: string, file: DiffFile): DiffComment[] | null {
  if (!list.some((c) => c.path === path)) return null
  const bySide: Partial<Record<CommentSide, Map<number, string>>> = {}
  let changed = false
  const next = list.map((c) => {
    if (c.path !== path || !c.code) return c
    const lines = (bySide[c.side] ??= sideLines(file, c.side))
    const now = snippetFrom(lines, c.from, c.to)
    if (now === c.code) {
      if (!c.stale) return c
      changed = true
      return { ...c, stale: false }
    }
    const span = c.to - c.from
    const hits: number[] = []
    for (const start of lines.keys()) {
      if (start !== c.from && snippetFrom(lines, start, start + span) === c.code) hits.push(start)
      if (hits.length > 1) break
    }
    if (hits.length === 1) {
      changed = true
      return { ...c, from: hits[0]!, to: hits[0]! + span, stale: false }
    }
    if (now === null || c.stale) return c
    changed = true
    return { ...c, stale: true }
  })
  return changed ? next : null
}

const indent = (text: string, pad: string): string =>
  text
    .split(/\r?\n/)
    .map((l) => (l ? pad + l : l))
    .join('\n')

export function promptItem(c: DiffComment, n: number, labels: PromptLabels): string {
  const head = `${n}. ${rangeLabel(c)}${c.side === 'old' ? ` ${labels.removed}` : ''}`
  const code = c.code ? `\n   \`\`\`\n${indent(c.code, '   ')}\n   \`\`\`` : ''
  return `${head}${code}\n${indent(c.text.trim(), '   ')}`
}

export function sortComments(list: readonly DiffComment[]): DiffComment[] {
  return [...list].sort((a, b) => a.path.localeCompare(b.path) || (a.side === b.side ? 0 : a.side === 'new' ? -1 : 1) || a.from - b.from || a.at - b.at)
}

export function buildPrompt(list: readonly DiffComment[], labels: PromptLabels): string {
  const items = sortComments(list.filter((c) => c.text.trim()))
  if (!items.length) return ''
  let out = labels.head
  for (let i = 0; i < items.length; i++) {
    const part = `\n\n${promptItem(items[i]!, i + 1, labels)}`
    const rest = items.length - i
    if (out.length + part.length > PROMPT_MAX) {
      out += `\n\n${labels.more(rest)}`
      break
    }
    out += part
  }
  return out
}

export function parseComments(raw: string | null): DiffComment[] {
  if (!raw) return []
  try {
    const data: unknown = JSON.parse(raw)
    if (!Array.isArray(data)) return []
    const out: DiffComment[] = []
    for (const v of data) {
      if (!v || typeof v !== 'object') continue
      const c = v as Record<string, unknown>
      if (typeof c.id !== 'string' || typeof c.path !== 'string' || typeof c.text !== 'string') continue
      if (typeof c.from !== 'number' || typeof c.to !== 'number' || !Number.isInteger(c.from) || !Number.isInteger(c.to) || c.from < 1) continue
      const [from, to] = normRange(c.from, c.to)
      out.push({
        id: c.id,
        path: c.path,
        side: c.side === 'old' ? 'old' : 'new',
        from,
        to,
        code: typeof c.code === 'string' ? c.code : '',
        text: c.text,
        stale: c.stale === true,
        at: typeof c.at === 'number' ? c.at : 0,
      })
    }
    return out.slice(-COMMENTS_MAX)
  } catch {
    return []
  }
}
