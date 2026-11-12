import type { MonitorSnapshot } from '../remote/monitorReducer'

export type Rail = 'failed' | 'asked' | 'working' | 'done' | 'idle'
export type MonitorFilter = 'all' | 'running' | 'waiting' | 'idle'
export type Provider = 'claude' | 'codex' | 'shell'

export interface MonitorAgent {
  id: string
  rail: Rail
  depth: number
  name: string | null
  tool: string | null
}

export interface MonitorSession {
  id: string
  title: string
  rail: Rail
  provider: Provider
  branch: string | null
  cwd: string | null
  tool: string | null
  toolSummary: string | null
  waitingFor: string | null
  since: number | null
  subagents: MonitorAgent[]
}

export interface ActivityItem {
  id: string
  kind: string
  at: number
  detail: string | null
}

type Item = Record<string, unknown>

const isItem = (value: unknown): value is Item => !!value && typeof value === 'object' && !Array.isArray(value)
const str = (value: unknown): string | null => (typeof value === 'string' && value !== '' ? value : null)
const items = (value: unknown): Item[] => (Array.isArray(value) ? value.filter(isItem) : [])

export function railOf(status: unknown): Rail {
  switch (status) {
    case 'working':
      return 'working'
    case 'waiting':
      return 'asked'
    case 'error':
    case 'failed':
      return 'failed'
    case 'completed':
    case 'done':
      return 'done'
    default:
      return 'idle'
  }
}

export function providerOf(value: unknown): Provider {
  return value === 'codex' || value === 'shell' ? value : 'claude'
}

export function isRunning(rail: Rail): boolean {
  return rail === 'working'
}

export function isWaiting(rail: Rail): boolean {
  return rail === 'asked' || rail === 'failed'
}

function rank(rail: Rail): number {
  return isWaiting(rail) ? 0 : isRunning(rail) ? 1 : 2
}

function parseSubagents(agents: Item[], terminalId: string): { main: Item | undefined; subagents: MonitorAgent[] } {
  const mine = agents.filter((a) => a.terminalId === terminalId)
  const main = mine.find((a) => a.role === 'main')
  const subs = mine.filter((a) => a.role === 'subagent')
  const known = new Set(subs.map((s) => str(s.id)))
  const mainId = str(main?.id)
  const out: MonitorAgent[] = []
  const seen = new Set<string>()
  const walk = (parent: string | null, depth: number): void => {
    for (const sub of subs) {
      const id = str(sub.id) ?? ''
      if (str(sub.parentId) !== parent || seen.has(id)) continue
      seen.add(id)
      out.push({ id, rail: railOf(sub.status), depth, name: str(sub.name) ?? str(sub.description) ?? str(sub.type), tool: str(sub.currentTool) })
      walk(id, depth + 1)
    }
  }
  walk(mainId, 1)
  for (const sub of subs) {
    const parent = str(sub.parentId)
    if (parent && parent !== mainId && !known.has(parent)) walk(parent, 1)
  }
  walk(null, 1)
  return { main, subagents: out }
}

export function parseSessions(snapshot: MonitorSnapshot | null): MonitorSession[] {
  if (!snapshot) return []
  const agents = items(snapshot.agents)
  const out: MonitorSession[] = []
  for (const terminal of items(snapshot.terminals)) {
    const id = str(terminal.id)
    if (!id) continue
    const { main, subagents } = parseSubagents(agents, id)
    const since = typeof terminal.statusSince === 'string' ? Date.parse(terminal.statusSince) : Number.NaN
    out.push({
      id,
      title: str(terminal.alias) ?? str(terminal.title) ?? id,
      rail: railOf(terminal.status),
      provider: providerOf(terminal.provider),
      branch: str(terminal.gitBranch),
      cwd: str(terminal.cwd),
      tool: str(main?.currentTool),
      toolSummary: str(main?.currentToolSummary),
      waitingFor: str(terminal.waitingFor),
      since: Number.isFinite(since) ? since : null,
      subagents,
    })
  }
  return out
}

export function matchesFilter(filter: MonitorFilter, session: MonitorSession): boolean {
  switch (filter) {
    case 'all':
      return true
    case 'running':
      return isRunning(session.rail)
    case 'waiting':
      return isWaiting(session.rail)
    case 'idle':
      return !isRunning(session.rail) && !isWaiting(session.rail)
  }
}

