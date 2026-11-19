import { ref } from 'vue'
import type { JsonMap } from '../remote/types'

const MAX_TRACKED = 8
const asked = new Map<string, string>()

export const undoOffer = ref<{ machine: string; seq: number; at: number }>({ machine: '', seq: 0, at: 0 })

function key(machine: string, cid: string): string {
  return `${machine}:${cid}`
}

export function rememberClose(machine: string, cid: string): void {
  asked.set(key(machine, cid), machine)
  while (asked.size > MAX_TRACKED) {
    const oldest = asked.keys().next().value
    if (oldest === undefined) break
    asked.delete(oldest)
  }
}

export function closeAnswered(machine: string, cid: string, ok: boolean, reply: JsonMap): boolean {
  const tracked = asked.delete(key(machine, cid))
  return tracked && ok && reply.cancelled !== true
}

export function offerUndo(machine: string, now = Date.now()): void {
  undoOffer.value = { machine, seq: undoOffer.value.seq + 1, at: now }
}
