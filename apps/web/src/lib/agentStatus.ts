import { cleanAgentStatus, isStatusStale, type WorkflowStatus, type WorkflowStatusEvent } from '@ccm/shared'
import { desktop } from './desktop'

interface StatusBridge {
  agentStatusWatch?: (key: string, dir: string | null) => Promise<boolean>
  onAgentStatus?: (handler: (event: unknown) => void) => () => void
}
const bridge = desktop as unknown as StatusBridge | undefined

export const hasAgentStatus = typeof bridge?.agentStatusWatch === 'function'

export const watchAgentStatus = (key: string, dir: string | null): Promise<boolean> => (bridge?.agentStatusWatch?.(key, dir) ?? Promise.resolve(false)).catch(() => false)

/** Main already sanitized; this keeps a malformed push from reaching the UI. */
export function parseStatusEvent(raw: unknown): WorkflowStatusEvent | null {
  if (!raw || typeof raw !== 'object') return null
  const o = raw as { key?: unknown; status?: unknown }
  if (typeof o.key !== 'string' || !o.key) return null
  if (o.status === null || o.status === undefined) return { key: o.key, status: null }
  const status = cleanAgentStatus(o.status)
  return status ? { key: o.key, status } : null
}

export const onAgentStatus = (handler: (event: WorkflowStatusEvent) => void): (() => void) =>
  bridge?.onAgentStatus?.((raw) => {
    const e = parseStatusEvent(raw)
    if (e) handler(e)
  }) ?? (() => undefined)

export const progressPct = (s: Pick<WorkflowStatus, 'progress'>): number | null => (s.progress === undefined ? null : Math.round(s.progress * 100))

export { isStatusStale }
