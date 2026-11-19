/** Split tree of one workspace tab; leaves are pane keys. Every operation returns a new tree. */
export type SplitDir = 'row' | 'col'
export type LayoutNode = { pane: string } | { dir: SplitDir; ratio: number; a: LayoutNode; b: LayoutNode } | GroupNode
export type GroupNode = { group: string; node: LayoutNode; name?: string; tone?: number; folded?: boolean }
export type GroupMeta = Pick<GroupNode, 'name' | 'tone' | 'folded'>
export type SplitNode = { dir: SplitDir; ratio: number; a: LayoutNode; b: LayoutNode }
export type FocusDir = 'left' | 'right' | 'up' | 'down'

export interface Box {
  x: number
  y: number
  w: number
  h: number
}

export interface Divider {
  /** Child choices from the root, e.g. "ab" = root.a.b. */
  path: string
  dir: SplitDir
  ratio: number
  box: Box
  group?: string
}

export const MIN_RATIO = 0.15
export const MAX_RATIO = 0.85
const UNIT: Box = { x: 0, y: 0, w: 1, h: 1 }

export const isLeaf = (n: LayoutNode): n is { pane: string } => 'pane' in n
export const isGroup = (n: LayoutNode): n is GroupNode => 'group' in n
export const isSplit = (n: LayoutNode): n is SplitNode => 'dir' in n
export const clampRatio = (r: number): number => Math.min(MAX_RATIO, Math.max(MIN_RATIO, Number.isFinite(r) ? r : 0.5))

export function leaves(n: LayoutNode): string[] {
  return isLeaf(n) ? [n.pane] : isGroup(n) ? leaves(n.node) : [...leaves(n.a), ...leaves(n.b)]
}

export function splitLeaf(n: LayoutNode, target: string, added: string, dir: SplitDir): LayoutNode {
  if (isLeaf(n)) return n.pane === target ? { dir, ratio: 0.5, a: n, b: { pane: added } } : n
  if (isGroup(n)) return { ...n, node: splitLeaf(n.node, target, added, dir) }
  return { ...n, a: splitLeaf(n.a, target, added, dir), b: splitLeaf(n.b, target, added, dir) }
}

/** The tree without that leaf (its sibling takes the parent's place); null when it was the last one. */
export function removeLeaf(n: LayoutNode, target: string): LayoutNode | null {
  if (isLeaf(n)) return n.pane === target ? null : n
  if (isGroup(n)) {
    const inner = removeLeaf(n.node, target)
    return inner ? (inner === n.node ? n : { ...n, node: inner }) : null
  }
  const a = removeLeaf(n.a, target)
  const b = removeLeaf(n.b, target)
  if (!a) return b
  if (!b) return a
  return a === n.a && b === n.b ? n : { ...n, a, b }
}

export type Edge = 'left' | 'right' | 'top' | 'bottom'

// 24 columns of 13px mono plus the well padding: narrower panes stop being usable for an agent TUI.
export const MIN_PANE = { w: 220, h: 140 } as const
export const GROUP_CHROME = { head: 30, pad: 4 } as const

/**
 * Where a new pane joins the tab: the roomiest pane (`prefer` wins a tie), split along its longer side, the other
 * side when that one would be too small. Two, three, four panes grow into halves, a half split again, then a 2 × 2 grid.
 * Null when every pane is already too small to halve. An unmeasured canvas (0 × 0) splits side by side.
 */
export function dockSpot(n: LayoutNode, prefer: string | undefined, size: { w: number; h: number }, min = MIN_PANE): { key: string; dir: SplitDir } | null {
  const all = boxes(n)
  const area = (k: string): number => all.get(k)!.w * all.get(k)!.h
  const order = [...all.keys()].sort((a, b) => {
    const d = area(b) - area(a)
    return Math.abs(d) > 1e-6 ? d : a === prefer ? -1 : b === prefer ? 1 : 0
  })
  for (const key of order) {
    if (!size.w || !size.h) return { key, dir: 'row' }
    const box = all.get(key)!
    const w = box.w * size.w
    const h = box.h * size.h
    const want: SplitDir = w >= h * 1.4 ? 'row' : 'col'
    const fits = (d: SplitDir): boolean => (d === 'row' ? w / 2 >= min.w : h / 2 >= min.h)
    if (fits(want)) return { key, dir: want }
    const other: SplitDir = want === 'row' ? 'col' : 'row'
    if (fits(other)) return { key, dir: other }
  }
  return null
}

