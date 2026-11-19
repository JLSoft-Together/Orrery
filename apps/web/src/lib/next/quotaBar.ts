import type { PlanLimits } from '@ccm/shared'
import { limitLevel, limitsStale, type LimitLevel } from '../headerStatus'

export type QuotaAgentId = 'claude' | 'codex'
export type QuotaKey = '5h' | '7d'

export interface QuotaWindowView {
  key: QuotaKey
  used: number
  tone: LimitLevel
  resetsAt: number | null
}

export interface QuotaAgent {
  id: QuotaAgentId
  name: string
  stale: boolean
  windows: QuotaWindowView[]
}

const NAMES: Record<QuotaAgentId, string> = { claude: 'Claude Code', codex: 'Codex' }

function windowsOf(limits: PlanLimits): QuotaWindowView[] {
  const out: QuotaWindowView[] = []
  for (const [key, w] of [
    ['5h', limits.fiveHour],
    ['7d', limits.sevenDay],
  ] as const) {
    if (!w || !Number.isFinite(w.usedPct)) continue
    const used = Math.min(100, Math.max(0, Math.round(w.usedPct)))
    const at = w.resetsAt ? Date.parse(w.resetsAt) : NaN
    out.push({ key, used, tone: limitLevel(100 - used), resetsAt: Number.isFinite(at) ? at : null })
  }
  return out
}

export function quotaAgents(claude: PlanLimits | null | undefined, codex: PlanLimits | null | undefined, nowMs: number): QuotaAgent[] {
  const list: QuotaAgent[] = []
  for (const [id, limits] of [
    ['claude', claude],
    ['codex', codex],
  ] as const) {
    if (!limits) continue
    const windows = windowsOf(limits)
    if (windows.length) list.push({ id, name: NAMES[id], stale: limitsStale(limits, nowMs), windows })
  }
  return list
}

const sameDay = (a: Date, b: Date): boolean => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()

export function resetLabel(resetsAt: number | null, nowMs: number, locale: string): string {
  if (resetsAt === null || resetsAt <= nowMs) return ''
  const at = new Date(resetsAt)
  const time = new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' }).format(at)
  if (sameDay(at, new Date(nowMs))) return time
  return `${new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(at)} ${time}`
}

export const QUOTA_TONE_CLASS: Record<LimitLevel, string> = { ok: 'bg-accent', warn: 'bg-warning', critical: 'bg-st-error' }
