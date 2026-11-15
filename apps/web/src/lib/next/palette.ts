import type { TerminalStatus } from '@ccm/shared'

export type SessionBucket = 'needs' | 'running' | 'doneUnseen' | 'recent'
export type NextPrefixMode = 'commands' | 'sessions' | 'files' | 'projects'
export type NextScope = 'all' | 'sessions' | 'files' | 'commands' | 'projects'

export const NEXT_SCOPES: readonly NextScope[] = ['all', 'sessions', 'files', 'commands', 'projects']
export const NEXT_PREFIX: Record<string, NextPrefixMode> = { '>': 'commands', '@': 'sessions', '#': 'files', ':': 'projects' }
export const NEXT_MODES: readonly NextPrefixMode[] = ['sessions', 'files', 'commands', 'projects']
export const BUCKET_ORDER: readonly SessionBucket[] = ['needs', 'running', 'doneUnseen', 'recent']
export const NEXT_ROW_HEIGHT = 52

export const nextPrefixOf = (mode: NextPrefixMode): string => Object.keys(NEXT_PREFIX).find((k) => NEXT_PREFIX[k] === mode)!

export function parseNextPrefix(query: string): { mode: NextPrefixMode | null; text: string } {
  const q = query.trimStart()
  const mode = NEXT_PREFIX[q[0] ?? '']
  return mode ? { mode, text: q.slice(1).trim() } : { mode: null, text: query.trim() }
}

export const scopeOfMode = (mode: NextPrefixMode): NextScope => mode

export interface BucketInput {
  status: TerminalStatus
  attention?: 'waiting' | 'done'
}

export function bucketOf(s: BucketInput): SessionBucket {
  if (s.status === 'waiting' || s.status === 'error' || s.attention === 'waiting') return 'needs'
  if (s.status === 'working') return 'running'
  if (s.attention === 'done') return 'doneUnseen'
  return 'recent'
}

export interface Rankable {
  bucket: SessionBucket
  since?: string
}

export function compareSessions(a: Rankable, b: Rankable): number {
  const byBucket = BUCKET_ORDER.indexOf(a.bucket) - BUCKET_ORDER.indexOf(b.bucket)
  if (byBucket) return byBucket
  const at = Date.parse(a.since ?? '') || 0
  const bt = Date.parse(b.since ?? '') || 0
  return a.bucket === 'needs' ? at - bt : bt - at
}

export function repoName(cwd: string | undefined): string | undefined {
  if (!cwd) return undefined
  const parts = cwd.split(/[\\/]+/).filter(Boolean)
  return parts[parts.length - 1]
}

export function sessionTerms(s: { model?: string | null; branch?: string; cwd?: string }): string[] {
  return [s.model, s.branch, repoName(s.cwd)].filter((x): x is string => !!x)
}

export type NextGroup = SessionBucket | 'sessions' | 'projects' | 'favorites' | 'files' | 'nav' | 'actions' | 'prefs' | 'settings' | 'help'

export const NEXT_GROUP_ORDER: readonly NextGroup[] = [
  'needs',
  'running',
  'doneUnseen',
  'recent',
  'sessions',
  'nav',
  'actions',
  'prefs',
  'help',
  'projects',
  'favorites',
  'files',
  'settings',
]

const COMMAND_GROUPS = new Set<NextGroup>(['nav', 'actions', 'prefs', 'help', 'settings'])
const SESSION_GROUPS = new Set<NextGroup>(['needs', 'running', 'doneUnseen', 'recent', 'sessions'])

export function inNextScope(group: NextGroup, scope: NextScope): boolean {
  if (scope === 'all') return true
  if (scope === 'sessions') return SESSION_GROUPS.has(group)
  if (scope === 'projects') return group === 'projects' || group === 'favorites'
  if (scope === 'files') return group === 'files'
  return COMMAND_GROUPS.has(group)
}

export function groupOfSession(bucket: SessionBucket, hasText: boolean): NextGroup {
  return hasText ? 'sessions' : bucket
}

export function cycleNextScope(current: NextScope, delta: number): NextScope {
  return NEXT_SCOPES[(NEXT_SCOPES.indexOf(current) + delta + NEXT_SCOPES.length) % NEXT_SCOPES.length]!
}
