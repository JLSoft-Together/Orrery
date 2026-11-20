import type { WorkspaceDoc } from '../remote/machineData'
import {
  INBOX_GROUPS,
  inboxGroupOf,
  isInboxReason,
  sortInbox,
  type RemoteInbox,
  type InboxGroup,
  type InboxReason,
} from '../../../../packages/shared/src/inbox'
import type { Provider } from './monitorModel'
import { parseWorkspace } from './workspaceModel'

export { INBOX_GROUPS, inboxGroupOf, isInboxReason }
export type { InboxGroup, InboxReason }

export interface InboxRow {
  paneKey: string
  title: string
  provider: Provider
  reason: InboxReason
  group: InboxGroup
  since?: string
  file?: string
  peer?: string
  terminalId: string | null
}

export interface InboxSection {
  group: InboxGroup
  rows: InboxRow[]
}

export function inboxRows(doc: WorkspaceDoc | null): InboxRow[] {
  const tabs = parseWorkspace(doc, [])
  const flag = doc?.inbox === true
  const rows: InboxRow[] = []
  for (const tab of tabs) {
    for (const pane of tab.panes) {
      const found: RemoteInbox | null = flag ? pane.inbox : pane.attention === 'waiting' ? { reason: 'question' } : pane.attention === 'done' ? { reason: 'done' } : null
      if (!found) continue
      rows.push({
        paneKey: pane.key,
        title: pane.title,
        provider: pane.provider,
        reason: found.reason,
        group: inboxGroupOf(found.reason),
        terminalId: pane.terminalId,
        ...(found.since ? { since: found.since } : {}),
        ...(found.file ? { file: found.file } : {}),
        ...(found.peer ? { peer: found.peer } : {}),
      })
    }
  }
  return sortInbox(rows)
}

export function inboxSections(rows: readonly InboxRow[]): InboxSection[] {
  return INBOX_GROUPS.flatMap((group) => {
    const list = rows.filter((r) => r.group === group)
    return list.length > 0 ? [{ group, rows: list }] : []
  })
}

export const inboxCount = (doc: WorkspaceDoc | null): number => inboxRows(doc).length

export function inboxSince(row: InboxRow): number | null {
  if (!row.since) return null
  const at = Date.parse(row.since)
  return Number.isFinite(at) ? at : null
}
