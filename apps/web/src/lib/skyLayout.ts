import type { TerminalStatus } from '@ccm/shared'
import { MAIN_HEIGHT, MAIN_WIDTH, PILL_HEIGHT, PILL_WIDTH, TRAY_WIDTH, type Point } from './layout'
import { groupOf } from './sessionGroups'

export const SKY_DOT_LIMIT = 8

const RING0_CAP = 6
const RING_CAP = 10
const RX0 = 290
const RY0 = 130
const RX_STEP = 260
const RY_STEP = 110
const SYSTEM_GAP = 90
const DOT_STEP = 26
const DOT_PER_ROW = 24
const DOT_DROP = 140
const NUDGE_STEPS = 4
const NUDGE_RAD = Math.PI / 36

export interface SkySystem {
  terminalId: string
  project: string
  mainId: string
  subs: string[]
}

export interface SkyDot {
  id: string
  project: string
}

export interface SkyOrbit {
  id: string
  cx: number
  cy: number
  rx: number
  ry: number
}

export interface SkyInput {
  systems: SkySystem[]
  dots: SkyDot[]
  nudges?: Record<string, number>
  reduced?: boolean
}

export interface SkyResult {
  positions: Map<string, Point>
  orbits: SkyOrbit[]
  dots: Map<string, Point>
  suns: Map<string, Point>
  slots: Map<string, number>
  width: number
  height: number
}

export function hashName(name: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < name.length; i++) {
    h ^= name.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }
  return h >>> 0
}

export function splitDimmed(list: readonly { id: string; status: TerminalStatus }[], limit = SKY_DOT_LIMIT): { full: string[]; dimmed: string[] } {
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

export function ringCount(subs: number): number {
  if (subs <= 0) return 0
  if (subs <= RING0_CAP) return 1
  return 1 + Math.ceil((subs - RING0_CAP) / RING_CAP)
}

function ringSizes(subs: number): number[] {
  const out: number[] = []
  let left = subs
  for (let k = 0; left > 0; k++) {
    const cap = k === 0 ? RING0_CAP : RING_CAP
    const n = Math.min(cap, left)
    out.push(n)
    left -= n
  }
  return out
}

export function systemExtent(subs: number): { w: number; h: number; rings: number } {
  const rings = ringCount(subs)
  if (!rings) return { w: MAIN_WIDTH + SYSTEM_GAP, h: MAIN_HEIGHT + SYSTEM_GAP, rings }
  const rx = RX0 + (rings - 1) * RX_STEP
  const ry = RY0 + (rings - 1) * RY_STEP
  return { w: 2 * (rx + PILL_WIDTH / 2) + SYSTEM_GAP, h: 2 * (ry + PILL_HEIGHT / 2) + SYSTEM_GAP, rings }
}

export function planetAngle(index: number, count: number, ring: number, nudges = 0, reduced = false): number {
  const base = -Math.PI / 2 + (Math.PI / count) + ring * (Math.PI / 8)
  const step = (2 * Math.PI * index) / count
  const nudge = reduced ? 0 : (Math.abs(Math.trunc(nudges)) % NUDGE_STEPS) * NUDGE_RAD
  return base + step + nudge
}

export function gridSize(projects: number): number {
  return Math.max(3, Math.ceil(Math.sqrt(projects)))
}

export function assignSlots(projects: readonly string[]): Map<string, number> {
  const g = gridSize(projects.length)
  const cells = g * g
  const order = [...new Set(projects)].sort((a, b) => hashName(a) - hashName(b) || a.localeCompare(b))
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

export function computeSky(input: SkyInput): SkyResult {
  const reduced = input.reduced ?? false
  const nudges = input.nudges ?? {}
  const byProject = new Map<string, SkySystem[]>()
  for (const s of input.systems) {
    const list = byProject.get(s.project) ?? []
    list.push(s)
    byProject.set(s.project, list)
  }
  const slots = assignSlots([...byProject.keys()])
  const g = gridSize(byProject.size)
  const colW: number[] = Array(g).fill(0)
  const rowH: number[] = Array(g).fill(0)
  const blocks = new Map<string, { w: number; h: number; systems: SkySystem[] }>()
  for (const [project, list] of byProject) {
    const sorted = [...list].sort((a, b) => a.terminalId.localeCompare(b.terminalId))
    let w = 0
    let h = 0
    for (const s of sorted) {
      const e = systemExtent(s.subs.length)
      w += e.w
      h = Math.max(h, e.h)
    }
    blocks.set(project, { w, h, systems: sorted })
    const slot = slots.get(project)!
    const col = slot % g
    const row = Math.floor(slot / g)
    colW[col] = Math.max(colW[col]!, w)
    rowH[row] = Math.max(rowH[row]!, h)
  }
  const colX: number[] = []
  const rowY: number[] = []
  let ax = 0
  for (const w of colW) {
    colX.push(ax)
    ax += w
  }
  let ay = 0
  for (const h of rowH) {
    rowY.push(ay)
    ay += h
  }

  const positions = new Map<string, Point>()
  const suns = new Map<string, Point>()
  const orbits: SkyOrbit[] = []
  for (const [project, block] of blocks) {
    const slot = slots.get(project)!
    const col = slot % g
    const row = Math.floor(slot / g)
    let x = colX[col]! + (colW[col]! - block.w) / 2
    const cy = rowY[row]! + rowH[row]! / 2
    for (const s of block.systems) {
      const e = systemExtent(s.subs.length)
      const cx = x + e.w / 2
      positions.set(s.mainId, { x: cx - MAIN_WIDTH / 2, y: cy - MAIN_HEIGHT / 2 })
      suns.set(s.terminalId, { x: cx, y: cy })
      let at = 0
      ringSizes(s.subs.length).forEach((count, ring) => {
        const rx = RX0 + ring * RX_STEP
        const ry = RY0 + ring * RY_STEP
        orbits.push({ id: `orbit:${s.terminalId}:${ring}`, cx, cy, rx, ry })
        for (let i = 0; i < count; i++) {
          const id = s.subs[at++]!
          const a = planetAngle(i, count, ring, nudges[id] ?? 0, reduced)
          positions.set(id, { x: cx + rx * Math.cos(a) - PILL_WIDTH / 2, y: cy + ry * Math.sin(a) - PILL_HEIGHT / 2 })
        }
      })
      x += e.w
    }
  }

  const width = ax
  const height = ay
  const dots = new Map<string, Point>()
  const dotList = [...input.dots].sort((a, b) => hashName(a.project) - hashName(b.project) || a.id.localeCompare(b.id))
  dotList.forEach((d, i) => {
    dots.set(d.id, { x: (i % DOT_PER_ROW) * DOT_STEP, y: height + DOT_DROP + Math.floor(i / DOT_PER_ROW) * DOT_STEP })
  })
  const dotRows = Math.ceil(dotList.length / DOT_PER_ROW)
  return { positions, orbits, dots, suns, slots, width: Math.max(width, dotList.length ? DOT_STEP * Math.min(dotList.length, DOT_PER_ROW) : 0), height: dotList.length ? height + DOT_DROP + dotRows * DOT_STEP : height }
}

export function trayPoint(sun: Point): Point {
  return { x: sun.x - TRAY_WIDTH / 2, y: sun.y + MAIN_HEIGHT / 2 + 8 }
}

export function dotId(terminalId: string): string {
  return `dot:${terminalId}`
}
