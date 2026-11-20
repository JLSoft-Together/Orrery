import type { ChatLog } from './chatModel'

const MAX = 8
const logs = new Map<string, ChatLog>()

const keyOf = (machine: string, pane: string): string => `${machine}\u0000${pane}`

export function cachedLog(machine: string, pane: string): ChatLog | null {
  return logs.get(keyOf(machine, pane)) ?? null
}

export function cacheLog(machine: string, pane: string, log: ChatLog): void {
  if (log.gen === null) return
  const key = keyOf(machine, pane)
  logs.delete(key)
  logs.set(key, log)
  while (logs.size > MAX) logs.delete(logs.keys().next().value as string)
}

export function forgetLogs(machine?: string): void {
  if (machine === undefined) {
    logs.clear()
    return
  }
  for (const key of [...logs.keys()]) if (key.startsWith(`${machine}\u0000`)) logs.delete(key)
}
