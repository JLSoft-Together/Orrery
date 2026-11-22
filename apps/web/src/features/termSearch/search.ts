export const MAX_RESULTS = 500
export const MAX_ROWS = 5000
export const DEBOUNCE_MS = 200
export const CONTEXT_LINES = 1
const SHOWN_CHARS = 200
const LEAD_CHARS = 60

export interface SearchOptions {
  caseSensitive: boolean
  regex: boolean
}

export type Compiled = { ok: true; re: RegExp } | { ok: false; error: string } | null

export interface TermRow {
  text: string
  wrapped: boolean
}

export interface TermDump {
  rows: TermRow[]
  firstRow: number
  cols: number
}

export interface LogicalLine {
  text: string
  row: number
}

export interface Range {
  start: number
  len: number
}

export interface Hit {
  row: number
  col: number
  len: number
  before: string[]
  after: string[]
  text: string
  ranges: Range[]
  count: number
}

export type PaneStatus = 'ok' | 'unloaded'

export interface PaneResult {
  key: string
  status: PaneStatus
  hits: Hit[]
  count: number
  more: boolean
}

export interface SearchSource {
  key: string
  read: () => Promise<TermDump | null>
}

const escapeRe = (s: string): string => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

export function compileQuery(query: string, opts: SearchOptions): Compiled {
  if (!query) return null
  const flags = opts.caseSensitive ? 'g' : 'gi'
  try {
    return { ok: true, re: new RegExp(opts.regex ? query : escapeRe(query), flags) }
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) }
  }
}

export function joinRows(dump: TermDump): LogicalLine[] {
  const out: LogicalLine[] = []
  dump.rows.forEach((r, i) => {
    const last = out[out.length - 1]
    if (r.wrapped && last) last.text += r.text
    else out.push({ text: r.text, row: dump.firstRow + i })
  })
  return out
}

function clip(text: string): string {
  return text.length > SHOWN_CHARS ? text.slice(0, SHOWN_CHARS) : text
}

function matchesIn(text: string, re: RegExp): Range[] {
  const out: Range[] = []
  re.lastIndex = 0
  let m: RegExpExecArray | null
  while ((m = re.exec(text))) {
    if (!m[0].length) {
      re.lastIndex++
      if (re.lastIndex > text.length) break
      continue
    }
    out.push({ start: m.index, len: m[0].length })
  }
  return out
}

export function searchLines(lines: readonly LogicalLine[], re: RegExp, cols: number, limit: number, context = CONTEXT_LINES): { hits: Hit[]; count: number; more: boolean } {
  const hits: Hit[] = []
  let count = 0
  const width = Math.max(1, cols)
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const found = matchesIn(line.text, re)
    if (!found.length) continue
    if (hits.length >= limit) return { hits, count, more: true }
    count += found.length
    const first = found[0]!
    const cut = first.start > LEAD_CHARS ? first.start - LEAD_CHARS : 0
    const shown = line.text.slice(cut, cut + SHOWN_CHARS)
    hits.push({
      row: line.row + Math.floor(first.start / width),
      col: first.start % width,
      len: first.len,
      text: shown,
      ranges: found.map((r) => ({ start: r.start - cut, len: r.len })).filter((r) => r.start >= 0 && r.start < shown.length).map((r) => ({ start: r.start, len: Math.min(r.len, shown.length - r.start) })),
      count: found.length,
      before: lines.slice(Math.max(0, i - context), i).map((l) => clip(l.text)),
      after: lines.slice(i + 1, i + 1 + context).map((l) => clip(l.text)),
    })
  }
  return { hits, count, more: false }
}

export function searchDump(key: string, dump: TermDump | null, re: RegExp, limit: number): PaneResult {
  if (!dump || !dump.rows.some((r) => r.text.trim())) return { key, status: 'unloaded', hits: [], count: 0, more: false }
  const rows = dump.rows.length > MAX_ROWS ? { ...dump, rows: dump.rows.slice(-MAX_ROWS), firstRow: dump.firstRow + dump.rows.length - MAX_ROWS } : dump
  const { hits, count, more } = searchLines(joinRows(rows), re, rows.cols, limit)
  return { key, status: 'ok', hits, count, more }
}

export type Schedule = (fn: () => void) => void

export const idleSchedule: Schedule = (fn) => {
  const ric = (globalThis as { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number }).requestIdleCallback
  if (ric) ric(fn, { timeout: 120 })
  else setTimeout(fn, 0)
}

export interface RunOptions {
  max?: number
  schedule?: Schedule
  cancelled?: () => boolean
  onPane?: (r: PaneResult, total: number) => void
}

export interface RunResult {
  panes: PaneResult[]
  total: number
  truncated: boolean
}

export async function runSearch(sources: readonly SearchSource[], re: RegExp, opts: RunOptions = {}): Promise<RunResult | null> {
  const max = opts.max ?? MAX_RESULTS
  const schedule = opts.schedule ?? idleSchedule
  const panes: PaneResult[] = []
  let total = 0
  let truncated = false
  for (const src of sources) {
    await new Promise<void>((resolve) => schedule(resolve))
    if (opts.cancelled?.()) return null
    let dump: TermDump | null = null
    try {
      dump = await src.read()
    } catch {
      dump = null
    }
    if (opts.cancelled?.()) return null
    const left = max - total
    const r = searchDump(src.key, dump, re, Math.max(0, left))
    if (r.more) truncated = true
    total += r.hits.length
    panes.push(r)
    opts.onPane?.(r, total)
  }
  return { panes, total, truncated }
}