/** Leaf `target` becomes a split holding `sub` on that edge and itself on the other; `n` itself when target is missing. */
export function dockTree(n: LayoutNode, target: string, sub: LayoutNode, edge: Edge): LayoutNode {
  if (isLeaf(n)) {
    if (n.pane !== target) return n
    const first = edge === 'left' || edge === 'top'
    return { dir: edge === 'left' || edge === 'right' ? 'row' : 'col', ratio: 0.5, a: first ? sub : n, b: first ? n : sub }
  }
  if (isGroup(n)) {
    const inner = dockTree(n.node, target, sub, edge)
    return inner === n.node ? n : { ...n, node: inner }
  }
  const a = dockTree(n.a, target, sub, edge)
  const b = dockTree(n.b, target, sub, edge)
  return a === n.a && b === n.b ? n : { ...n, a, b }
}

/** Leaf `src` leaves its slot and docks onto `edge` of leaf `target`; `n` itself when that is not a move. */
export function movePane(n: LayoutNode, src: string, target: string, edge: Edge): LayoutNode {
  const keys = leaves(n)
  if (src === target || !keys.includes(src) || !keys.includes(target)) return n
  const rest = removeLeaf(n, src)
  return rest ? dockTree(rest, target, { pane: src }, edge) : n
}

/** The two leaves trade places; every split keeps its direction and ratio. */
export function swapPanes(n: LayoutNode, a: string, b: string): LayoutNode {
  const keys = leaves(n)
  if (a === b || !keys.includes(a) || !keys.includes(b)) return n
  return mapLeaves(n, (k) => (k === a ? b : k === b ? a : k))
}

export interface Rect {
  left: number
  top: number
  right: number
  bottom: number
}

/** Edge of the rect nearest to the point, distances taken as fractions of width / height (diagonal split). */
export function edgeAt(r: Rect, x: number, y: number): Edge {
  const w = Math.max(1, r.right - r.left)
  const h = Math.max(1, r.bottom - r.top)
  const d: Array<[Edge, number]> = [
    ['left', (x - r.left) / w],
    ['right', (r.right - x) / w],
    ['top', (y - r.top) / h],
    ['bottom', (r.bottom - y) / h],
  ]
  return d.reduce((best, cur) => (cur[1] < best[1] ? cur : best))[0]
}

/** Pane under the point and the edge to dock on; null over a gap or over `exclude`. */
export function dropTargetAt(rects: ReadonlyMap<string, Rect>, x: number, y: number, exclude: string | null): { key: string; edge: Edge } | null {
  for (const [key, r] of rects) {
    if (x < r.left || x > r.right || y < r.top || y > r.bottom) continue
    return key === exclude ? null : { key, edge: edgeAt(r, x, y) }
  }
  return null
}

export function setRatio(n: LayoutNode, path: string, ratio: number): LayoutNode {
  if (isLeaf(n)) return n
  if (isGroup(n)) return { ...n, node: setRatio(n.node, path, ratio) }
  if (!path) return { ...n, ratio: clampRatio(ratio) }
  const side = path[0] === 'a' ? 'a' : 'b'
  return { ...n, [side]: setRatio(n[side], path.slice(1), ratio) }
}

function childBoxes(n: SplitNode, box: Box): [Box, Box] {
  if (n.dir === 'row') {
    const w = box.w * n.ratio
    return [
      { ...box, w },
      { ...box, x: box.x + w, w: box.w - w },
    ]
  }
  const h = box.h * n.ratio
  return [
    { ...box, h },
    { ...box, y: box.y + h, h: box.h - h },
  ]
}