export function sortSessions(sessions: MonitorSession[]): MonitorSession[] {
  return sessions
    .map((session, index) => ({ session, index }))
    .sort((a, b) => rank(a.session.rail) - rank(b.session.rail) || a.index - b.index)
    .map((entry) => entry.session)
}

export function filterCounts(sessions: MonitorSession[]): Record<MonitorFilter, number> {
  return {
    all: sessions.length,
    running: sessions.filter((s) => matchesFilter('running', s)).length,
    waiting: sessions.filter((s) => matchesFilter('waiting', s)).length,
    idle: sessions.filter((s) => matchesFilter('idle', s)).length,
  }
}

const GROUP_ORDER: Rail[] = ['failed', 'asked', 'working', 'done', 'idle']

export function groupByState(sessions: MonitorSession[]): Array<{ rail: Rail; sessions: MonitorSession[] }> {
  return GROUP_ORDER.map((rail) => ({ rail, sessions: sessions.filter((s) => s.rail === rail) })).filter((g) => g.sessions.length > 0)
}

export type Translate = (key: string, named?: Record<string, unknown>) => string

export function formatElapsed(ms: number, t: Translate): string {
  if (ms < 60_000) return t('duration.lessThanMinute')
  const hours = Math.floor(ms / 3_600_000)
  if (hours >= 24) return t('duration.days', { n: Math.floor(hours / 24) })
  if (hours >= 1) return t('duration.hours', { n: hours })
  return t('duration.minutes', { n: Math.floor(ms / 60_000) })
}

export const ACTIVITY_SHOWN = 12

function detailOf(kind: string, data: Item): string | null {
  let parts: unknown[]
  if (kind === 'tool.started' || kind === 'tool.failed') parts = [data.tool, data.error]
  else if (kind.startsWith('agent.')) parts = [data.agentType, data.description, data.error]
  else if (kind === 'terminal.renamed') parts = [data.to ?? data.title]
  else if (kind === 'terminal.status') parts = [data.to, data.waitingFor]
  else parts = [data.title, data.description, data.error]
  const text = parts
    .filter((p): p is string => typeof p === 'string' && p !== '')
    .join(' · ')
    .replace(/\s+/g, ' ')
  if (!text) return null
  return text.length > 160 ? `${text.slice(0, 160)}…` : text
}

export function parseActivity(raw: unknown, terminalId: string): ActivityItem[] {
  const list = Array.isArray(raw) ? raw : []
  const out: ActivityItem[] = []
  for (let i = list.length - 1; i >= 0 && out.length < ACTIVITY_SHOWN; i--) {
    const entry = list[i]
    if (!isItem(entry) || entry.terminalId !== terminalId) continue
    const kind = str(entry.kind)
    const at = typeof entry.at === 'string' ? Date.parse(entry.at) : Number.NaN
    if (!kind || !Number.isFinite(at)) continue
    out.push({ id: str(entry.id) ?? String(i), kind, at, detail: isItem(entry.data) ? detailOf(kind, entry.data) : null })
  }
  return out
}

export function isBadActivity(kind: string): boolean {
  return kind.endsWith('.failed') || kind === 'sensitive' || kind === 'launch.failed'
}

const ACTIVITY_KEYS: Record<string, string> = {
  'terminal.started': 'started',
  'terminal.ended': 'ended',
  'terminal.renamed': 'renamed',
  'terminal.status': 'status',
  'session.cleared': 'cleared',
  'terminal.compacted': 'compacted',
  'agent.started': 'agentStarted',
  'agent.completed': 'agentDone',
  'agent.failed': 'agentFailed',
  'agent.cancelled': 'agentCancelled',
  'tool.started': 'tool',
  'tool.failed': 'toolFailed',
  'turn.started': 'turnStarted',
  'turn.completed': 'turnDone',
  'turn.failed': 'turnFailed',
  'turn.interrupted': 'turnStopped',
  sensitive: 'sensitive',
}

export function activityKey(kind: string): string | null {
  return ACTIVITY_KEYS[kind] ?? null
}
