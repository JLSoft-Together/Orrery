import type { AgentStatus, ToolOutcome, ToolSpan } from '@ccm/shared'
import { hashName, subAngle } from './orbitRings'

export const FOCUS_RING_FRACTIONS = [0.52, 0.72, 0.92] as const
export const MAIN_RING_FRACTION = 0.3
export const TRAILS_PER_AGENT = 4
export const FEED_LIMIT = 60
export const PLANET_RADIUS = 11

const MIN_ARC = 0.08
const MAX_ARC = 0.7
const TRAIL_GAP = 0.05
const TRAIL_TOTAL = Math.PI * 1.6
const FALLBACK_W = 700
const FALLBACK_H = 560

export interface FocusSub {
  id: string
  status: AgentStatus
}

export interface FocusInput {
  terminalId: string
  mainId: string | null
  subs: readonly FocusSub[]
  spans: readonly Pick<ToolSpan, 'id' | 'agentId' | 'start' | 'end' | 'outcome'>[]
  nowMs: number
  width: number
  height: number
  nudges?: Readonly<Record<string, number>>
  reduced?: boolean
}

export interface Planet {
  id: string
  status: AgentStatus
  ring: number
  r: number
  angle: number
  x: number
  y: number
}

export interface Trail {
  id: string
  agentId: string
  r: number
  a0: number
  a1: number
  running: boolean
  outcome?: ToolOutcome
}

export interface FocusResult {
  cx: number
  cy: number
  half: number
  sunR: number
  mainR: number
  rings: number[]
  planets: Planet[]
  trails: Trail[]
}

export function arcLength(ms: number): number {
  const sec = Math.max(0, ms) / 1000
  return Math.min(MAX_ARC, Math.max(MIN_ARC, 0.06 + 0.1 * Math.log2(1 + sec)))
}

export function arcPath(cx: number, cy: number, r: number, a0: number, a1: number): string {
  const x0 = cx + r * Math.cos(a0)
  const y0 = cy + r * Math.sin(a0)
  const x1 = cx + r * Math.cos(a1)
  const y1 = cy + r * Math.sin(a1)
  const large = a1 - a0 > Math.PI ? 1 : 0
  const f = (n: number) => Math.round(n * 100) / 100
  return `M${f(x0)} ${f(y0)} A${f(r)} ${f(r)} 0 ${large} 1 ${f(x1)} ${f(y1)}`
}

function trailsFor(
  agentId: string,
  r: number,
  anchor: number,
  gap: number,
  spans: FocusInput['spans'],
  nowMs: number,
): Trail[] {
  const mine = spans.filter((s) => s.agentId === agentId).sort((a, b) => Date.parse(b.start) - Date.parse(a.start))
  const out: Trail[] = []
  let end = anchor - gap
  let used = 0
  for (const s of mine.slice(0, TRAILS_PER_AGENT)) {
    const start = Date.parse(s.start)
    const stop = s.end ? Date.parse(s.end) : nowMs
    if (!Number.isFinite(start) || !Number.isFinite(stop)) continue
    const len = arcLength(stop - start)
    if (used + len > TRAIL_TOTAL) break
    out.push({ id: s.id, agentId, r, a0: end - len, a1: end, running: !s.end, outcome: s.outcome })
    end -= len + TRAIL_GAP
    used += len + TRAIL_GAP
  }
  return out
}

export function computeFocus(input: FocusInput): FocusResult {
  const width = input.width > 0 ? input.width : FALLBACK_W
  const height = input.height > 0 ? input.height : FALLBACK_H
  const reduced = input.reduced ?? false
  const nudges = input.nudges ?? {}
  const cx = width / 2
  const cy = height / 2
  const half = Math.max(120, Math.min(width, height) / 2 - 44)
  const sunR = half * 0.12
  const mainR = half * MAIN_RING_FRACTION
  const rings = FOCUS_RING_FRACTIONS.map((f) => f * half)

  const planets: Planet[] = []
  const trails: Trail[] = []
  const ordered = [...input.subs].sort((a, b) => a.id.localeCompare(b.id))
  ordered.forEach((sub, i) => {
    const ring = i % rings.length
    const r = rings[ring]!
    const angle = subAngle(input.terminalId, i, 1, nudges[sub.id] ?? 0, reduced) + (hashName(sub.id) % 628) / 100
    planets.push({ id: sub.id, status: sub.status, ring, r, angle, x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) })
    trails.push(...trailsFor(sub.id, r, angle, (PLANET_RADIUS + 4) / r, input.spans, input.nowMs))
  })
  if (input.mainId) {
    const anchor = subAngle(input.mainId, 0, 1, nudges[input.mainId] ?? 0, reduced)
    trails.push(...trailsFor(input.mainId, mainR, anchor, 0.05, input.spans, input.nowMs))
  }
  return { cx, cy, half, sunR, mainR, rings, planets, trails }
}

export interface FeedRow {
  id: string
  agentId: string
  tool: string
  start: string
  ms: number | null
  state: 'running' | ToolOutcome
}

export function feedRows(spans: Iterable<ToolSpan>, liveAgents: ReadonlySet<string>, limit = FEED_LIMIT): FeedRow[] {
  const all: ToolSpan[] = []
  for (const s of spans) all.push(s)
  all.sort((a, b) => Date.parse(b.start) - Date.parse(a.start) || b.id.localeCompare(a.id))
  return all.slice(0, limit).map((s): FeedRow => {
    const start = Date.parse(s.start)
    const end = s.end ? Date.parse(s.end) : null
    const ms = end !== null && Number.isFinite(start) && Number.isFinite(end) ? Math.max(0, end - start) : null
    return {
      id: s.id,
      agentId: s.agentId,
      tool: s.tool,
      start: s.start,
      ms,
      state: s.end ? (s.outcome ?? 'ok') : liveAgents.has(s.agentId) ? 'running' : 'interrupted',
    }
  })
}

export function railOrder(groups: readonly { ids: readonly string[] }[]): string[] {
  return groups.flatMap((g) => g.ids)
}

export function stepSystem(ids: readonly string[], current: string, delta: 1 | -1): string {
  if (!ids.length) return current
  const at = ids.indexOf(current)
  if (at < 0) return ids[0]!
  return ids[(at + delta + ids.length) % ids.length]!
}
