import type { MonitorSnapshot } from '../remote/monitorReducer'
import type { ChatLastEntry } from './chatModel'
import type { Provider, Rail } from './monitorModel'
import { baseName, type WsPane, type WsTab } from './workspaceModel'

export interface LiveInfo {
  at: number | null
  contextPct: number | null
  model: string | null
  tool: string | null
  toolSummary: string | null
  toolAt: number | null
}

export interface ChatRow {
  key: string
  title: string
  provider: Provider
  project: string | null
  cwd: string | null
  rail: Rail
  last: string | null
  at: number | null
  tool: string | null
  waitText: string | null
  canChat: boolean
  canAttach: boolean
  pane: WsPane
}

type Obj = Record<string, unknown>

const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v)
const str = (v: unknown): string | null => (typeof v === 'string' && v !== '' ? v : null)
const time = (v: unknown): number | null => {
  if (typeof v === 'number' && Number.isFinite(v)) return v
  if (typeof v !== 'string') return null
  const ms = Date.parse(v)
  return Number.isFinite(ms) ? ms : null
}

export const EMPTY_LIVE: LiveInfo = { at: null, contextPct: null, model: null, tool: null, toolSummary: null, toolAt: null }

export function liveOf(snapshot: MonitorSnapshot | null, terminalId: string | null): LiveInfo {
  if (!snapshot || !terminalId) return EMPTY_LIVE
  const terminals = Array.isArray(snapshot.terminals) ? snapshot.terminals : []
  const agents = Array.isArray(snapshot.agents) ? snapshot.agents : []
  const term = terminals.find((x): x is Obj => isObj(x) && x.id === terminalId)
  const main = agents.find((x): x is Obj => isObj(x) && x.terminalId === terminalId && x.role === 'main')
  const pct = main && typeof main.contextPct === 'number' && Number.isFinite(main.contextPct) ? Math.max(0, Math.min(100, Math.round(main.contextPct))) : null
  return {
    at: time(term?.lastActivityAt),
    contextPct: pct,
    model: str(main?.model),
    tool: str(main?.currentTool),
    toolSummary: str(main?.currentToolSummary),
    toolAt: time(main?.currentToolStartedAt),
  }
}

export function projectOf(tab: WsTab, pane: WsPane): string | null {
  const group = tab.groups.find((g) => g.panes.some((p) => p.key === pane.key) && g.key !== '~rest')
  if (group) return group.name || baseName(group.project)
  if (tab.project) return baseName(tab.project)
  return pane.cwd ? baseName(pane.cwd) : null
}

export function canChatPane(pane: WsPane): boolean {
  return pane.isTerm && pane.provider !== 'shell'
}

export function chatRows(tabs: readonly WsTab[], snapshot: MonitorSnapshot | null, last: Readonly<Record<string, ChatLastEntry>>): ChatRow[] {
  const rows: ChatRow[] = []
  const seen = new Set<string>()
  for (const tab of tabs) {
    for (const pane of tab.panes) {
      if (seen.has(pane.key) || !pane.isTerm) continue
      seen.add(pane.key)
      const live = liveOf(snapshot, pane.terminalId)
      const said = last[pane.key]
      const at = Math.max(live.at ?? 0, said?.at ?? 0, pane.since ?? 0) || null
      rows.push({
        key: pane.key,
        title: pane.title,
        provider: pane.provider,
        project: projectOf(tab, pane),
        cwd: pane.cwd,
        rail: pane.rail,
        last: said?.text ?? null,
        at,
        tool: pane.tool ? (pane.toolSummary ? `${pane.tool} ${pane.toolSummary}` : pane.tool) : null,
        waitText: pane.waitText,
        canChat: canChatPane(pane),
        canAttach: pane.canAttach,
        pane,
      })
    }
  }
  return rows
}

export function sortRows(rows: readonly ChatRow[]): { waiting: ChatRow[]; rest: ChatRow[] } {
  const byTime = (a: ChatRow, b: ChatRow): number => (b.at ?? 0) - (a.at ?? 0) || a.title.localeCompare(b.title)
  const waiting = rows.filter((r) => r.rail === 'asked').sort(byTime)
  const rest = rows.filter((r) => r.rail !== 'asked').sort(byTime)
  return { waiting, rest }
}

export function filterRows(rows: readonly ChatRow[], query: string): ChatRow[] {
  const q = query.trim().toLocaleLowerCase()
  if (!q) return [...rows]
  return rows.filter((r) => `${r.title} ${r.project ?? ''} ${r.last ?? ''}`.toLocaleLowerCase().includes(q))
}

export function firstCwd(tabs: readonly WsTab[], rows: readonly ChatRow[]): string | null {
  const sorted = [...rows].sort((a, b) => (b.at ?? 0) - (a.at ?? 0))
  return sorted.find((r) => r.cwd)?.cwd ?? tabs.find((t) => t.cwd)?.cwd ?? null
}

export function shortAgo(ms: number, t: (key: string, values?: Record<string, unknown>) => string): string {
  if (ms < 60_000) return t('duration.lessThanMinute')
  if (ms < 3_600_000) return t('duration.minutes', { n: Math.floor(ms / 60_000) })
  if (ms < 86_400_000) return t('duration.hours', { n: Math.floor(ms / 3_600_000) })
  return t('duration.days', { n: Math.floor(ms / 86_400_000) })
}
