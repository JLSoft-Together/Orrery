import { waitReason, type LayoutNodeLike, type RemoteInbox, type RemotePaneGroup, type RemotePane, type RemoteTab, type RemoteWorkspaceSnapshot } from '@ccm/shared'
import { isTerm, presetOf, workCwd, type Pane, type Tab } from '../stores/workspace'
import type { LayoutNode } from './splitLayout'
import { groupsOf, leaves } from './splitLayout'

export const REMOTE_TITLE_MAX = 120

export interface RemotePaneFacts {
  title: string
  rail: string
  busy: boolean
  subagents: number
  waitingFor?: string
  branch?: string
}

export interface RemoteSummaryInput {
  tabs: readonly Tab[]
  panes: readonly Pane[]
  closedCount: number
  facts: (pane: Pane) => RemotePaneFacts
  inbox?: (pane: Pane) => RemoteInbox | undefined
}

const treeOf = (node: LayoutNode): LayoutNodeLike => ('pane' in node ? { pane: node.pane } : 'group' in node ? treeOf(node.node) : { dir: node.dir, ratio: node.ratio, a: treeOf(node.a), b: treeOf(node.b) })

const clip = (s: string): string => (s.length > REMOTE_TITLE_MAX ? s.slice(0, REMOTE_TITLE_MAX) : s)

const baseName = (p: string): string => p.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? ''

function viewTitle(pane: Pane, fallback: string): string {
  if (pane.kind === 'file' && pane.path) return baseName(pane.path)
  return fallback
}

function inboxOf(x: RemoteInbox | undefined): RemoteInbox | undefined {
  if (!x) return undefined
  return { reason: x.reason, ...(x.since ? { since: x.since } : {}), ...(x.file ? { file: clip(x.file) } : {}), ...(x.peer ? { peer: clip(x.peer) } : {}) }
}

function paneOf(pane: Pane, facts: RemotePaneFacts, inbox?: RemoteInbox): RemotePane {
  const term = isTerm(pane)
  const cwd = term || pane.kind === 'run' || pane.kind === 'changes' || pane.kind === 'log' ? (term ? workCwd(pane) : (pane.root ?? pane.cwd)) : undefined
  const branch = facts.branch ?? pane.branch
  const wait = facts.rail === 'asked' ? waitReason(facts.waitingFor) : undefined
  return {
    key: pane.key,
    kind: pane.kind ?? 'term',
    ...(term ? { preset: pane.preset, agent: presetOf(pane) } : {}),
    title: clip(term ? facts.title : viewTitle(pane, facts.title)),
    ...(cwd ? { cwd } : {}),
    ...(branch ? { branch } : {}),
    state: pane.state,
    rail: facts.rail,
    ...(wait ? { waitText: wait } : {}),
    ...(pane.attention ? { attention: pane.attention } : {}),
    subagents: facts.subagents,
    ...(pane.sessionId ? { sessionId: pane.sessionId } : {}),
    ...(pane.terminalId ? { terminalId: pane.terminalId } : {}),
    dirty: !!pane.dirty,
    busy: facts.busy,
    ...(inbox ? { inbox } : {}),
  }
}

export function groupsOfTab(root: LayoutNode): RemotePaneGroup[] {
  return groupsOf(root).map((g) => ({
    project: g.project,
    ...(g.name ? { name: clip(g.name) } : {}),
    ...(typeof g.tone === 'number' ? { tone: g.tone } : {}),
    ...(g.folded ? { folded: true } : {}),
    panes: g.keys,
  }))
}

export function buildSummary(input: RemoteSummaryInput, rev: number): RemoteWorkspaceSnapshot {
  const byKey = new Map(input.panes.map((p) => [p.key, p]))
  const tabs: RemoteTab[] = input.tabs.map((tab) => {
    const groups = groupsOfTab(tab.root)
    return {
      key: tab.key,
      ...(tab.name ? { name: tab.name } : {}),
      ...(tab.project ? { project: tab.project } : {}),
      tree: treeOf(tab.root),
      ...(tab.zoom ? { zoom: tab.zoom } : {}),
      ...(tab.folded ? { folded: true } : {}),
      ...(groups.length ? { groups } : {}),
      panes: leaves(tab.root).flatMap((k) => {
        const pane = byKey.get(k)
        return pane ? [paneOf(pane, input.facts(pane), inboxOf(input.inbox?.(pane)))] : []
      }),
    }
  })
  return { rev, tabs, closedCount: input.closedCount, ...(input.inbox ? { inbox: true as const } : {}) }
}

export const summaryHash = (s: Pick<RemoteWorkspaceSnapshot, 'tabs' | 'closedCount' | 'inbox'>): string => JSON.stringify([s.tabs, s.closedCount, s.inbox ?? false])