/** Each pane's rectangle as fractions of the tab's area. */
export function boxes(n: LayoutNode, box: Box = UNIT, out = new Map<string, Box>()): Map<string, Box> {
  if (isLeaf(n)) return out.set(n.pane, box)
  if (isGroup(n)) return boxes(n.node, box, out)
  const [a, b] = childBoxes(n, box)
  boxes(n.a, a, out)
  boxes(n.b, b, out)
  return out
}

export function dividers(n: LayoutNode, box: Box = UNIT, path = '', group?: string): Divider[] {
  if (isLeaf(n)) return []
  if (isGroup(n)) return dividers(n.node, box, path, n.group)
  const [a, b] = childBoxes(n, box)
  const own: Divider = group === undefined ? { path, dir: n.dir, ratio: n.ratio, box } : { path, dir: n.dir, ratio: n.ratio, box, group }
  return [own, ...dividers(n.a, a, `${path}a`, group), ...dividers(n.b, b, `${path}b`, group)]
}

const EPS = 1e-6

/** Nearest pane on that side that overlaps the current one across the other axis. */
export function neighbor(n: LayoutNode, from: string, dir: FocusDir): string | null {
  const all = boxes(n)
  const src = all.get(from)
  if (!src) return null
  let best: string | null = null
  let bestDist = Infinity
  for (const [key, b] of all) {
    if (key === from) continue
    const horizontal = dir === 'left' || dir === 'right'
    const overlaps = horizontal ? b.y < src.y + src.h - EPS && b.y + b.h > src.y + EPS : b.x < src.x + src.w - EPS && b.x + b.w > src.x + EPS
    if (!overlaps) continue
    const dist =
      dir === 'right' ? b.x - (src.x + src.w) : dir === 'left' ? src.x - (b.x + b.w) : dir === 'down' ? b.y - (src.y + src.h) : src.y - (b.y + b.h)
    if (dist < -EPS) continue
    // Ties (several panes touching that edge) go to the one sharing most of the current pane's span.
    const center = horizontal ? Math.abs(b.y + b.h / 2 - (src.y + src.h / 2)) : Math.abs(b.x + b.w / 2 - (src.x + src.w / 2))
    const score = dist + center * 1e-3
    if (score < bestDist) {
      bestDist = score
      best = key
    }
  }
  return best
}

export function isLayout(v: unknown, depth = 0): v is LayoutNode {
  if (!v || typeof v !== 'object' || depth > 16) return false
  const n = v as Record<string, unknown>
  if (typeof n.pane === 'string') return true
  if (typeof n.group === 'string') return isLayout(n.node, depth + 1)
  return (n.dir === 'row' || n.dir === 'col') && typeof n.ratio === 'number' && isLayout(n.a, depth + 1) && isLayout(n.b, depth + 1)
}

export function mapLeaves(n: LayoutNode, fn: (key: string) => string): LayoutNode {
  if (isLeaf(n)) return { pane: fn(n.pane) }
  if (isGroup(n)) return { ...metaOf(n), group: n.group, node: mapLeaves(n.node, fn) }
  return { dir: n.dir, ratio: n.ratio, a: mapLeaves(n.a, fn), b: mapLeaves(n.b, fn) }
}

export type SameProject = (a: string, b: string) => boolean
const strict: SameProject = (a, b) => a === b

export function hasGroups(n: LayoutNode): boolean {
  return isGroup(n) || (isSplit(n) && (hasGroups(n.a) || hasGroups(n.b)))
}

export function groupNodes(n: LayoutNode, out: GroupNode[] = []): GroupNode[] {
  if (isGroup(n)) out.push(n)
  else if (isSplit(n)) {
    groupNodes(n.a, out)
    groupNodes(n.b, out)
  }
  return out
}

export function groupsOf(n: LayoutNode): Array<GroupMeta & { project: string; keys: string[] }> {
  return groupNodes(n).map((g) => ({ ...metaOf(g), project: g.group, keys: leaves(g.node) }))
}

export function groupOf(n: LayoutNode, key: string): string | undefined {
  return groupNodes(n).find((g) => leaves(g.node).includes(key))?.group
}

export function findGroup(n: LayoutNode, project: string, eq: SameProject = strict): GroupNode | undefined {
  return groupNodes(n).find((g) => eq(g.group, project))
}

