import type { TreeRow } from './tree'

export interface TreeWidthOptions {
  indent: number
  measure: (text: string, bold: boolean) => number
  extra?: (row: TreeRow) => number
}

const BASE = 4
const LEAD = 14 + 4
const ICON = 16 + 4
const TAIL = 28

export function rowContentWidth(row: TreeRow, opts: TreeWidthOptions): number {
  const left = BASE + Math.max(0, row.depth - 1) * opts.indent
  const icon = row.depth > 0 ? ICON : 0
  const text = opts.measure(row.name, row.depth === 0)
  return Math.ceil(left + LEAD + icon + text + TAIL + (opts.extra?.(row) ?? 0))
}

export function treeContentWidth(rows: readonly TreeRow[], opts: TreeWidthOptions): number {
  let widest = 0
  for (const row of rows) {
    if (row.depth === 0) continue
    const w = rowContentWidth(row, opts)
    if (w > widest) widest = w
  }
  return widest
}

export function textMeasurer(font: () => string): (text: string, bold: boolean) => number {
  const cache = new Map<string, number>()
  let canvas: CanvasRenderingContext2D | null = null
  let lastFont = ''
  return (text, bold) => {
    const f = font()
    if (f !== lastFont) {
      cache.clear()
      lastFont = f
      canvas = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d')
    }
    const key = `${bold ? 'b' : 'n'}${text}`
    const hit = cache.get(key)
    if (hit !== undefined) return hit
    let w = text.length * 7
    if (canvas) {
      canvas.font = bold ? `600 ${f}` : f
      w = canvas.measureText(text).width
    }
    cache.set(key, w)
    return w
  }
}
