import type { WorkspaceDoc } from '../remote/machineData'
import { providerOf, type MonitorSession, type Provider, type Rail } from './monitorModel'

export interface WsPane {
  key: string
  title: string
  provider: Provider
  rail: Rail
  tool: string | null
  toolSummary: string | null
  waitText: string | null
  subagents: number
  cwd: string | null
  branch: string | null
  sessionId: string | null
  terminalId: string | null
  isTerm: boolean
  canAttach: boolean
  asked: boolean
  since: number | null
}

export interface WsTab {
  key: string
  name: string
  project: string | null
  cwd: string | null
  panes: WsPane[]
}

type Item = Record<string, unknown>

const isItem = (value: unknown): value is Item => !!value && typeof value === 'object' && !Array.isArray(value)
const str = (value: unknown): string | null => (typeof value === 'string' && value !== '' ? value : null)

export function paneRail(value: unknown): Rail {
  switch (value) {
    case 'failed':
      return 'failed'
    case 'asked':
      return 'asked'
    case 'working':
    case 'subagents':
      return 'working'
    case 'done':
      return 'done'
    default:
      return 'idle'
  }
}

export function baseName(path: string): string {
  return path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? path
}

function parsePane(raw: Item, sessions: Map<string, MonitorSession>): WsPane | null {
  const key = str(raw.key)
  if (!key) return null
  const terminalId = str(raw.terminalId)
  const sessionId = str(raw.sessionId)
  const live = terminalId ? sessions.get(terminalId) : undefined
  const preset = str(raw.preset) ?? str(raw.agent)
  const rail = paneRail(raw.rail)
  const isTerm = (str(raw.kind) ?? 'term') === 'term'
  return {
    key,
    title: str(raw.title) ?? key,
    provider: providerOf(preset ?? live?.provider),
    rail,
    tool: live?.tool ?? null,
    toolSummary: live?.toolSummary ?? null,
    waitText: str(raw.waitText) ?? live?.waitingFor ?? null,
    subagents: typeof raw.subagents === 'number' ? raw.subagents : 0,
    cwd: str(raw.cwd),
    branch: str(raw.branch),
    sessionId,
    terminalId,
    isTerm,
    canAttach: isTerm && sessionId !== null,
    asked: rail === 'asked',
    since: live?.since ?? null,
  }
}

export function parseWorkspace(doc: WorkspaceDoc | null, sessions: MonitorSession[]): WsTab[] {
  if (!doc || !Array.isArray(doc.tabs)) return []
  const byTerminal = new Map(sessions.map((s) => [s.id, s]))
  const tabs: WsTab[] = []
  for (const raw of doc.tabs) {
    if (!isItem(raw)) continue
    const key = str(raw.key)
    if (!key || !Array.isArray(raw.panes)) continue
    const panes = raw.panes.filter(isItem).flatMap((p) => parsePane(p, byTerminal) ?? [])
    const project = str(raw.project)
    const cwd = panes.find((p) => p.cwd)?.cwd ?? null
    tabs.push({ key, name: str(raw.name) ?? (project ? baseName(project) : (cwd ? baseName(cwd) : key)), project, cwd: project ?? cwd, panes })
  }
  return tabs
}

export function findPane(tabs: WsTab[], key: string): { tab: WsTab; pane: WsPane } | null {
  for (const tab of tabs) {
    const pane = tab.panes.find((p) => p.key === key)
    if (pane) return { tab, pane }
  }
  return null
}

export function previewLines(pane: WsPane): string[] {
  const lines: string[] = []
  if (pane.tool) lines.push(`> ${pane.toolSummary ? `${pane.tool} ${pane.toolSummary}` : pane.tool}`)
  if (pane.waitText) lines.push(pane.waitText)
  if (pane.subagents > 0) lines.push(`+${pane.subagents}`)
  return lines.slice(0, 3)
}

export function paneColorVar(provider: Provider): string {
  return `var(--ccm-agent-${provider})`
}
