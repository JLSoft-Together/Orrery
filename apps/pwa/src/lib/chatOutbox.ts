import type { ChatItem } from './chatModel'

export type OutState = 'sending' | 'sent' | 'unsent'

export interface OutMsg {
  id: string
  text: string
  state: OutState
  at: number
}

export const OUT_KEEP_MS = 60_000
export const OUT_MATCH_MS = 15_000

const norm = (text: string): string => text.replace(/\s+/g, ' ').trim()

export function settleOutbox(outbox: readonly OutMsg[], items: readonly ChatItem[], now: number): OutMsg[] {
  const mine = items.filter((x): x is Extract<ChatItem, { k: 'me' }> => x.k === 'me')
  const used = new Set<string>()
  return outbox.filter((msg) => {
    if (msg.state !== 'sent') return true
    if (now - msg.at > OUT_KEEP_MS) return false
    const want = norm(msg.text)
    const hit = mine.find((x) => !used.has(x.id) && (x.at === undefined || x.at >= msg.at - OUT_MATCH_MS) && (norm(x.text) === want || norm(x.text).startsWith(want.slice(0, 200))))
    if (!hit) return true
    used.add(hit.id)
    return false
  })
}

export function setState(outbox: readonly OutMsg[], id: string, state: OutState, at?: number): OutMsg[] {
  return outbox.map((m) => (m.id === id ? { ...m, state, ...(at !== undefined ? { at } : {}) } : m))
}
