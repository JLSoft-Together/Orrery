import { waitReason, type LayoutNodeLike, type RemotePane, type RemoteTab, type RemoteWorkspaceSnapshot } from '@ccm/shared'
import { isTerm, presetOf, workCwd, type Pane, type Tab } from '../stores/workspace'
import type { LayoutNode } from './splitLayout'
import { leaves } from './splitLayout'

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
}

const treeOf = (node: LayoutNode): LayoutNodeLike => ('pane' in node ? { pane: node.pane } : { dir: node.dir, ratio: node.ratio, a: treeOf(node.a), b: treeOf(node.b) })

const clip = (s: string): string => (s.length > REMOTE_TITLE_MAX ? s.slice(0, REMOTE_TITLE_MAX) : s)

const baseName = (p: string): string => p.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? ''

function viewTitle(pane: Pane, fallback: string): string {
  if (pane.kind === 'file' && pane.path) return baseName(pane.path)
  return fallback
}

function paneOf(pane: Pane, facts: RemotePaneFacts): RemotePane {
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
  }
}

export function buildSummary(input: RemoteSummaryInput, rev: number): RemoteWorkspaceSnapshot {
  const byKey = new Map(input.panes.map((p) => [p.key, p]))
  const tabs: RemoteTab[] = input.tabs.map((tab) => ({
    key: tab.key,
    ...(tab.name ? { name: tab.name } : {}),
    ...(tab.project ? { project: tab.project } : {}),
    tree: treeOf(tab.root),
    ...(tab.zoom ? { zoom: tab.zoom } : {}),
    ...(tab.folded ? { folded: true } : {}),
    panes: leaves(tab.root).flatMap((k) => {
      const pane = byKey.get(k)
      return pane ? [paneOf(pane, input.facts(pane))] : []
    }),
  }))
  return { rev, tabs, closedCount: input.closedCount }
}

export const summaryHash = (s: Pick<RemoteWorkspaceSnapshot, 'tabs' | 'closedCount'>): string => JSON.stringify([s.tabs, s.closedCount])
