export interface TreemapRect {
  x: number
  y: number
  w: number
  h: number
}

export interface TreemapCell extends TreemapRect {
  index: number
}

function worst(areas: number[], side: number): number {
  let sum = 0
  let max = 0
  let min = Infinity
  for (const a of areas) {
    sum += a
    if (a > max) max = a
    if (a < min) min = a
  }
  const s2 = side * side
  return Math.max((s2 * max) / (sum * sum), (sum * sum) / (s2 * min))
}

export function squarify(values: number[], frame: TreemapRect): TreemapCell[] {
  if (frame.w <= 0 || frame.h <= 0) return []
  const items = values
    .map((value, index) => ({ value, index }))
    .filter((o) => Number.isFinite(o.value) && o.value > 0)
    .sort((a, b) => b.value - a.value)
  if (!items.length) return []
  let total = 0
  for (const o of items) total += o.value
  const scale = (frame.w * frame.h) / total
  const areas = items.map((o) => o.value * scale)
  const out: TreemapCell[] = []
  let rect = { ...frame }
  let start = 0

  const place = (from: number, to: number, last: boolean): void => {
    let sum = 0
    for (let i = from; i < to; i++) sum += areas[i] ?? 0
    if (rect.w >= rect.h) {
      const cw = last ? rect.w : Math.min(rect.w, sum / rect.h)
      let y = rect.y
      for (let i = from; i < to; i++) {
        const ch = i === to - 1 ? rect.y + rect.h - y : (areas[i] ?? 0) / cw
        out.push({ index: items[i]!.index, x: rect.x, y, w: cw, h: ch })
        y += ch
      }
      rect = { x: rect.x + cw, y: rect.y, w: rect.w - cw, h: rect.h }
    } else {
      const rh = last ? rect.h : Math.min(rect.h, sum / rect.w)
      let x = rect.x
      for (let i = from; i < to; i++) {
        const cw = i === to - 1 ? rect.x + rect.w - x : (areas[i] ?? 0) / rh
        out.push({ index: items[i]!.index, x, y: rect.y, w: cw, h: rh })
        x += cw
      }
      rect = { x: rect.x, y: rect.y + rh, w: rect.w, h: rect.h - rh }
    }
  }

  for (let i = 1; i <= areas.length; i++) {
    if (i === areas.length) {
      place(start, i, true)
      break
    }
    const side = Math.min(rect.w, rect.h)
    if (worst(areas.slice(start, i + 1), side) > worst(areas.slice(start, i), side)) {
      place(start, i, false)
      start = i
    }
  }
  return out
}

export function cellAt(cells: TreemapCell[], x: number, y: number): TreemapCell | null {
  for (let i = cells.length - 1; i >= 0; i--) {
    const c = cells[i]
    if (c && x >= c.x && x < c.x + c.w && y >= c.y && y < c.y + c.h) return c
  }
  return null
}

export function inset(c: TreemapRect, by: number): TreemapRect {
  const b = Math.min(by, c.w / 2, c.h / 2)
  return { x: c.x + b, y: c.y + b, w: c.w - 2 * b, h: c.h - 2 * b }
}
