import type { LiveRow } from './railLive'

export const TOOLS_OPEN_KEY = 'ccm.next.railToolsOpen'

export type FootMode = 'beside' | 'under' | 'icon'

export const FOOT_BESIDE_FROM = 300
export const FOOT_UNDER_FROM = 190

export function footModeFor(width: number, iconRail: boolean): FootMode {
  if (iconRail || !Number.isFinite(width) || width <= 0) return iconRail ? 'icon' : 'under'
  if (width >= FOOT_BESIDE_FROM) return 'beside'
  if (width >= FOOT_UNDER_FROM) return 'under'
  return 'icon'
}

export const TOOL_PANE_KINDS: readonly string[] = ['browser', 'file', 'changes', 'log']

export function isToolRow(row: Pick<LiveRow, 'src'>): boolean {
  const src = row.src
  return src?.kind === 'other' && !!src.pane?.kind && TOOL_PANE_KINDS.includes(src.pane.kind)
}

export type ToolDetail = { kind: 'dirty' } | { kind: 'files'; n: number } | null

export function toolDetail(row: Pick<LiveRow, 'src'>): ToolDetail {
  const pane = row.src?.pane
  if (!pane) return null
  if (pane.kind === 'file' && pane.dirty) return { kind: 'dirty' }
  if (pane.kind === 'changes' && typeof pane.count === 'number' && pane.count > 0) return { kind: 'files', n: pane.count }
  return null
}

export interface ToolSplit {
  main: string[]
  tools: string[]
  dirty: number
}

export function splitTools(keys: readonly string[], rowOf: (key: string) => LiveRow | undefined): ToolSplit {
  const out: ToolSplit = { main: [], tools: [], dirty: 0 }
  for (const k of keys) {
    const r = rowOf(k)
    if (r && isToolRow(r)) {
      out.tools.push(k)
      if (toolDetail(r)?.kind === 'dirty') out.dirty++
    } else out.main.push(k)
  }
  return out
}

export function parseToolsOpen(raw: string | null | undefined): Set<string> {
  if (!raw) return new Set()
  try {
    const v: unknown = JSON.parse(raw)
    return new Set(Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [])
  } catch {
    return new Set()
  }
}

export function readToolsOpen(): Set<string> {
  try {
    return parseToolsOpen(localStorage.getItem(TOOLS_OPEN_KEY))
  } catch {
    return new Set()
  }
}

export function writeToolsOpen(open: ReadonlySet<string>): void {
  try {
    localStorage.setItem(TOOLS_OPEN_KEY, JSON.stringify([...open]))
  } catch {
    return
  }
}

export function toggleOpen(open: ReadonlySet<string>, key: string): Set<string> {
  const next = new Set(open)
  if (!next.delete(key)) next.add(key)
  return next
}
