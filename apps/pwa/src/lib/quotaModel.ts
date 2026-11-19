import type { RemoteQuota, RemoteQuotaAgent, RemoteQuotaWindow } from '@ccm/shared'
import type { MonitorSnapshot } from '../remote/monitorReducer'
import { formatElapsed, type Translate } from './monitorModel'

export const QUOTA_STALE_MS = 30 * 60_000

type Item = Record<string, unknown>

const isItem = (value: unknown): value is Item => !!value && typeof value === 'object' && !Array.isArray(value)

const SOURCES: ReadonlyArray<readonly [RemoteQuotaAgent, string]> = [
  ['claude', 'limits'],
  ['codex', 'codexLimits'],
]

const WINDOWS: ReadonlyArray<readonly [RemoteQuotaWindow, string]> = [
  ['5h', 'fiveHour'],
  ['week', 'sevenDay'],
]

export function quotaOf(snapshot: MonitorSnapshot | null | undefined, now: number): RemoteQuota[] {
  const out: RemoteQuota[] = []
  if (!snapshot) return out
  for (const [agent, field] of SOURCES) {
    const limits = snapshot[field]
    if (!isItem(limits)) continue
    const updated = typeof limits.updatedAt === 'string' ? Date.parse(limits.updatedAt) : Number.NaN
    const stale = Number.isFinite(updated) && now - updated > QUOTA_STALE_MS
    for (const [window, key] of WINDOWS) {
      const w = limits[key]
      if (!isItem(w) || typeof w.usedPct !== 'number' || !Number.isFinite(w.usedPct)) continue
      const at = typeof w.resetsAt === 'string' ? Date.parse(w.resetsAt) : Number.NaN
      out.push({ agent, window, pct: Math.min(100, Math.max(0, Math.round(w.usedPct))), resetAt: Number.isFinite(at) ? at : null, stale })
    }
  }
  return out
}

export function quotaByAgent(rows: readonly RemoteQuota[]): Array<{ agent: RemoteQuotaAgent; windows: RemoteQuota[]; stale: boolean }> {
  const out: Array<{ agent: RemoteQuotaAgent; windows: RemoteQuota[]; stale: boolean }> = []
  for (const row of rows) {
    let group = out.find((g) => g.agent === row.agent)
    if (!group) {
      group = { agent: row.agent, windows: [], stale: row.stale }
      out.push(group)
    }
    group.windows.push(row)
  }
  return out
}

export function formatPct(pct: number, locale: string): string {
  return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 }).format(pct / 100)
}

const sameDay = (a: Date, b: Date): boolean => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()

export function formatResetAt(resetAt: number, now: number, locale: string): string {
  const at = new Date(resetAt)
  const time = new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' }).format(at)
  if (sameDay(at, new Date(now))) return time
  return `${new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(at)} ${time}`
}

export function formatReset(resetAt: number | null, now: number, locale: string, t: Translate): string {
  if (resetAt === null || resetAt <= now) return ''
  return t('quota.resetsIn', { time: formatElapsed(resetAt - now, t), at: formatResetAt(resetAt, now, locale) })
}
