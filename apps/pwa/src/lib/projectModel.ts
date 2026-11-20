import { isRunning, isWaiting, type MonitorSession, type Rail } from './monitorModel'

export interface ProjectGroup {
  key: string
  name: string | null
  letter: string
  tone: number
  branch: string | null
  sessions: MonitorSession[]
}

export type RailCounts = Record<Rail, number>

export function projectName(cwd: string | null): string | null {
  if (!cwd) return null
  const parts = cwd.replace(/\\/g, '/').split('/').filter(Boolean)
  return parts.length ? parts[parts.length - 1]! : null
}

export function toneOf(text: string): number {
  let hash = 0
  for (let i = 0; i < text.length; i++) hash = (hash * 31 + text.charCodeAt(i)) >>> 0
  return hash % 8
}

export function letterOf(name: string | null): string {
  const first = name ? [...name.trim()][0] : undefined
  return first ? first.toUpperCase() : '?'
}

export function groupByProject(sessions: readonly MonitorSession[]): ProjectGroup[] {
  const order: string[] = []
  const byKey = new Map<string, ProjectGroup>()
  for (const session of sessions) {
    const name = projectName(session.cwd)
    const key = name ?? ''
    let group = byKey.get(key)
    if (!group) {
      group = { key, name, letter: letterOf(name), tone: toneOf(key), branch: null, sessions: [] }
      byKey.set(key, group)
      order.push(key)
    }
    group.sessions.push(session)
    if (!group.branch && session.branch) group.branch = session.branch
  }
  return order.map((key) => byKey.get(key)!)
}

export function railCounts(sessions: readonly Pick<MonitorSession, 'rail'>[]): RailCounts {
  const counts: RailCounts = { failed: 0, asked: 0, working: 0, done: 0, idle: 0 }
  for (const session of sessions) counts[session.rail]++
  return counts
}

export type CardState = 'live' | 'connecting' | 'lost'

export function cardState(status: string, hasData: boolean): CardState {
  if (status === 'ready') return 'live'
  if (status === 'discovering' || status === 'handshaking' || status === 'reconnecting') return hasData ? 'lost' : 'connecting'
  return 'lost'
}

export function osLabel(hello: Record<string, unknown> | null | undefined): string | null {
  const raw = hello?.os ?? hello?.platform
  if (typeof raw !== 'string') return null
  switch (raw.toLowerCase()) {
    case 'win32':
    case 'windows':
    case 'win':
      return 'Windows'
    case 'darwin':
    case 'macos':
    case 'mac':
      return 'macOS'
    case 'linux':
      return 'Linux'
    default:
      return null
  }
}

export function machineSubtitle(hello: Record<string, unknown> | null | undefined): string | null {
  const version = typeof hello?.version === 'string' && hello.version !== '' ? hello.version : null
  if (!version) return null
  const os = osLabel(hello)
  return os ? `Orrery ${version}, ${os}` : `Orrery ${version}`
}

export type QuotaTone = 'ok' | 'warn' | 'bad'

export function quotaTone(pct: number): QuotaTone {
  if (pct >= 85) return 'bad'
  if (pct >= 65) return 'warn'
  return 'ok'
}

export type ProjectRow = { kind: 'single'; project: ProjectGroup; session: MonitorSession } | { kind: 'group'; project: ProjectGroup }

function projectRank(group: ProjectGroup): number {
  let best = 2
  for (const s of group.sessions) best = Math.min(best, isWaiting(s.rail) ? 0 : isRunning(s.rail) ? 1 : 2)
  return best
}

export function projectRows(sessions: readonly MonitorSession[]): ProjectRow[] {
  return groupByProject(sessions)
    .map((project, index) => ({ project, index, rank: projectRank(project) }))
    .sort((a, b) => a.rank - b.rank || a.index - b.index)
    .map(({ project }) => (project.sessions.length === 1 ? { kind: 'single', project, session: project.sessions[0]! } : { kind: 'group', project }))
}