export function groupBoxes(n: LayoutNode, box: Box = UNIT, out = new Map<string, Box>()): Map<string, Box> {
  if (isGroup(n)) return out.set(n.group, box)
  if (isSplit(n)) {
    const [a, b] = childBoxes(n, box)
    groupBoxes(n.a, a, out)
    groupBoxes(n.b, b, out)
  }
  return out
}

function swapNode(n: LayoutNode, target: LayoutNode, by: LayoutNode): LayoutNode {
  if (n === target) return by
  if (!isSplit(n)) return n
  const a = swapNode(n.a, target, by)
  const b = swapNode(n.b, target, by)
  return a === n.a && b === n.b ? n : { ...n, a, b }
}

function dropNode(n: LayoutNode, target: LayoutNode): LayoutNode | null {
  if (n === target) return null
  if (!isSplit(n)) return n
  const a = dropNode(n.a, target)
  const b = dropNode(n.b, target)
  if (!a) return b
  if (!b) return a
  return a === n.a && b === n.b ? n : { ...n, a, b }
}

export function ungrouped(n: LayoutNode): LayoutNode {
  if (isGroup(n)) return ungrouped(n.node)
  return isSplit(n) ? { ...n, a: ungrouped(n.a), b: ungrouped(n.b) } : n
}

function beside(near: LayoutNode, sub: LayoutNode, edge: Edge): LayoutNode {
  const first = edge === 'left' || edge === 'top'
  return { dir: edge === 'left' || edge === 'right' ? 'row' : 'col', ratio: 0.5, a: first ? sub : near, b: first ? near : sub }
}

export function settleGroups(n: LayoutNode, projectOf: (key: string) => string, eq: SameProject = strict): { root: LayoutNode; project?: string } {
  const uniform = (m: LayoutNode): string | null => {
    const keys = leaves(m)
    const first = projectOf(keys[0]!)
    return keys.every((k) => eq(projectOf(k), first)) ? first : null
  }
  if (!hasGroups(n)) {
    const one = uniform(n)
    if (one !== null) return { root: n, project: one }
  }
  const wrap = (m: LayoutNode): LayoutNode => {
    if (isGroup(m)) return { ...metaOf(m), group: m.group, node: ungrouped(m.node) }
    if (!hasGroups(m)) {
      const one = uniform(m)
      if (one !== null) return { group: one, node: m }
    }
    return isSplit(m) ? { ...m, a: wrap(m.a), b: wrap(m.b) } : m
  }
  let root = wrap(n)
  for (;;) {
    const list = groupNodes(root)
    let pair: [GroupNode, GroupNode] | null = null
    for (let j = 1; j < list.length && !pair; j++) {
      const i = list.findIndex((g, at) => at < j && eq(g.group, list[j]!.group))
      if (i >= 0) pair = [list[i]!, list[j]!]
    }
    if (!pair) break
    const [keep, dup] = pair
    const rest = dropNode(root, dup)!
    const last = leaves(keep.node).at(-1)!
    root = swapNode(rest, keep, { ...metaOf(keep), group: keep.group, node: dockTree(keep.node, last, dup.node, 'right') })
  }
  const groups = groupNodes(root)
  if (groups.length === 1) return { root: groups[0]!.node, project: groups[0]!.group }
  return { root }
}

export function dockGroup(n: LayoutNode, home: string, near: string, project: string, sub: LayoutNode, edge: Edge, eq: SameProject = strict): LayoutNode {
  const root: LayoutNode = hasGroups(n) ? n : { group: home, node: n }
  const own = findGroup(root, project, eq)
  const flat = ungrouped(sub)
  if (own) {
    const last = leaves(own.node).at(-1)!
    return swapNode(root, own, { ...metaOf(own), group: own.group, node: dockTree(own.node, last, flat, edge === 'left' || edge === 'right' ? 'right' : 'bottom') })
  }
  const at = groupNodes(root).find((g) => leaves(g.node).includes(near))
  if (!at) return n
  return swapNode(root, at, beside(at, { group: project, node: flat }, edge))
}

