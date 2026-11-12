import type { MonitorSnapshot } from './monitorReducer'

export interface StatusCounts {
  working: number
  waiting: number
  idle: number
}

export function statusCounts(snapshot: MonitorSnapshot | null): StatusCounts {
  const counts: StatusCounts = { working: 0, waiting: 0, idle: 0 }
  const terminals = snapshot?.terminals
  if (!Array.isArray(terminals)) return counts
  for (const terminal of terminals) {
    const status = terminal && typeof terminal === 'object' ? (terminal as Record<string, unknown>).status : undefined
    if (status === 'working') counts.working++
    else if (status === 'waiting') counts.waiting++
    else if (status === 'idle') counts.idle++
  }
  return counts
}
