import { INBOX_GROUPS, inboxGroupOf, waitReason, type AgentNoteLevel, type InboxGroup, type InboxReason, type TerminalSession } from '@ccm/shared'

export { INBOX_GROUPS, inboxGroupOf }
export type { InboxGroup, InboxReason }

export const SEVERITY: Record<InboxReason, number> = { menu: 0, permission: 1, question: 2, clash: 3, failed: 4, loop: 4.1, checks: 4.2, stuck: 4.3, note: 5, done: 6 }

export interface InboxNote {
  text: string
  level: AgentNoteLevel
  at: string
  merged: number
}

export interface InboxItem {
  id: string
  reason: InboxReason
  terminalId?: string
  paneKey?: string
  since?: string
  stamp: string
  note?: InboxNote
  question?: string
  file?: string
  peer?: string
}

export interface InboxMenu {
  paneKey: string
  terminalId?: string
  question: string
  since: number
}

export interface InboxClash {
  fileKey: string
  file: string
  peers: readonly { terminalId: string; name: string; at: number }[]
}

export interface InboxPane {
  key: string
  terminalId?: string
  failed: boolean
  asked: boolean
  since?: number
}

export interface InboxInput {
  terminals: readonly TerminalSession[]
  loops: ReadonlyMap<string, { since: string }>
  stuck: ReadonlySet<string>
  failedRoots: readonly string[]
  doneUnseen: ReadonlySet<string>
  notes?: ReadonlyMap<string, InboxNote>
  menus?: readonly InboxMenu[]
  clashes?: readonly InboxClash[]
  panes?: readonly InboxPane[]
  paneOf?: (terminalId: string) => string | undefined
  snoozed?: (key: string) => boolean
  dismissed: Readonly<Record<string, string>>
}

