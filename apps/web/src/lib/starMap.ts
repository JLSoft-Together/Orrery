import type { AgentStatus, Provider } from '@ccm/shared'
import { hashName } from './orbitRings'
import type { SessionGroupKey } from './sessionGroups'

export const CELL_W = 230
export const CELL_H = 210
export const MAX_PLANETS = 6
export const MIN_SCALE = 0.4
export const MAX_SCALE = 2.4

export interface SystemKey {
  id: string
  project: string
}

export interface Placement {
  id: string
  x: number
  y: number
  slot: number
}

export interface SlotResult {
  placements: Placement[]
  slots: Map<string, number>
  cols: number
  rows: number
  width: number
  height: number
}

export interface Fit {
  scale: number
  x: number
  y: number
}

export interface PlanetSpot {
  id: string
  status: AgentStatus
  ring: 0 | 1
  angle: number
}

export interface PlanetRing {
  ring: 0 | 1
  radius: number
  seconds: number
}

export interface PlanetLayout {
  rings: PlanetRing[]
  spots: PlanetSpot[]
  more: number
}

export interface LinkPath {
  d: string
  mx: number
  my: number
}

export function statusRank(group: SessionGroupKey): number {
  return group === 'needs' ? 0 : group === 'running' ? 1 : group === 'idle' ? 2 : 3
}

export function slotColumns(width: number): number {
  return Math.max(1, Math.floor((width > 0 ? width : 800) / CELL_W))
}

export function slotRows(count: number, cols: number): number {
  return Math.max(1, Math.ceil((count * 1.3) / cols))
}

export function layoutSlots(keys: readonly SystemKey[], width: number, prev: ReadonlyMap<string, number> = new Map()): SlotResult {
  const cols = slotColumns(width)
  const rows = slotRows(keys.length, cols)
  const cells = cols * rows
  const taken = new Set<number>()
  const slots = new Map<string, number>()
  for (const k of keys) {
    const old = prev.get(k.id)
    if (old !== undefined && old < cells && !taken.has(old)) {
      taken.add(old)
      slots.set(k.id, old)
    }
  }
  const fresh = keys.filter((k) => !slots.has(k.id)).sort((a, b) => hashName(a.project + a.id) - hashName(b.project + b.id) || a.id.localeCompare(b.id))
  for (const k of fresh) {
    let slot = hashName(k.project) % cells
    let guard = 0
    while (taken.has(slot) && guard++ < cells) slot = (slot + 1) % cells
    taken.add(slot)
    slots.set(k.id, slot)
  }
  const placements = keys.map((k): Placement => {
    const slot = slots.get(k.id) ?? 0
    const h = hashName(k.id)
    const jx = ((h % 31) - 15) * 0.8
    const jy = (((h >>> 5) % 31) - 15) * 0.6
    return { id: k.id, slot, x: (slot % cols) * CELL_W + CELL_W / 2 + jx, y: Math.floor(slot / cols) * CELL_H + CELL_H / 2 + jy }
  })
  return { placements, slots, cols, rows, width: cols * CELL_W, height: rows * CELL_H }
}

export function fitView(world: { width: number; height: number }, view: { width: number; height: number }): Fit {
  if (view.width <= 0 || view.height <= 0 || world.width <= 0 || world.height <= 0) return { scale: 1, x: 0, y: 0 }
  const scale = Math.min(1, Math.max(MIN_SCALE, Math.min(view.width / world.width, view.height / world.height)))
  return { scale, x: (view.width - world.width * scale) / 2, y: (view.height - world.height * scale) / 2 }
}

export function clampScale(s: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, s))
}

export function zoomAt(fit: Fit, factor: number, px: number, py: number): Fit {
  const scale = clampScale(fit.scale * factor)
  const k = scale / fit.scale
  return { scale, x: px - (px - fit.x) * k, y: py - (py - fit.y) * k }
}

export function sunRadius(group: SessionGroupKey, running: number, busy: boolean): number {
  if (group === 'ended') return 15
  if (group === 'idle') return 19
  const base = group === 'needs' ? 26 : 23
  return base + Math.min(running, 4) * 1.5 + (busy ? 2 : 0)
}

export function planetLayout(subs: readonly { id: string; status: AgentStatus }[], sun: number): PlanetLayout {
  const shown = subs.slice(0, MAX_PLANETS)
  const inner = shown.length > 3 ? Math.ceil(shown.length / 2) : shown.length
  const rings: PlanetRing[] = []
  if (shown.length) rings.push({ ring: 0, radius: sun + 18, seconds: 18 })
  if (shown.length > inner) rings.push({ ring: 1, radius: sun + 34, seconds: 30 })
  const spots = shown.map((s, i): PlanetSpot => {
    const ring: 0 | 1 = i < inner ? 0 : 1
    const count = ring === 0 ? inner : shown.length - inner
    const index = ring === 0 ? i : i - inner
    return { id: s.id, status: s.status, ring, angle: (360 / count) * index + (ring === 1 ? 40 : 0) - 90 }
  })
  return { rings, spots, more: Math.max(0, subs.length - MAX_PLANETS) }
}

export function peerPath(a: { x: number; y: number }, b: { x: number; y: number }, bend = 0.22): LinkPath {
  const mx0 = (a.x + b.x) / 2
  const my0 = (a.y + b.y) / 2
  const dx = b.x - a.x
  const dy = b.y - a.y
  const cx = mx0 - dy * bend
  const cy = my0 + dx * bend
  return { d: `M${round(a.x)} ${round(a.y)} Q${round(cx)} ${round(cy)} ${round(b.x)} ${round(b.y)}`, mx: round(mx0 * 0.5 + cx * 0.5), my: round(my0 * 0.5 + cy * 0.5) }
}

export function agentVar(provider: Provider | undefined): string {
  if (provider === 'codex') return 'var(--ccm-agent-codex)'
  if (provider === 'deepseek') return 'var(--ccm-agent-deepseek)'
  return 'var(--ccm-agent-claude)'
}

export function toolFile(tool: string | undefined, summary: string | undefined): string | null {
  if (!tool || !summary) return null
  if (!/^(Read|Edit|Write|MultiEdit|NotebookEdit)$/.test(tool)) return null
  const trimmed = summary.trim()
  if (!trimmed || /\s/.test(trimmed) || trimmed.length > 300) return null
  const base = trimmed.split(/[\\/]/).filter(Boolean).pop() ?? ''
  if (!base || base.length > 40) return base ? `${base.slice(0, 37)}…` : null
  return base
}

export function idleMinutes(sinceIso: string | undefined, nowMs: number): number | null {
  if (!sinceIso) return null
  const from = Date.parse(sinceIso)
  if (!Number.isFinite(from)) return null
  return Math.max(0, Math.floor((nowMs - from) / 60_000))
}

function round(n: number): number {
  return Math.round(n * 10) / 10
}
