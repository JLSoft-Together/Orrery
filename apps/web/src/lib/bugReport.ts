import type { BugReportItem } from './desktop'

export const SEEN_KEY = 'ccm.bugReport.seen'
const EVENTS_NAME = 'crashes/events.txt'
export const UI_ERROR_GRACE_MS = 3000
export const CRASH_PROMPT_DELAY_MS = 5000

export const KIND_ORDER: BugReportItem['kind'][] = ['report', 'log', 'crash', 'dump']

export interface ItemGroup {
  kind: BugReportItem['kind']
  items: BugReportItem[]
  bytes: number
}

export function formatBytes(bytes: number, locale = 'en'): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—'
  if (bytes < 1024) return `${bytes} B`
  const fmt = (n: number, digits: number) => new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(n)
  if (bytes < 1024 ** 2) return `${fmt(bytes / 1024, 0)} KB`
  return `${fmt(bytes / 1024 ** 2, bytes >= 100 * 1024 ** 2 ? 0 : 1)} MB`
}

export function groupItems(items: readonly BugReportItem[]): ItemGroup[] {
  return KIND_ORDER.map((kind) => {
    const list = items.filter((i) => i.kind === kind)
    return { kind, items: list, bytes: list.reduce((n, i) => n + i.bytes, 0) }
  }).filter((g) => g.items.length > 0)
}

export function totalBytes(items: readonly BugReportItem[], dumps: boolean): number {
  return items.reduce((n, i) => (i.kind === 'dump' && !dumps ? n : n + i.bytes), 0)
}

export const overLimit = (total: number, limit: number): boolean => limit > 0 && total > limit

export function readSeen(storage: Pick<Storage, 'getItem'> | null): Set<string> | null {
  try {
    const raw = storage?.getItem(SEEN_KEY)
    if (raw === null || raw === undefined) return null
    const list: unknown = JSON.parse(raw)
    return Array.isArray(list) ? new Set(list.filter((x): x is string => typeof x === 'string')) : null
  } catch {
    return null
  }
}

export function writeSeen(storage: Pick<Storage, 'setItem'> | null, seen: ReadonlySet<string>): void {
  try {
    storage?.setItem(SEEN_KEY, JSON.stringify([...seen].slice(-200)))
  } catch {
    return
  }
}

export interface CrashDiff {
  fresh: string[]
  next: Set<string>
  first: boolean
}

export function diffCrashes(items: readonly BugReportItem[], seen: ReadonlySet<string> | null): CrashDiff {
  const crashes = items.filter((i) => i.kind === 'crash' || i.kind === 'dump')
  const keyOf = (i: BugReportItem): string => (i.name === EVENTS_NAME ? `${i.name}#${i.bytes}` : i.name)
  const keys = crashes.map(keyOf)
  const next = new Set([...[...(seen ?? [])].filter((k) => !k.startsWith(`${EVENTS_NAME}#`)), ...keys])
  if (seen === null) return { fresh: [], next, first: true }
  const seenEvents = Math.max(0, ...[...seen].filter((k) => k.startsWith(`${EVENTS_NAME}#`)).map((k) => Number(k.slice(EVENTS_NAME.length + 1)) || 0))
  const fresh = crashes.filter((i) => (i.name === EVENTS_NAME ? i.bytes > seenEvents : !seen.has(i.name))).map((i) => i.name)
  return { fresh, next, first: false }
}

const NOISE = [/ResizeObserver loop/i, /^script error\.?$/i, /^$/]
const EXTENSION = /(chrome|moz|safari|webkit-masked-url)-?extension:/i

export function isNoise(message: string, stack = ''): boolean {
  const text = message.trim()
  if (NOISE.some((re) => re.test(text))) return true
  return EXTENSION.test(text) || EXTENSION.test(stack)
}

export function createUiErrorGate(now: () => number, graceMs = UI_ERROR_GRACE_MS): (message: string) => boolean {
  const startedAt = now()
  let shown = false
  return (message) => {
    if (shown || now() - startedAt < graceMs || isNoise(message)) return false
    shown = true
    return true
  }
}
