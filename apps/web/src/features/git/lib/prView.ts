import type { GhCiDoneEvent, GhCiRun, GhCiState, GhFail, GhMergeMethod, GhPr } from '@ccm/shared'

export type CiIcon = 'spin' | 'check' | 'x' | 'minus'
export type CiTone = 'wait' | 'ok' | 'bad' | 'idle'

export interface CiLook {
  icon: CiIcon
  tone: CiTone
  text: string
  cls: string
}

const LOOKS: Record<GhCiState, CiLook> = {
  pending: { icon: 'spin', tone: 'wait', text: 'v111pr.ciState.pending', cls: 'text-warning' },
  success: { icon: 'check', tone: 'ok', text: 'v111pr.ciState.success', cls: 'text-success' },
  failure: { icon: 'x', tone: 'bad', text: 'v111pr.ciState.failure', cls: 'text-st-error' },
  neutral: { icon: 'minus', tone: 'idle', text: 'v111pr.ciState.neutral', cls: 'text-ink-muted' },
  none: { icon: 'minus', tone: 'idle', text: 'v111pr.ciState.none', cls: 'text-ink-faint' },
}

export const ciLook = (state: GhCiState): CiLook => LOOKS[state] ?? LOOKS.none

export const runLook = (state: GhCiRun['state']): CiLook & { runText: string } => ({ ...ciLook(state), runText: `v111pr.runState.${state}` })

export function elapsedMs(run: Pick<GhCiRun, 'startedAt' | 'completedAt'>, now: number): number | null {
  if (!run.startedAt) return null
  const from = Date.parse(run.startedAt)
  if (!Number.isFinite(from)) return null
  const to = run.completedAt ? Date.parse(run.completedAt) : now
  if (!Number.isFinite(to)) return null
  return Math.max(0, to - from)
}

export function elapsedParts(ms: number): { key: 's' | 'm' | 'h'; n: number; m: number; s: number; h: number } {
  const total = Math.floor(ms / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return { key: h > 0 ? 'h' : m > 0 ? 'm' : 's', n: total, h, m, s }
}

export interface MergeOutcome {
  merged: boolean
  branchDeleted: boolean
  deleteError?: { error?: string; detail?: string }
}

export function permissionLabel(raw: string | null): string {
  if (!raw) return ''
  return raw
    .split(',')
    .map((part) => part.trim().replace(/^([\w-]+)\s*=\s*(\w+)$/, '$1 ($2)'))
    .filter(Boolean)
    .join(', ')
}

export type NeedsPush = 'branch' | 'ahead' | null

export type PanelState =
  | { kind: 'hidden' }
  | { kind: 'scope'; permission: string | null }
  | { kind: 'noPr'; canCreate: boolean; needsPush: NeedsPush }
  | { kind: 'pr'; pr: GhPr }

export interface PanelInput {
  fail: GhFail | null
  pr: GhPr | null | undefined
  branch: string | null
  base: string | null
  upstream: string | null
  ahead: number
}

export function panelState(i: PanelInput): PanelState {
  if (i.fail) return i.fail.error === 'gh.scope' ? { kind: 'scope', permission: i.fail.permission ?? null } : { kind: 'hidden' }
  if (!i.branch) return { kind: 'hidden' }
  if (i.pr) return { kind: 'pr', pr: i.pr }
  if (i.pr === undefined) return { kind: 'hidden' }
  if (i.base && i.branch === i.base) return { kind: 'hidden' }
  return { kind: 'noPr', canCreate: true, needsPush: !i.upstream ? 'branch' : i.ahead > 0 ? 'ahead' : null }
}

export function canMerge(pr: GhPr): boolean {
  return pr.state === 'open' && !pr.draft && pr.mergeable !== false
}

export const MERGE_METHODS: readonly GhMergeMethod[] = ['merge', 'squash', 'rebase']

export const isMergeMethod = (v: unknown): v is GhMergeMethod => typeof v === 'string' && (MERGE_METHODS as readonly string[]).includes(v)

const METHOD_KEY = 'ccm.gh.mergeMethod'

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage
  } catch {
    return null
  }
}

function readMap(storage: Pick<Storage, 'getItem'> | null): Record<string, unknown> {
  try {
    const raw = storage?.getItem(METHOD_KEY)
    const v = raw ? (JSON.parse(raw) as unknown) : {}
    return v && typeof v === 'object' ? (v as Record<string, unknown>) : {}
  } catch {
    return {}
  }
}

export function readMethod(root: string, storage: Pick<Storage, 'getItem'> | null = safeStorage()): GhMergeMethod {
  const m = readMap(storage)[root]
  return isMergeMethod(m) ? m : 'merge'
}

export function writeMethod(root: string, method: GhMergeMethod, storage: Pick<Storage, 'getItem' | 'setItem'> | null = safeStorage()): void {
  try {
    const next: Record<string, unknown> = { ...readMap(storage), [root]: method }
    const keys = Object.keys(next)
    for (const k of keys.slice(0, Math.max(0, keys.length - 100))) delete next[k]
    storage?.setItem(METHOD_KEY, JSON.stringify(next))
  } catch {
    return
  }
}

const KNOWN_ERRORS = new Set([
  'denied',
  'invalid',
  'cancelled',
  'network',
  'timeout',
  'no-remote',
  'no-keychain',
  'gh.notGithub',
  'gh.noAccount',
  'gh.noAccess',
  'gh.auth',
  'gh.scope',
  'gh.rateLimit',
  'gh.notFound',
  'gh.notMergeable',
  'gh.conflict',
  'gh.invalid',
  'gh.watchMax',
  'gh.http',
  'gh.logGone',
  'gh.forkHead',
])

export const errorKey = (code: string | undefined): string => (code && KNOWN_ERRORS.has(code) ? `v111pr.error.${code}` : 'v111pr.error.unknown')

export const CHIP_HOLD_MS = 10 * 60_000

export interface CiEntry {
  state: GhCiState
  at: number
  branch: string
}

export function chipState(entry: CiEntry | undefined, now: number): 'pending' | 'success' | 'failure' | null {
  if (!entry) return null
  if (entry.state === 'pending' || entry.state === 'failure') return entry.state
  if (entry.state === 'success' && now - entry.at < CHIP_HOLD_MS) return 'success'
  return null
}

export function toastParts(ev: Pick<GhCiDoneEvent, 'state' | 'failed' | 'branch' | 'timedOut'>): { title: string; body: string; kind: 'ok' | 'bad' | 'idle' } | null {
  if (ev.timedOut) return { title: 'v111pr.toast.timeoutTitle', body: 'v111pr.toast.body', kind: 'idle' }
  if (ev.state === 'success') return { title: 'v111pr.toast.successTitle', body: 'v111pr.toast.body', kind: 'ok' }
  if (ev.state === 'failure') return { title: 'v111pr.toast.failureTitle', body: ev.failed.length ? 'v111pr.toast.bodyFailed' : 'v111pr.toast.body', kind: 'bad' }
  if (ev.state === 'neutral') return { title: 'v111pr.toast.neutralTitle', body: 'v111pr.toast.body', kind: 'idle' }
  return null
}
