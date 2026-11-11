import type { ExtProjectErrorCode, ProjectMcpRow, ProjectPluginRow } from '@ccm/shared'

export const FILTER_MIN_ROWS = 12

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
