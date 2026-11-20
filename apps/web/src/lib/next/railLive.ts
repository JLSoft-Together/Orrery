import type { WaitReason } from '@ccm/shared'
import type { RailCluster, RailRow, RailState } from '../rail/railModel'

export type AttnKind = 'permission' | 'question' | 'error'
export type StatusGroup = 'failed' | 'working' | 'done' | 'idle'

export const STATUS_GROUPS: readonly StatusGroup[] = ['failed', 'working', 'done', 'idle']
export const SNOOZE_MINUTES = [5, 15, 60] as const
export const ERROR_SNOOZE_MAX_MIN = 15
export const UNDO_MS = 5000
export const SIGNAL_MS = 450

export interface LiveExtras {
  model: string | null
  ctxPct: number | null
  tool: string | null
  waitReason: WaitReason | null
  waitText: string | null
  stateText: string | null
  title: string
  clash?: RowClash | null
}

export interface RowClash {
  n: number
  tip: string
}

export interface LiveRow {
  key: string
  paneKey: string
  tabKey: string
  clusterKey: string
  clusterName: string
  tone: number
  agent: string
  title: string
  state: RailState
  attn: AttnKind | null
  since: number
  subagents: number
  model: string | null
  ctxPct: number | null
  tool: string | null
  waitText: string | null
  stateText: string | null
  shell: boolean
  focused: boolean
  src: RailRow
  clash?: RowClash
}

export interface LayoutSection {
  id: string
  keys: string[]
}

export function attnKindOf(state: RailState, reason: WaitReason | null): AttnKind | null {
  if (state === 'failed') return 'error'
  if (state === 'asked') return reason === 'permission' ? 'permission' : 'question'
  return null
}

export function statusGroupOf(state: RailState): StatusGroup | null {
  switch (state) {
    case 'asked':
      return null
    case 'failed':
      return 'failed'
    case 'working':
    case 'subagents':
    case 'starting':
      return 'working'
    case 'done':
      return 'done'
    default:
      return 'idle'
  }
}

export function attnRank(kind: AttnKind | null, state: RailState): number {
  if (kind === 'permission' || kind === 'question') return 0
  if (kind === 'error') return 1
  if (state === 'done') return 2
  return 3
}

const age = (since: number): number => (since > 0 ? since : Number.POSITIVE_INFINITY)

export function compareNeeds(a: Pick<LiveRow, 'attn' | 'state' | 'since' | 'key'>, b: Pick<LiveRow, 'attn' | 'state' | 'since' | 'key'>): number {
  return attnRank(a.attn, a.state) - attnRank(b.attn, b.state) || age(a.since) - age(b.since) || a.key.localeCompare(b.key)
}

export function sortNeeds<T extends Pick<LiveRow, 'attn' | 'state' | 'since' | 'key'>>(rows: readonly T[]): T[] {
  return [...rows].sort(compareNeeds)
}

export function needsOf(rows: readonly LiveRow[], snoozed: (key: string) => boolean): LiveRow[] {
  return sortNeeds(rows.filter((r) => r.attn !== null && !snoozed(r.key)))
}

export function recentFirst<T extends Pick<LiveRow, 'since' | 'key'>>(rows: readonly T[]): T[] {
  return [...rows].sort((a, b) => b.since - a.since || a.key.localeCompare(b.key))
}

export function snoozeChoices(kind: AttnKind): number[] {
  return SNOOZE_MINUTES.filter((m) => kind !== 'error' || m <= ERROR_SNOOZE_MAX_MIN)
}

export function clampSnooze(kind: AttnKind, minutes: number): number {
  const max = kind === 'error' ? ERROR_SNOOZE_MAX_MIN : SNOOZE_MINUTES[SNOOZE_MINUTES.length - 1]!
  return Math.min(max, Math.max(1, Math.round(minutes)))
}

export function snoozeUntil(kind: AttnKind, minutes: number, nowMs: number): number {
  return nowMs + clampSnooze(kind, minutes) * 60_000
}

export function isSnoozedAt(until: Readonly<Record<string, number>>, key: string, nowMs: number): boolean {
  return (until[key] ?? 0) > nowMs
}

export function pruneSnoozed(until: Readonly<Record<string, number>>, nowMs: number, alive?: (key: string) => boolean): Record<string, number> {
  const out: Record<string, number> = {}
  for (const [k, v] of Object.entries(until)) if (v > nowMs && (!alive || alive(k))) out[k] = v
  return out
}

