import type { MenuEntry } from '../menu'

export interface RailSelection {
  keys: string[]
  anchor: string | null
}

export const NO_SELECTION: RailSelection = { keys: [], anchor: null }

export interface SelectMods {
  toggle: boolean
  range: boolean
}

function span(order: readonly string[], a: string, b: string): string[] {
  const i = order.indexOf(a)
  const j = order.indexOf(b)
  if (i < 0 || j < 0) return order.includes(b) ? [b] : []
  return order.slice(Math.min(i, j), Math.max(i, j) + 1)
}

const union = (a: readonly string[], b: readonly string[]): string[] => [...new Set([...a, ...b])]

export function clickSelect(sel: RailSelection, key: string, order: readonly string[], mods: SelectMods, current: string | null): RailSelection {
  if (!mods.toggle && !mods.range) return NO_SELECTION
  const seed = sel.keys.length ? sel.keys : current && current !== key && order.includes(current) ? [current] : []
  if (mods.range) {
    const anchor = sel.anchor ?? (current && order.includes(current) ? current : key)
    const range = span(order, anchor, key)
    return { keys: mods.toggle ? union(seed, range) : range, anchor }
  }
  const keys = seed.includes(key) ? seed.filter((k) => k !== key) : [...seed, key]
  return keys.length ? { keys, anchor: key } : NO_SELECTION
}

export function extendSelect(sel: RailSelection, from: string, to: string, order: readonly string[]): RailSelection {
  const anchor = sel.anchor ?? from
  return { keys: span(order, anchor, to), anchor }
}

export function pruneSelection(sel: RailSelection, exists: (key: string) => boolean): RailSelection {
  const keys = sel.keys.filter(exists)
  if (keys.length === sel.keys.length) return sel
  return keys.length ? { keys, anchor: sel.anchor && exists(sel.anchor) ? sel.anchor : (keys[0] ?? null) } : NO_SELECTION
}

export interface SelRow {
  paneKey: string
  tabKey: string
  tabRow: boolean
}

export function selectionPanes(keys: readonly string[], rowOf: (key: string) => SelRow | undefined, leavesOf: (tabKey: string) => readonly string[]): string[] {
  const out: string[] = []
  for (const key of keys) {
    const row = rowOf(key)
    if (!row) continue
    for (const k of row.tabRow ? leavesOf(row.tabKey) : [row.paneKey]) if (!out.includes(k)) out.push(k)
  }
  return out
}

export function selectionFacts(panes: readonly string[], tabOf: (paneKey: string) => { key: string; size: number } | undefined): { group: boolean; ungroup: boolean } {
  const tabs = new Set<string>()
  let ungroup = false
  for (const k of panes) {
    const tab = tabOf(k)
    if (!tab) continue
    tabs.add(tab.key)
    if (tab.size > 1) ungroup = true
  }
  return { group: panes.length > 1 && tabs.size > 1, ungroup }
}

type T = (key: string, params?: Record<string, unknown>, plural?: number) => string

export function selectionMenu(ctx: { count: number; group: boolean; ungroup: boolean }, t: T): MenuEntry[] {
  return [
    { id: 'sel-group', label: t('railSelect.group'), disabled: !ctx.group },
    { id: 'sel-ungroup', label: t('railSelect.ungroup'), disabled: !ctx.ungroup },
    { separator: true },
    { id: 'sel-close', label: t('railSelect.close', { n: ctx.count }, ctx.count) },
    { separator: true },
    { id: 'sel-clear', label: t('railSelect.clear'), shortcut: 'Esc' },
  ]
}
