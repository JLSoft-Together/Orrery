import type { AgentStatus, TerminalStatus } from '@ccm/shared'
import { groupOf } from './sessionGroups'

export const ORBIT_DOT_LIMIT = 8
export const RING_FRACTIONS = [0.37, 0.63, 0.87] as const
export const NODE_RADIUS = 17
export const SUB_SHOWN = 6

const SECTOR_BASE = 12
const SECTOR_FILL = 0.8
const NODE_ARC = 40
const DOT_ARC = 16
const LANE_GAP = 26
const SUB_DISTANCE = 29
const NUDGE_STEPS = 4
const NUDGE_RAD = Math.PI / 36
const MARGIN = 34
const FALLBACK_W = 800
const FALLBACK_H = 600

export interface RingSub {
  id: string
  status: AgentStatus
}

export interface RingSession {
  id: string
  project: string
  status: TerminalStatus
  subs: RingSub[]
}

export interface RingsInput {
  sessions: readonly RingSession[]
  width: number
  height: number
  nudges?: Readonly<Record<string, number>>
  reduced?: boolean
}

export interface SubPoint {
  id: string
  status: AgentStatus
  x: number
  y: number
}

export interface RingNode {
  id: string
  project: string
  ring: 0 | 1 | 2
  x: number
  y: number
  dot: boolean
  subs: SubPoint[]
  moreSubs: number
}

export interface Sector {
  project: string
  angle: number
  slot: number
  line: { x1: number; y1: number; x2: number; y2: number }
  label: { x: number; y: number; anchor: 'start' | 'middle' | 'end' }
}

export interface RingsResult {
  cx: number
  cy: number
  half: number
  core: number
  radii: number[]
  sectors: Sector[]
  nodes: RingNode[]
}

export function hashName(name: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < name.length; i++) {
    h ^= name.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }
  return h >>> 0
}

export function splitDimmed(list: readonly { id: string; status: TerminalStatus }[], limit = ORBIT_DOT_LIMIT): { full: string[]; dimmed: string[] } {
  if (list.length <= limit) return { full: list.map((s) => s.id), dimmed: [] }
  const full: string[] = []
  const dimmed: string[] = []
  for (const s of list) {
    const key = groupOf(s.status)
    if (key === 'needs' || key === 'running') full.push(s.id)
    else dimmed.push(s.id)
  }
  return { full, dimmed }
}

export function ringOf(status: TerminalStatus): 0 | 1 | 2 {
  const g = groupOf(status)
  return g === 'needs' ? 0 : g === 'running' ? 1 : 2
}

export function sectorCount(projects: number): number {
  return SECTOR_BASE * Math.max(1, Math.ceil(projects / SECTOR_BASE))
}

export function assignSectors(projects: readonly string[]): Map<string, number> {
  const unique = [...new Set(projects)]
  const cells = sectorCount(unique.length)
  const order = unique.sort((a, b) => hashName(a) - hashName(b) || a.localeCompare(b))
  const taken = new Set<number>()
  const out = new Map<string, number>()
  for (const p of order) {
    let slot = hashName(p) % cells
    while (taken.has(slot)) slot = (slot + 1) % cells
    taken.add(slot)
    out.set(p, slot)
  }
  return out
}

export function sectorAngle(slot: number, cells: number): number {
  return -Math.PI / 2 + (slot * 2 * Math.PI) / cells
}

export function subAngle(sessionId: string, index: number, count: number, nudges = 0, reduced = false): number {
  const base = ((hashName(sessionId) % 360) * Math.PI) / 180
  const nudge = reduced ? 0 : (Math.abs(Math.trunc(nudges)) % NUDGE_STEPS) * NUDGE_RAD
  return base + (2 * Math.PI * index) / Math.max(1, count) + nudge
}

function anchorFor(cos: number): 'start' | 'middle' | 'end' {
  if (Math.abs(cos) < 0.3) return 'middle'
  return cos > 0 ? 'start' : 'end'
}

