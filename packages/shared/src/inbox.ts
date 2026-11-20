export const INBOX_REASONS = ['menu', 'permission', 'question', 'clash', 'failed', 'loop', 'checks', 'stuck', 'note', 'done'] as const
export type InboxReason = (typeof INBOX_REASONS)[number]

export const INBOX_GROUPS = ['menu', 'permission', 'question', 'clash', 'error', 'note', 'done'] as const
export type InboxGroup = (typeof INBOX_GROUPS)[number]

const GROUP_OF: Record<InboxReason, InboxGroup> = {
  menu: 'menu',
  permission: 'permission',
  question: 'question',
  clash: 'clash',
  failed: 'error',
  loop: 'error',
  checks: 'error',
  stuck: 'error',
  note: 'note',
  done: 'done',
}

export function inboxGroupOf(reason: InboxReason): InboxGroup {
  return GROUP_OF[reason]
}

export function isInboxReason(value: unknown): value is InboxReason {
  return typeof value === 'string' && (INBOX_REASONS as readonly string[]).includes(value)
}

export const isUrgentInboxGroup = (g: InboxGroup): boolean => g === 'menu' || g === 'permission' || g === 'question' || g === 'clash' || g === 'error'

export interface RemoteInbox {
  reason: InboxReason
  since?: string
  file?: string
  peer?: string
}

export const REMOTE_INBOX_TEXT_MAX = 120

export function parseRemoteInbox(value: unknown): RemoteInbox | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const v = value as Record<string, unknown>
  if (!isInboxReason(v.reason)) return null
  const text = (x: unknown): string | undefined => (typeof x === 'string' && x ? x.slice(0, REMOTE_INBOX_TEXT_MAX) : undefined)
  const since = text(v.since)
  const file = text(v.file)
  const peer = text(v.peer)
  return { reason: v.reason, ...(since ? { since } : {}), ...(file ? { file } : {}), ...(peer ? { peer } : {}) }
}

export function sortInbox<T extends { reason: InboxReason; since?: string }>(items: readonly T[]): T[] {
  const at = (i: T): number => (i.since ? Date.parse(i.since) || Infinity : Infinity)
  const rank = (i: T): number => INBOX_GROUPS.indexOf(inboxGroupOf(i.reason))
  return [...items].sort((a, b) => rank(a) - rank(b) || at(a) - at(b))
}
