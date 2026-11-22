import { inboxGroupOf, type InboxReason } from '@ccm/shared'

const NEEDS_ANSWER = new Set(['menu', 'permission', 'question'])

export function needsAnswer(reason: InboxReason): boolean {
  return NEEDS_ANSWER.has(inboxGroupOf(reason))
}

export function needIds(items: readonly { id: string; reason: InboxReason }[]): string[] {
  return items.filter((i) => needsAnswer(i.reason)).map((i) => i.id)
}

export function badgeCount(items: readonly { id: string; reason: InboxReason }[]): number {
  return needIds(items).length
}

export function hasFresh(prev: ReadonlySet<string>, ids: readonly string[]): boolean {
  return ids.some((id) => !prev.has(id))
}

export interface FlashInput {
  enabled: boolean
  fresh: boolean
  focused: boolean
  quiet: boolean
}

export function shouldFlash(i: FlashInput): boolean {
  return i.enabled && i.fresh && !i.focused && !i.quiet
}
