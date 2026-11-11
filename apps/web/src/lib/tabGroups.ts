export const MORE_BUTTON_W = 56
export const TAB_WIDTH_FALLBACK = 150

export function groupKeys(keys: readonly string[], groupOf: (key: string) => string): string[] {
  const order: string[] = []
  const members = new Map<string, string[]>()
  for (const k of keys) {
    const g = groupOf(k)
    if (!members.has(g)) {
      members.set(g, [])
      order.push(g)
    }
    members.get(g)!.push(k)
  }
  return order.flatMap((g) => members.get(g)!)
}

export function groupStarts(keys: readonly string[], groupOf: (key: string) => string): Set<string> {
  const out = new Set<string>()
  for (let i = 1; i < keys.length; i++) if (groupOf(keys[i]!) !== groupOf(keys[i - 1]!)) out.add(keys[i]!)
  return out
}

export function clampSlot(order: readonly string[], groupOf: (key: string) => string, key: string, slot: number): number {
  const g = groupOf(key)
  let first = -1
  let last = -1
  order.forEach((k, i) => {
    if (groupOf(k) !== g) return
    if (first < 0) first = i
    last = i
  })
  if (first < 0) return slot
  return Math.min(Math.max(slot, first), last + 1)
}

export function withinGroup(order: readonly string[], groupOf: (key: string) => string, key: string, slot: number): { list: string[]; slot: number } {
  const g = groupOf(key)
  const first = order.findIndex((k) => groupOf(k) === g)
  const list = order.filter((k) => groupOf(k) === g)
  if (first < 0) return { list, slot }
  return { list, slot: clampSlot(order, groupOf, key, slot) - first }
}

export interface FitInput {
  keys: readonly string[]
  widths: ReadonlyMap<string, number>
  active: string | null
  avail: number
  gap?: number
  fallback?: number
  moreWidth?: number
}

export interface Fit {
  shown: string[]
  hidden: string[]
}

export function fitTabs(input: FitInput): Fit {
  const { keys, widths, active, avail } = input
  const gap = input.gap ?? 4
  const fallback = input.fallback ?? TAB_WIDTH_FALLBACK
  const more = input.moreWidth ?? MORE_BUTTON_W
  const w = (k: string): number => (widths.get(k) ?? fallback) + gap
  if (avail <= 0) return { shown: [...keys], hidden: [] }
  const total = keys.reduce((n, k) => n + w(k), 0)
  if (total <= avail + gap) return { shown: [...keys], hidden: [] }
  const pinned = active && keys.includes(active) ? active : null
  let budget = avail - more - (pinned ? w(pinned) : 0)
  const shown = new Set<string>()
  if (pinned) shown.add(pinned)
  for (const k of keys) {
    if (k === pinned) continue
    if (w(k) > budget) break
    shown.add(k)
    budget -= w(k)
  }
  return { shown: keys.filter((k) => shown.has(k)), hidden: keys.filter((k) => !shown.has(k)) }
}
