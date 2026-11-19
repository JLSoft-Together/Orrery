import { boxes, clampRatio, hasGroups, isGroup, isLeaf, leaves, MIN_PANE, type LayoutNode, type SplitDir } from './splitLayout'

export type LayoutPreset = 'one' | 'cols2' | 'cols3' | 'grid2' | 'mainCol' | 'topTwo'
export const LAYOUT_PRESETS: readonly LayoutPreset[] = ['one', 'cols2', 'cols3', 'grid2', 'mainCol', 'topTwo']
export const MAIN_RATIO = 0.6

export function lineOf(items: readonly LayoutNode[], dir: SplitDir): LayoutNode | null {
  if (!items.length) return null
  if (items.length === 1) return items[0]!
  const cut = Math.floor(items.length / 2)
  const a = lineOf(items.slice(0, cut), dir)!
  const b = lineOf(items.slice(cut), dir)!
  return { dir, ratio: clampRatio(cut / items.length), a, b }
}

export function chunk<T>(list: readonly T[], parts: number): T[][] {
  const n = Math.max(1, Math.min(parts, list.length))
  const out: T[][] = []
  let at = 0
  for (let i = 0; i < n; i++) {
    const size = Math.ceil((list.length - at) / (n - i))
    out.push(list.slice(at, at + size))
    at += size
  }
  return out
}

const leafs = (keys: readonly string[]): LayoutNode[] => keys.map((pane) => ({ pane }))

function columns(keys: readonly string[], n: number): LayoutNode | null {
  return lineOf(
    chunk(keys, n).map((col) => lineOf(leafs(col), 'col')!),
    'row',
  )
}

function rowsOf(keys: readonly string[], perRow: number): LayoutNode | null {
  const rows: string[][] = []
  for (let i = 0; i < keys.length; i += perRow) rows.push(keys.slice(i, i + perRow))
  return lineOf(
    rows.map((r) => lineOf(leafs(r), 'row')!),
    'col',
  )
}

export function orderKeys(keys: readonly string[], focus?: string): string[] {
  return focus && keys.includes(focus) ? [focus, ...keys.filter((k) => k !== focus)] : [...keys]
}

export function presetLayout(preset: LayoutPreset, keys: readonly string[], focus?: string): LayoutNode | null {
  const list = [...new Set(keys)]
  if (!list.length) return null
  if (list.length === 1) return { pane: list[0]! }
  switch (preset) {
    case 'one':
      return null
    case 'cols2':
      return columns(list, 2)
    case 'cols3':
      return columns(list, 3)
    case 'grid2':
      return rowsOf(list, 2)
    case 'mainCol': {
      const [main, ...rest] = orderKeys(list, focus)
      return { dir: 'row', ratio: MAIN_RATIO, a: { pane: main! }, b: lineOf(leafs(rest), 'col')! }
    }
    case 'topTwo': {
      const [top, ...rest] = orderKeys(list, focus)
      return { dir: 'col', ratio: 0.5, a: { pane: top! }, b: lineOf(leafs(rest), 'row')! }
    }
  }
}

function weight(n: LayoutNode, dir: SplitDir): number {
  if ('dir' in n && n.dir === dir) return weight(n.a, dir) + weight(n.b, dir)
  return 1
}

export function evenLayout(n: LayoutNode): LayoutNode {
  if (isLeaf(n)) return n
  if (isGroup(n)) return { ...n, node: evenLayout(n.node) }
  const wa = weight(n.a, n.dir)
  const wb = weight(n.b, n.dir)
  return { dir: n.dir, ratio: clampRatio(wa / (wa + wb)), a: evenLayout(n.a), b: evenLayout(n.b) }
}

export function presetAllowed(root: LayoutNode): boolean {
  return !hasGroups(root) && leaves(root).length > 1
}

export function layoutFits(root: LayoutNode, size: { w: number; h: number }, min: { w: number; h: number } = MIN_PANE, gap = 8): boolean {
  if (!size.w || !size.h) return true
  for (const b of boxes(root).values()) if (b.w * size.w - gap < min.w || b.h * size.h - gap < min.h) return false
  return true
}

export function sameLayout(a: LayoutNode, b: LayoutNode): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}