export function computeRings(input: RingsInput): RingsResult {
  const width = input.width > 0 ? input.width : FALLBACK_W
  const height = input.height > 0 ? input.height : FALLBACK_H
  const reduced = input.reduced ?? false
  const nudges = input.nudges ?? {}
  const cx = width / 2
  const cy = height / 2
  const half = Math.max(120, Math.min(width, height) / 2 - MARGIN)
  const radii = RING_FRACTIONS.map((f) => f * half)
  const core = Math.min(34, half * 0.12)

  const projects = [...new Set(input.sessions.map((s) => s.project))]
  const slots = assignSectors(projects)
  const cells = sectorCount(projects.length)
  const span = ((2 * Math.PI) / cells) * SECTOR_FILL

  const dimmed = new Set(splitDimmed(input.sessions).dimmed)
  const groups = new Map<string, RingSession[]>()
  for (const s of input.sessions) {
    const ring = dimmed.has(s.id) ? 2 : ringOf(s.status)
    const key = `${s.project}\u0000${ring}`
    const list = groups.get(key) ?? []
    list.push(s)
    groups.set(key, list)
  }

  const nodes: RingNode[] = []
  for (const [key, list] of groups) {
    const [project, ringText] = key.split('\u0000') as [string, string]
    const ring = Number(ringText) as 0 | 1 | 2
    const slot = slots.get(project)!
    const center = sectorAngle(slot, cells)
    const r = radii[ring]!
    const sorted = [...list].sort((a, b) => a.id.localeCompare(b.id))
    const arc = sorted.every((s) => dimmed.has(s.id)) ? DOT_ARC : NODE_ARC
    const perLane = Math.max(1, Math.floor((span * r) / arc))
    sorted.forEach((s, i) => {
      const lane = Math.floor(i / perLane)
      const inLane = Math.min(perLane, sorted.length - lane * perLane)
      const j = i - lane * perLane
      const step = inLane > 1 ? Math.min(span / (inLane - 1), arc / r) : 0
      const angle = center + (j - (inLane - 1) / 2) * step
      const offset = lane === 0 ? 0 : (lane % 2 === 1 ? 1 : -1) * Math.ceil(lane / 2) * LANE_GAP
      const rr = r + offset
      const x = cx + rr * Math.cos(angle)
      const y = cy + rr * Math.sin(angle)
      const isDot = dimmed.has(s.id)
      const shown = isDot ? [] : s.subs.slice(0, SUB_SHOWN)
      const subs = shown.map((sub, k): SubPoint => {
        const a = subAngle(s.id, k, shown.length, nudges[sub.id] ?? 0, reduced)
        return { id: sub.id, status: sub.status, x: x + SUB_DISTANCE * Math.cos(a), y: y + SUB_DISTANCE * Math.sin(a) }
      })
      nodes.push({ id: s.id, project, ring, x, y, dot: isDot, subs, moreSubs: isDot ? 0 : Math.max(0, s.subs.length - SUB_SHOWN) })
    })
  }
  nodes.sort((a, b) => a.ring - b.ring || a.id.localeCompare(b.id))

  const sectors: Sector[] = projects
    .map((project): Sector => {
      const slot = slots.get(project)!
      const angle = sectorAngle(slot, cells)
      const cos = Math.cos(angle)
      const sin = Math.sin(angle)
      return {
        project,
        slot,
        angle,
        line: { x1: cx + core * cos, y1: cy + core * sin, x2: cx + half * cos, y2: cy + half * sin },
        label: { x: cx + (half + 12) * cos, y: cy + (half + 12) * sin + 4, anchor: anchorFor(cos) },
      }
    })
    .sort((a, b) => a.slot - b.slot)

  return { cx, cy, half, core, radii, sectors, nodes }
}

export type NavKey = 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight'

export function initialSelection(nodes: readonly Pick<RingNode, 'id'>[]): string | null {
  return nodes[0]?.id ?? null
}

export function navigate(nodes: readonly Pick<RingNode, 'id' | 'x' | 'y'>[], current: string | null, key: NavKey): string | null {
  const from = nodes.find((n) => n.id === current)
  if (!from) return initialSelection(nodes)
  const dir = key === 'ArrowUp' ? { x: 0, y: -1 } : key === 'ArrowDown' ? { x: 0, y: 1 } : key === 'ArrowLeft' ? { x: -1, y: 0 } : { x: 1, y: 0 }
  let best: string = from.id
  let bestScore = Infinity
  for (const n of nodes) {
    if (n.id === from.id) continue
    const dx = n.x - from.x
    const dy = n.y - from.y
    const along = dx * dir.x + dy * dir.y
    if (along <= 0) continue
    const across = Math.abs(dx * dir.y - dy * dir.x)
    const score = along + 2 * across
    if (score < bestScore || (score === bestScore && n.id < best)) {
      best = n.id
      bestScore = score
    }
  }
  return best
}