export function takeGroup(n: LayoutNode, project: string, eq: SameProject = strict): { rest: LayoutNode | null; node: LayoutNode } | null {
  const g = findGroup(n, project, eq)
  return g ? { rest: dropNode(n, g), node: g.node } : null
}

export function moveGroup(n: LayoutNode, project: string, near: string, edge: Edge, eq: SameProject = strict): LayoutNode {
  const src = findGroup(n, project, eq)
  const dst = findGroup(n, near, eq)
  if (!src || !dst || src === dst) return n
  const rest = dropNode(n, src)
  return rest ? swapNode(rest, dst, beside(dst, src, edge)) : n
}

export const GROUP_TONES = 8
export const GROUP_NAME_MAX = 40

export function metaOf(g: GroupMeta): GroupMeta {
  const out: GroupMeta = {}
  const name = typeof g.name === 'string' ? g.name.replace(/[\x00-\x1f\x7f]/g, ' ').trim().slice(0, GROUP_NAME_MAX) : ''
  if (name) out.name = name
  if (typeof g.tone === 'number' && Number.isInteger(g.tone) && g.tone >= 0 && g.tone < GROUP_TONES) out.tone = g.tone
  if (g.folded === true) out.folded = true
  return out
}

export function cleanGroups(n: LayoutNode): LayoutNode {
  if (isLeaf(n)) return n
  if (isGroup(n)) return { ...metaOf(n), group: n.group, node: cleanGroups(n.node) }
  return { dir: n.dir, ratio: clampRatio(n.ratio), a: cleanGroups(n.a), b: cleanGroups(n.b) }
}

export function setGroupMeta(n: LayoutNode, project: string, patch: GroupMeta, eq: SameProject = strict): LayoutNode {
  const g = findGroup(n, project, eq)
  if (!g) return n
  return swapNode(n, g, { ...metaOf({ ...metaOf(g), ...patch }), group: g.group, node: g.node })
}

function skeleton(n: LayoutNode): LayoutNode {
  if (isGroup(n)) return { pane: n.group }
  return isSplit(n) ? { ...n, a: skeleton(n.a), b: skeleton(n.b) } : n
}

export function neighborGroup(n: LayoutNode, project: string, dir: FocusDir): string | null {
  return hasGroups(n) ? neighbor(skeleton(n), project, dir) : null
}

const allFolded = (n: LayoutNode): boolean => {
  if (isGroup(n)) return !!n.folded
  return isSplit(n) && allFolded(n.a) && allFolded(n.b)
}

const foldCount = (n: LayoutNode, dir: SplitDir): number => (isSplit(n) && n.dir === dir ? foldCount(n.a, dir) + foldCount(n.b, dir) : 1)

export function foldLayout(n: LayoutNode, size: { w: number; h: number }, strip: { w: number; h: number }): { root: LayoutNode; locked: string[] } {
  const locked: string[] = []
  if (!hasGroups(n) || !size.w || !size.h) return { root: n, locked }
  const walk = (m: LayoutNode, box: Box, path: string): LayoutNode => {
    if (!isSplit(m)) return m
    const fa = allFolded(m.a)
    const fb = allFolded(m.b)
    let ratio = m.ratio
    if (fa !== fb) {
      const span = m.dir === 'row' ? box.w : box.h
      const s = (m.dir === 'row' ? strip.w : strip.h) * foldCount(fa ? m.a : m.b, m.dir)
      const part = Math.min(0.9, Math.max(0.01, s / Math.max(1, span)))
      ratio = fa ? part : 1 - part
      locked.push(path)
    }
    const next: SplitNode = { ...m, ratio }
    const [ba, bb] = childBoxes(next, box)
    return { ...next, a: walk(m.a, ba, `${path}a`), b: walk(m.b, bb, `${path}b`) }
  }
  return { root: walk(n, { x: 0, y: 0, w: size.w, h: size.h }, ''), locked }
}

export function foldedKeys(n: LayoutNode): Set<string> {
  return new Set(groupNodes(n).flatMap((g) => (g.folded ? leaves(g.node) : [])))
}
