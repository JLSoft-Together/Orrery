import { randomBytes } from 'node:crypto'
import type { TelegramButton } from '@ccm/shared'
import type { Markup } from './actions'

export const REPLY_TTL_MS = 24 * 3600_000
export const REPLY_GROUPS_MAX = 200
/** Telegram caps callback_data at 64 bytes, so only a token travels; the values stay in this process. */
export const REPLY_DATA_RE = /^rb:([A-Za-z0-9_-]{12})([0-5])$/

export interface ReplyGroup {
  /** PTY session id of the pane that sent the message. */
  pane: string
  project: string
  chatId: string
  threadId: number | null
  buttons: TelegramButton[]
  at: number
  /** Label of the button that answered the message. */
  done?: string
  /** A tap is being typed; another tap waits for its outcome. */
  busy?: boolean
}

export type ReplyClaim =
  | { kind: 'expired' }
  | { kind: 'answered'; label: string }
  | { kind: 'busy' }
  | { kind: 'ok'; token: string; index: number; group: ReplyGroup; button: TelegramButton }

/** Buttons of `telegram_send` messages, one group per message; the first valid tap answers it. */
export class ReplyButtons {
  private readonly groups = new Map<string, ReplyGroup>()

  constructor(
    private readonly now: () => number,
    private readonly ttlMs = REPLY_TTL_MS,
    private readonly max = REPLY_GROUPS_MAX,
  ) {}

  add(group: Omit<ReplyGroup, 'at' | 'done' | 'busy'>): string {
    this.prune()
    let token = randomBytes(9).toString('base64url')
    while (this.groups.has(token)) token = randomBytes(9).toString('base64url')
    this.groups.set(token, { ...group, buttons: group.buttons.map((b) => ({ ...b })), at: this.now() })
    while (this.groups.size > this.max) this.groups.delete(this.groups.keys().next().value as string)
    return token
  }

  markup(token: string, labels: string[]): Markup {
    return { inline_keyboard: labels.map((text, i) => [{ text, callback_data: `rb:${token}${i}` }]) }
  }

  forget(token: string): void {
    this.groups.delete(token)
  }

  claim(data: string): ReplyClaim {
    const m = REPLY_DATA_RE.exec(data)
    const group = m ? this.groups.get(m[1]!) : undefined
    if (!m || !group || this.now() - group.at > this.ttlMs) {
      if (m) this.groups.delete(m[1]!)
      return { kind: 'expired' }
    }
    const index = Number(m[2])
    const button = group.buttons[index]
    if (!button) return { kind: 'expired' }
    if (group.done !== undefined) return { kind: 'answered', label: group.done }
    if (group.busy) return { kind: 'busy' }
    group.busy = true
    return { kind: 'ok', token: m[1]!, index, group, button }
  }

  finish(token: string, label: string): void {
    const group = this.groups.get(token)
    if (!group) return
    group.done = label
    group.busy = false
  }

  release(token: string): void {
    const group = this.groups.get(token)
    if (group) group.busy = false
  }

  /** Some message still waits for a tap: the bot keeps reading updates for it even with receive mode off. */
  pending(): boolean {
    this.prune()
    for (const g of this.groups.values()) if (g.done === undefined) return true
    return false
  }

  private prune(): void {
    const now = this.now()
    for (const [k, g] of this.groups) if (now - g.at > this.ttlMs) this.groups.delete(k)
  }
}
