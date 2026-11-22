import type { ExtProjectChange, ExtProjectErrorCode, ExtSessionUsage, ProjectMcpRow, ProjectPluginRow } from '@ccm/shared'

export const FILTER_MIN_ROWS = 8

const SCOPE_ORDER: Record<ProjectMcpRow['scope'], number> = { project: 0, local: 1, user: 2, plugin: 3, connector: 4 }

export function sortMcp(rows: readonly ProjectMcpRow[]): ProjectMcpRow[] {
  return [...rows].sort((a, b) => Number(!!b.locked) - Number(!!a.locked) || SCOPE_ORDER[a.scope] - SCOPE_ORDER[b.scope] || a.name.localeCompare(b.name))
}

export function sortPlugins(rows: readonly ProjectPluginRow[]): ProjectPluginRow[] {
  return [...rows].sort((a, b) => a.name.localeCompare(b.name))
}

export function filterRows<T extends { name: string }>(rows: readonly T[], query: string): T[] {
  const q = query.trim().toLowerCase()
  return q ? rows.filter((r) => r.name.toLowerCase().includes(q)) : [...rows]
}

export const showFilter = (mcp: number, plugins: number): boolean => mcp + plugins > FILTER_MIN_ROWS

export function withEnabled<T extends { name: string; enabled: boolean }>(rows: readonly T[], name: string, enabled: boolean): T[] {
  return rows.map((r) => (r.name === name ? { ...r, enabled } : r))
}

export const errorKey = (code: ExtProjectErrorCode): string => `sessionTools.error.${code}`

export function canRestartWithResume(pane: { state: string } | undefined, status: string | undefined): boolean {
  return !!pane && pane.state === 'running' && status === 'idle'
}

export function toolsEntryVisible(s: { hasApi: boolean; hasCwd: boolean; claude: boolean; ended: boolean }): boolean {
  return s.hasApi && s.hasCwd && s.claude && !s.ended
}

export type ToolKind = ExtProjectChange['kind']
export type Draft = Readonly<Record<string, boolean>>

export const draftKey = (kind: ToolKind, name: string): string => `${kind}:${name}`

export const shownEnabled = (draft: Draft, kind: ToolKind, row: { name: string; enabled: boolean }): boolean => draft[draftKey(kind, row.name)] ?? row.enabled

export function setDraft(draft: Draft, kind: ToolKind, row: { name: string; enabled: boolean }, value: boolean): Draft {
  const key = draftKey(kind, row.name)
  const next = { ...draft }
  if (value === row.enabled) delete next[key]
  else next[key] = value
  return next
}

export function pendingChanges(draft: Draft, mcp: readonly ProjectMcpRow[], plugins: readonly ProjectPluginRow[]): ExtProjectChange[] {
  const out: ExtProjectChange[] = []
  for (const row of mcp) {
    const v = draft[draftKey('mcp', row.name)]
    if (v !== undefined && v !== row.enabled && !row.locked) out.push({ kind: 'mcp', name: row.name, enabled: v })
  }
  for (const row of plugins) {
    const v = draft[draftKey('plugin', row.name)]
    if (v !== undefined && v !== row.enabled) out.push({ kind: 'plugin', name: row.name, enabled: v })
  }
  return out
}

export function draftOf(changes: readonly ExtProjectChange[]): Draft {
  return Object.fromEntries(changes.map((c) => [draftKey(c.kind, c.name), c.enabled]))
}

export function withChanges<T extends { name: string; enabled: boolean }>(rows: readonly T[], kind: ToolKind, changes: readonly ExtProjectChange[]): T[] {
  const hit = new Map(changes.filter((c) => c.kind === kind).map((c) => [c.name, c.enabled]))
  return rows.map((r) => (hit.has(r.name) ? { ...r, enabled: hit.get(r.name)! } : r))
}

export function groupTarget(draft: Draft, kind: ToolKind, rows: readonly { name: string; enabled: boolean; locked?: boolean }[]): boolean | null {
  const free = rows.filter((r) => !r.locked)
  if (!free.length) return null
  return free.some((r) => !shownEnabled(draft, kind, r))
}

export function setGroup(draft: Draft, kind: ToolKind, rows: readonly { name: string; enabled: boolean; locked?: boolean }[], value: boolean): Draft {
  let next = draft
  for (const r of rows) if (!r.locked) next = setDraft(next, kind, r, value)
  return next
}

export interface ToolsSnapshot {
  mcp: ProjectMcpRow[]
  plugins: ProjectPluginRow[]
}

export const projectCache = new Map<string, ToolsSnapshot>()
export const restoreDrafts = new Map<string, Draft>()

export type UsageData = Extract<ExtSessionUsage, { ok: true }>

export const toolKey = (name: string): string => name.replace(/[^A-Za-z0-9_-]/g, '_')

export const mcpCalls = (usage: UsageData, name: string): number => usage.servers[toolKey(name)] ?? 0

export function pluginCalls(usage: UsageData, name: string): number {
  const base = name.split('@')[0] ?? name
  const prefix = `plugin_${toolKey(base)}_`
  let n = usage.namespaces[base] ?? 0
  for (const [k, v] of Object.entries(usage.servers)) if (k.startsWith(prefix)) n += v
  return n
}

export const isUnused = (usage: UsageData | null, row: ProjectMcpRow): boolean => !!usage && usage.total > 0 && !row.locked && row.enabled && mcpCalls(usage, row.name) === 0

export function unusedDraft(draft: Draft, usage: UsageData | null, rows: readonly ProjectMcpRow[]): Draft {
  let next = draft
  for (const r of rows) if (isUnused(usage, r)) next = setDraft(next, 'mcp', r, false)
  return next
}

export const usageCache = new Map<string, UsageData>()