const norm = (p: string): string => p.replace(/[\\/]+$/, '').replace(/\//g, '\\').toLowerCase()
const inside = (dir: string, root: string): boolean => norm(dir) === norm(root) || norm(dir).startsWith(`${norm(root)}\\`)
const iso = (ms: number | undefined): string | undefined => (ms ? new Date(ms).toISOString() : undefined)

interface Reason {
  reason: InboxReason
  since?: string
  question?: string
}

interface Bucket {
  terminalId?: string
  paneKey?: string
  status?: string
  statusSince?: string
  reasons: Reason[]
}

export function waitingReason(x: Pick<TerminalSession, 'waitingFor'>): InboxReason {
  return waitReason(x.waitingFor) === 'permission' ? 'permission' : 'question'
}

export function inboxItems(input: InboxInput): InboxItem[] {
  const buckets = new Map<string, Bucket>()
  const live = new Map<string, TerminalSession>()
  for (const x of input.terminals) if (x.status !== 'stale') live.set(x.id, x)
  const bucket = (terminalId: string | undefined, paneKey: string | undefined): Bucket => {
    const id = terminalId && live.has(terminalId) ? terminalId : `pane:${paneKey}`
    let b = buckets.get(id)
    if (!b) {
      const x = terminalId ? live.get(terminalId) : undefined
      b = { ...(x ? { terminalId: x.id, status: x.status, statusSince: x.statusSince } : {}), reasons: [] }
      buckets.set(id, b)
    }
    if (paneKey && !b.paneKey) b.paneKey = paneKey
    return b
  }
  for (const x of live.values()) {
    const reasons: Reason[] = []
    if (x.status === 'waiting') reasons.push({ reason: waitingReason(x), since: x.statusSince })
    if (x.status === 'error') reasons.push({ reason: 'failed', since: x.statusSince })
    const loop = input.loops.get(x.id)
    if (loop) reasons.push({ reason: 'loop', since: loop.since })
    if (x.cwd && input.failedRoots.some((r) => inside(x.cwd!, r))) reasons.push({ reason: 'checks' })
    if (input.stuck.has(x.id)) reasons.push({ reason: 'stuck', since: x.lastActivityAt })
    if (input.doneUnseen.has(x.id) && x.status !== 'working') reasons.push({ reason: 'done', since: x.statusSince })
    const note = input.notes?.get(x.id)
    if (note) reasons.push({ reason: 'note', since: note.at })
    if (reasons.length) bucket(x.id, input.paneOf?.(x.id)).reasons.push(...reasons)
  }
  const menuPanes = new Set<string>()
  for (const m of input.menus ?? []) {
    menuPanes.add(m.paneKey)
    bucket(m.terminalId, m.paneKey).reasons.push({ reason: 'menu', since: iso(m.since), question: m.question })
  }
  for (const p of input.panes ?? []) {
    if (menuPanes.has(p.key) || (!p.failed && !p.asked)) continue
    const b = bucket(p.terminalId, p.key)
    if (p.failed) b.reasons.push({ reason: 'failed', since: iso(p.since) ?? b.statusSince })
    else if (!b.terminalId || b.status !== 'waiting') b.reasons.push({ reason: 'question', since: iso(p.since) ?? b.statusSince })
  }
  const out: InboxItem[] = []
  for (const [id, b] of buckets) {
    const top = b.reasons.sort((a, c) => SEVERITY[a.reason] - SEVERITY[c.reason])[0]
    if (!top) continue
    const note = top.reason === 'note' && b.terminalId ? input.notes?.get(b.terminalId) : undefined
    const stamp = note ? `note|${note.at}` : top.reason === 'menu' ? `menu|${top.since ?? ''}` : `${top.reason}|${b.status ?? 'pane'}|${b.statusSince ?? top.since ?? ''}`
    out.push({
      id,
      reason: top.reason,
      ...(b.terminalId ? { terminalId: b.terminalId } : {}),
      ...(b.paneKey ? { paneKey: b.paneKey } : {}),
      since: top.since,
      stamp,
      ...(note ? { note } : {}),
      ...(top.question ? { question: top.question } : {}),
    })
  }
  for (const c of input.clashes ?? []) {
    const peers = [...c.peers].filter((p) => live.has(p.terminalId)).sort((a, b) => b.at - a.at)
    const lead = peers[0]
    if (!lead || peers.length < 2) continue
    out.push({
      id: `clash:${c.fileKey}`,
      reason: 'clash',
      terminalId: lead.terminalId,
      ...(input.paneOf?.(lead.terminalId) ? { paneKey: input.paneOf(lead.terminalId) } : {}),
      since: iso(Math.min(...peers.map((p) => p.at))),
      stamp: `clash|${peers.map((p) => p.terminalId).sort().join(',')}`,
      file: c.file,
      peer: peers
        .slice(1)
        .map((p) => p.name)
        .join(', '),
    })
  }
  const shown = out.filter((i) => input.dismissed[i.id] !== i.stamp && !input.snoozed?.(i.paneKey ?? i.id))
  const at = (i: InboxItem): number => (i.since ? Date.parse(i.since) || Infinity : Infinity)
  const rank = (i: InboxItem): number => INBOX_GROUPS.indexOf(inboxGroupOf(i.reason))
  return shown.sort((a, b) => rank(a) - rank(b) || at(a) - at(b) || a.id.localeCompare(b.id))
}

export function inboxGroups(items: readonly InboxItem[]): { group: InboxGroup; items: InboxItem[] }[] {
  return INBOX_GROUPS.map((group) => ({ group, items: items.filter((i) => inboxGroupOf(i.reason) === group) })).filter((g) => g.items.length)
}

export function nextInboxItem(items: readonly InboxItem[], currentPane: string | null): InboxItem | null {
  if (!items.length) return null
  const at = currentPane ? items.findIndex((i) => i.paneKey === currentPane) : -1
  return items[(at + 1) % items.length] ?? null
}

export type InboxTone = 'waiting' | 'error' | 'warning' | 'muted'

export function inboxTone(items: readonly InboxItem[]): InboxTone {
  const groups = new Set(items.map((i) => inboxGroupOf(i.reason)))
  if (groups.has('menu') || groups.has('permission') || groups.has('question')) return 'waiting'
  if (groups.has('error')) return 'error'
  if (groups.has('clash')) return 'warning'
  return 'muted'
}

export function pruneDismissed(dismissed: Readonly<Record<string, string>>, liveIds: ReadonlySet<string>): Record<string, string> {
  return Object.fromEntries(Object.entries(dismissed).filter(([id]) => liveIds.has(id)))
}