export function buildLiveRows(clusters: readonly RailCluster[], extrasOf: (row: RailRow) => LiveExtras, toneOf: (cluster: RailCluster) => number): LiveRow[] {
  const out: LiveRow[] = []
  for (const c of clusters) {
    if (!c.live) continue
    const tone = toneOf(c)
    for (const k of c.checkouts)
      for (const r of k.rows) {
        const x = extrasOf(r)
        const shell = r.kind === 'shell'
        out.push({
          key: r.key,
          paneKey: r.paneKey,
          tabKey: r.tabKey,
          clusterKey: c.key,
          clusterName: c.name,
          tone,
          agent: r.agent,
          title: x.title,
          state: r.state,
          attn: attnKindOf(r.state, x.waitReason),
          since: r.since,
          subagents: r.subagents,
          model: shell || r.kind === 'other' ? null : x.model,
          ctxPct: shell || r.kind === 'other' ? null : x.ctxPct,
          tool: x.tool,
          waitText: x.waitText,
          stateText: x.stateText,
          shell,
          focused: r.focused,
          src: r,
          ...(x.clash ? { clash: x.clash } : {}),
        })
      }
  }
  return out
}

export function detailOf(r: Pick<LiveRow, 'state' | 'tool' | 'waitText' | 'stateText'>): string | null {
  if (r.state === 'working' && r.tool) return r.tool
  const text = r.state === 'asked' ? (r.waitText ?? r.stateText) : r.stateText
  return text ? text.replaceAll(' · ', ', ') : text
}

export function compactElapsed(ms: number): string {
  const s = Math.floor(Math.max(0, ms) / 1000)
  if (s < 1) return ''
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  if (m < 10) return `${m}m ${String(s % 60).padStart(2, '0')}s`
  if (m < 60) return `${m}m`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ${String(m % 60).padStart(2, '0')}m`
  return `${Math.floor(h / 24)}d`
}

export function timeLabel(r: Pick<LiveRow, 'since'>, nowMs: number): string {
  return r.since > 0 ? compactElapsed(nowMs - r.since) : ''
}

export function layoutOf(rows: readonly LiveRow[], mode: 'project' | 'status', clusterOrder: readonly string[]): LayoutSection[] {
  const sections: LayoutSection[] = []
  if (mode === 'project') {
    for (const key of clusterOrder) {
      const keys = rows.filter((r) => r.clusterKey === key).map((r) => r.key)
      if (keys.length) sections.push({ id: `p:${key}`, keys })
    }
    return sections
  }
  for (const g of STATUS_GROUPS) {
    const keys = recentFirst(rows.filter((r) => statusGroupOf(r.state) === g)).map((r) => r.key)
    if (keys.length) sections.push({ id: `s:${g}`, keys })
  }
  return sections
}

export const needsLayout = (needs: readonly LiveRow[]): LayoutSection[] => (needs.length ? [{ id: 'needs', keys: needs.map((r) => r.key) }] : [])

export function freezeLayout(prev: readonly LayoutSection[] | null, next: readonly LayoutSection[], hold: boolean): LayoutSection[] {
  if (!hold || !prev) return next.map((s) => ({ id: s.id, keys: [...s.keys] }))
  const nextKeys = new Set(next.flatMap((s) => s.keys))
  const placed = new Set<string>()
  const out: LayoutSection[] = []
  for (const s of prev) {
    const keys = s.keys.filter((k) => nextKeys.has(k))
    for (const k of keys) placed.add(k)
    out.push({ id: s.id, keys })
  }
  for (const s of next) {
    const fresh = s.keys.filter((k) => !placed.has(k))
    if (!fresh.length) continue
    const at = out.find((o) => o.id === s.id)
    if (at) at.keys.push(...fresh)
    else out.push({ id: s.id, keys: [...fresh] })
  }
  return out.filter((s) => s.keys.length)
}

export interface CollapsedProject {
  clusterKey: string
  name: string
  tone: number
  rows: LiveRow[]
  needs: number
  failed: number
}

export function collapsedProjects(rows: readonly LiveRow[], clusterOrder: readonly string[], snoozed: (key: string) => boolean): CollapsedProject[] {
  const out: CollapsedProject[] = []
  for (const key of clusterOrder) {
    const mine = rows.filter((r) => r.clusterKey === key)
    if (!mine.length) continue
    const live = mine.filter((r) => r.attn !== null && !snoozed(r.key))
    out.push({
      clusterKey: key,
      name: mine[0]!.clusterName,
      tone: mine[0]!.tone,
      rows: mine,
      needs: live.filter((r) => r.attn !== 'error').length,
      failed: live.filter((r) => r.attn === 'error').length,
    })
  }
  return out
}

export function isSignalEdge(prev: RailState | undefined, next: RailState): boolean {
  return prev !== undefined && prev !== next && (next === 'asked' || next === 'failed')
}
