export interface SunburstArc {
  index: number
  ring: number
  start: number
  end: number
  parent: number | null
}

const TAU = Math.PI * 2

export function layoutArcs(values: number[], start: number, end: number, ring: number, parent: number | null = null): SunburstArc[] {
  const items = values
    .map((value, index) => ({ value, index }))
    .filter((o) => Number.isFinite(o.value) && o.value > 0)
    .sort((a, b) => b.value - a.value)
  if (!items.length || end <= start) return []
  let total = 0
  for (const o of items) total += o.value
  const span = end - start
  const out: SunburstArc[] = []
  let a = start
  items.forEach((o, i) => {
    const b = i === items.length - 1 ? end : a + (o.value / total) * span
    out.push({ index: o.index, ring, start: a, end: b, parent })
    a = b
  })
  return out
}

export function layoutRings(values: number[], sizes: number[][], parentSizes: number[]): SunburstArc[] {
  const first = layoutArcs(values, 0, TAU, 0)
  const out = [...first]
  for (const arc of first) {
    const kids = sizes[arc.index]
    if (!kids?.length) continue
    let sum = 0
    for (const k of kids) sum += k > 0 ? k : 0
    const parent = parentSizes[arc.index] ?? 0
    if (!(sum > 0) || !(parent > 0)) continue
    const fill = Math.min(1, sum / parent)
    out.push(...layoutArcs(kids, arc.start, arc.start + (arc.end - arc.start) * fill, 1, arc.index))
  }
  return out
}

export interface SunburstGeometry {
  cx: number
  cy: number
  inner: number
  ringWidth: number
}

export function arcAt(arcs: SunburstArc[], g: SunburstGeometry, x: number, y: number): SunburstArc | null {
  const dx = x - g.cx
  const dy = y - g.cy
  const r = Math.hypot(dx, dy)
  if (r < g.inner) return null
  const ring = Math.floor((r - g.inner) / g.ringWidth)
  let ang = Math.atan2(dy, dx)
  if (ang < 0) ang += TAU
  for (const a of arcs) if (a.ring === ring && ang >= a.start && ang < a.end) return a
  return null
}

export { TAU }
