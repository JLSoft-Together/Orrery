import { leaves, type LayoutNode } from './splitLayout'

export interface TabShape {
  key: string
  root: LayoutNode
  focus: string
  project?: string
  name?: string
  folded?: boolean
  zoom?: string
}

export interface LayoutSnapshot {
  tabs: TabShape[]
  projects: Record<string, string | null>
  active: string | null
  sig: string
}

export const LAYOUT_UNDO_MS = 5000

const shapeOf = (t: TabShape): TabShape => {
  const out: TabShape = { key: t.key, root: JSON.parse(JSON.stringify(t.root)) as LayoutNode, focus: t.focus }
  if (t.project !== undefined) out.project = t.project
  if (t.name !== undefined) out.name = t.name
  if (t.folded) out.folded = true
  if (t.zoom !== undefined) out.zoom = t.zoom
  return out
}

export function layoutSignature(tabs: readonly TabShape[]): string {
  return JSON.stringify(tabs.map(shapeOf))
}

export function snapshotLayout(tabs: readonly TabShape[], panes: ReadonlyArray<{ key: string; project?: string }>, active: string | null): LayoutSnapshot {
  const projects: Record<string, string | null> = {}
  for (const p of panes) projects[p.key] = p.project ?? null
  const shaped = tabs.map(shapeOf)
  return { tabs: shaped, projects, active, sig: JSON.stringify(shaped) }
}

export function snapshotPanes(s: Pick<LayoutSnapshot, 'tabs'>): string[] {
  return s.tabs.flatMap((t) => leaves(t.root)).sort()
}

export function samePaneSet(s: Pick<LayoutSnapshot, 'tabs'>, keys: readonly string[]): boolean {
  const a = snapshotPanes(s)
  const b = [...keys].sort()
  return a.length === b.length && a.every((k, i) => k === b[i])
}

export function cloneTabs(s: Pick<LayoutSnapshot, 'tabs'>): TabShape[] {
  return s.tabs.map(shapeOf)
}

export interface LayoutUndo {
  record(before: LayoutSnapshot, afterSig: string): boolean
  canUndo(currentSig: string, paneKeys: readonly string[]): boolean
  take(currentSig: string, paneKeys: readonly string[]): LayoutSnapshot | null
  clear(): void
  pending(): LayoutSnapshot | null
}

export function createLayoutUndo(): LayoutUndo {
  let entry: { before: LayoutSnapshot; after: string } | null = null
  const canUndo = (currentSig: string, paneKeys: readonly string[]): boolean => !!entry && entry.after === currentSig && samePaneSet(entry.before, paneKeys)
  return {
    record(before, afterSig) {
      if (before.sig === afterSig) return false
      entry = { before, after: afterSig }
      return true
    },
    canUndo,
    take(currentSig, paneKeys) {
      if (!canUndo(currentSig, paneKeys)) return null
      const before = entry!.before
      entry = null
      return before
    },
    clear() {
      entry = null
    },
    pending: () => entry?.before ?? null,
  }
}
