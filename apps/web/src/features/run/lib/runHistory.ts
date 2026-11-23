import type { RunHistoryEntry, RunLogLine } from '@ccm/shared'

export const COMPARE_MAX_DIFF = 500
export const COMPARE_MAX_LINES = 200_000
export const HISTORY_PAGE = 1000

const RULES: [RegExp, string][] = [
  [/\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+-]\d{2}:?\d{2})?/g, '<ts>'],
  [/\b\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?/g, '<ts>'],
  [/\b\d{1,2}:\d{2}:\d{2}(?:[.,]\d+)?\b/g, '<ts>'],
  [/(?:[A-Za-z]:)?[^\s'"`]*[\\/](?:Temp|tmp)[\\/][^\s'"`]*/gi, '<tmp>'],
  [/\/var\/folders\/[^\s'"`]*/g, '<tmp>'],
  [/\b(pid|PID|process|Process)[ =:]*\d+/g, '$1 <pid>'],
  [/\((\d{2,})\)/g, '(<pid>)'],
  [/\[(\d{2,})\]/g, '[<pid>]'],
  [/\b[0-9a-fA-F]{8,}\b/g, '<hex>'],
  [/\b\d+(?:\.\d+)?\s?(?:ms|s)\b/g, '<dur>'],
]

export function normalizeLine(text: string): string {
  let out = text
  for (const [re, to] of RULES) out = out.replace(re, to)
  return out.replace(/\s+/g, ' ').trim()
}

export interface DiffLine {
  index: number
  text: string
}

export interface RunCompare {
  oldCode: number | null
  newCode: number | null
  oldMs: number | null
  newMs: number | null
  onlyOld: DiffLine[]
  onlyNew: DiffLine[]
  totalOnlyOld: number
  totalOnlyNew: number
}

export function durationOf(e: Pick<RunHistoryEntry, 'start' | 'end'>): number | null {
  return e.end === null ? null : Math.max(0, e.end - e.start)
}

export function formatDuration(ms: number | null): string {
  if (ms === null) return ''
  if (ms < 1000) return `${Math.round(ms)} ms`
  const s = ms / 1000
  if (s < 60) return `${s.toFixed(s < 10 ? 1 : 0)} s`
  const m = Math.floor(s / 60)
  return `${m} m ${String(Math.round(s - m * 60)).padStart(2, '0')} s`
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(n < 10 * 1024 ? 1 : 0)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}

export function compareRuns(oldRun: RunHistoryEntry, newRun: RunHistoryEntry, oldLines: readonly RunLogLine[], newLines: readonly RunLogLine[]): RunCompare {
  const oldNorm = oldLines.map((l) => normalizeLine(l.text))
  const newNorm = newLines.map((l) => normalizeLine(l.text))
  const countOld = new Map<string, number>()
  const countNew = new Map<string, number>()
  for (const n of oldNorm) if (n) countOld.set(n, (countOld.get(n) ?? 0) + 1)
  for (const n of newNorm) if (n) countNew.set(n, (countNew.get(n) ?? 0) + 1)
  const side = (norm: string[], lines: readonly RunLogLine[], other: Map<string, number>): { list: DiffLine[]; total: number } => {
    const used = new Map<string, number>()
    const list: DiffLine[] = []
    let total = 0
    for (let i = 0; i < norm.length; i++) {
      const n = norm[i]!
      if (!n) continue
      const seen = (used.get(n) ?? 0) + 1
      used.set(n, seen)
      if (seen <= (other.get(n) ?? 0)) continue
      total++
      if (list.length < COMPARE_MAX_DIFF) list.push({ index: i, text: lines[i]!.text })
    }
    return { list, total }
  }
  const o = side(oldNorm, oldLines, countNew)
  const n = side(newNorm, newLines, countOld)
  return {
    oldCode: oldRun.exitCode,
    newCode: newRun.exitCode,
    oldMs: durationOf(oldRun),
    newMs: durationOf(newRun),
    onlyOld: o.list,
    onlyNew: n.list,
    totalOnlyOld: o.total,
    totalOnlyNew: n.total,
  }
}

export type ExitTone = 'ok' | 'error' | 'unknown'

export function exitTone(e: Pick<RunHistoryEntry, 'exitCode' | 'end'>): ExitTone {
  if (e.end === null || e.exitCode === null) return 'unknown'
  return e.exitCode === 0 ? 'ok' : 'error'
}

export function defaultCompare(list: readonly RunHistoryEntry[]): { baseline: string; latest: string } | null {
  const done = list.filter((e) => e.end !== null)
  if (done.length < 2) return null
  const latest = done[0]!
  const baseline = done.find((e) => e.pinned && e.id !== latest.id) ?? done[1]!
  return { baseline: baseline.id, latest: latest.id }
}
