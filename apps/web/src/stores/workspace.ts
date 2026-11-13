import { defineStore } from 'pinia'
import type { CompactFlow } from '../lib/compactHandoff'
import type { SearchQuery } from '../lib/preview'
import { computed, ref, shallowRef, watch } from 'vue'
import {
  cleanPanePrompt,
  isAgentSessionId,
  isPtyPreset,
  OPEN_POLICY,
  PANE_LIVE_MAX,
  type OrrerySource,
  type PaneOpenRequest,
  type PaneOpenResult,
  type PeerLink,
  type PtyPreset,
  type SessionOpenError,
  type SessionOpenInput,
  type SessionTarget,
  type SessionOpenResult,
  type TerminalSession,
} from '@ccm/shared'
import {
  dockGroup,
  dockSpot,
  dockTree,
  findGroup,
  GROUP_CHROME,
  groupBoxes,
  groupNodes,
  groupOf,
  groupsOf,
  isGroup,
  isLayout,
  isLeaf,
  leaves,
  mapLeaves,
  moveGroup as moveGroupNode,
  neighborGroup,
  setGroupMeta as setGroupLayoutMeta,
  cleanGroups,
  type GroupMeta,
  settleGroups,
  takeGroup,
  ungrouped,
  movePane as moveLeaf,
  neighbor,
  removeLeaf,
  setRatio as setTreeRatio,
  splitLeaf,
  swapPanes as swapLeaves,
  type Edge,
  type FocusDir,
  type LayoutNode,
  type SplitDir,
} from '../lib/splitLayout'
import type { SavedLayout } from '../lib/projects'
import { hasDeepSeek, presetEnabled } from '../lib/desktop'
import { useDeepSeekStore } from './deepseek'
import { dirKey } from './favorites'
import { openedLinkId, usePeersStore } from './peers'
import { useTerminalsStore } from './terminals'

export type PaneState = 'parked' | 'starting' | 'running' | 'exited' | 'gone' | 'error'

/** Missing = 'term' (state saved before v21). */
export type PaneKind = 'term' | 'browser' | 'file' | 'changes' | 'log' | 'run'

export type PaneSpec =
  | { kind: 'term'; preset: PtyPreset }
  | { kind: 'browser'; url: string }
  | { kind: 'file'; path: string; line?: number }
  | { kind: 'changes' }
  | { kind: 'log' }
  | { kind: 'run' }

/** Split menu entries. */
export type SplitChoice = PtyPreset | 'browser' | 'project'

export interface Pane {
  key: string
  kind?: PaneKind
  /** Terms only; viewer panes keep 'shell' as a placeholder. */
  preset: PtyPreset
  /** Browser: http(s) URL or absolute file path (preview URLs change every run). Empty = blank page. */
  url?: string
  /** File viewer: absolute path, opened at `line`. */
  path?: string
  line?: number
  /** File viewer (v42): editor buffer differs from the file on disk. Runtime only, never persisted. */
  dirty?: boolean
  /** Changes / Log: git toplevel once known; Changes: number of changed files. */
  root?: string
  count?: number
  /** Changes: absolute path of the file to select; Log: commit hash to select. Runtime only, cleared once shown. */
  select?: string
  /** Browser page title / favicon, runtime only. */
  title?: string
  favicon?: string | null
  /** Last time this pane took focus: picks the agent a browser pane sends to. */
  focusedAt?: number
  cwd?: string
  project?: string
  /** Shell's current folder reported via OSC 7 / 9;9 after `cd`; runtime only, `cwd` stays the starting folder. */
  liveCwd?: string
  /** PTY host session id once spawned; lets a reload re-attach instead of starting over. */
  sessionId?: string
  pid?: number
  /** Monitor terminal running inside this pane (agent process descends from the PTY), found by `useWorkspaceLinks`. */
  terminalId?: string
  /** What runs now when it differs from `preset`, from the monitor link (`agentNow`): an agent typed in a shell pane, or 'shell' once an agent pane's agent exited. Runtime only. */
  agent?: PtyPreset
  /** Agent session to continue when spawning (`claude --resume` / `codex resume`). */
  resume?: string
  /** Latest agent session seen running in this pane (from the monitor link); what a later restore resumes. */
  agentSessionId?: string
  /** Claude session id minted for a new pane (`--session-id`), equal to `agentSessionId` until the monitor sees it run. */
  mintedSession?: string
  /** Session id of `agentSessionId` the monitor has not yet seen take a turn: no transcript, so `--resume` would fail. */
  untouchedSession?: string
  /** Latched when the agent starts waiting or finishes while the pane is out of sight; cleared when focused. */
  attention?: 'waiting' | 'done'
  /** Git branch of the pane's folder, pushed by the PTY host when HEAD changes. */
  branch?: string
  /** A command was entered in this shell: closing it may stop a dev server or build the monitor cannot see. */
  ranCommand?: boolean
  /** Script line started from the scripts menu: names the row; typed once (`pendingCommand`) when the shell is ready. */
  command?: string
  pendingCommand?: string
  openTicket?: string
  prefill?: string
  prefillSubmit?: boolean
  /** Set when `prefillSubmit` was armed by the Compact flow; value = arm time, so typing or a timeout can disarm it. */
  prefillAuto?: number
  typeKeys?: string
  compact?: CompactFlow
  openedBy?: string
  state: PaneState
  exitCode?: number
  error?: string
}

export interface Tab {
  key: string
  root: LayoutNode
  /** Pane that gets focus when the tab is selected. */
  focus: string
  /** Folder the tab was opened for: fixed for the tab's life, a `cd` never moves it. Missing = home / saved before v38. */
  project?: string
  name?: string
  folded?: boolean
  zoom?: string
}

export const TAB_NAME_MAX = 60

const zoomOf = (t: Partial<Tab>, root: LayoutNode): string | undefined => {
  const keys = leaves(root)
  return typeof t.zoom === 'string' && keys.length > 1 && keys.includes(t.zoom) ? t.zoom : undefined
}

export const CLOSED_MAX = 10
export const OPENED_NOTE_MS = 120_000
/** open_in_workspace: viewers one agent may keep open with unsaved edits before it is refused. */
export const AGENT_VIEWERS_MAX = 4
const HAND_BACK_TO = new Set<PtyPreset>(['claude', 'codex'])

export interface ClosedPane {
  key: string
  kind?: PaneKind
  preset: PtyPreset
  cwd?: string
  project?: string
  resume?: string
  url?: string
  path?: string
  line?: number
  name: string
  openedBy?: string
}

export interface ClosedAnchor {
  siblingKey: string
  group: string[]
  dir: SplitDir
  side: 'a' | 'b'
  ratio: number
  outer?: boolean
}

export interface ClosedTab {
  root: LayoutNode
  focus: string
  name?: string
  project?: string
  index: number
  panes: ClosedPane[]
}

export interface ClosedLoose {
  pane: ClosedPane
  anchor: ClosedAnchor | null
  project?: string
}

export interface ClosedEntry {
  id: number
  at: number
  tabs: ClosedTab[]
  loose: ClosedLoose[]
}

export const closedPanes = (e: ClosedEntry): ClosedPane[] => [...e.tabs.flatMap((t) => t.panes), ...e.loose.map((l) => l.pane)]

const copyTree = (n: LayoutNode): LayoutNode => mapLeaves(n, (k) => k)

const remapTree = (n: LayoutNode, map: ReadonlyMap<string, string>): LayoutNode => mapLeaves(n, (k) => map.get(k) ?? k)

export function anchorOf(root: LayoutNode, key: string): ClosedAnchor | null {
  if (isLeaf(root)) return null
  if (isGroup(root)) return anchorOf(root.node, key)
  for (const side of ['a', 'b'] as const) {
    const child = root[side]
    const solo = isLeaf(child) ? child.pane : isGroup(child) && isLeaf(child.node) ? child.node.pane : null
    if (solo !== key) continue
    const group = leaves(root[side === 'a' ? 'b' : 'a'])
    const anchor: ClosedAnchor = { siblingKey: side === 'a' ? group[0]! : group[group.length - 1]!, group, dir: root.dir, side, ratio: root.ratio }
    return isGroup(child) ? { ...anchor, outer: true } : anchor
  }
  return anchorOf(root.a, key) ?? anchorOf(root.b, key)
}

function wrapGroup(n: LayoutNode, want: ReadonlySet<string>, wrap: (n: LayoutNode) => LayoutNode, outer = false): LayoutNode | null {
  const keys = leaves(n)
  if (isGroup(n) && !outer) {
    const inner = wrapGroup(n.node, want, wrap, outer)
    if (inner) return { ...n, node: inner }
  }
  if (keys.length === want.size && keys.every((k) => want.has(k))) return wrap(n)
  if (isLeaf(n) || isGroup(n) || !keys.some((k) => want.has(k))) return null
  const a = wrapGroup(n.a, want, wrap, outer)
  if (a) return { ...n, a }
  const b = wrapGroup(n.b, want, wrap, outer)
  return b ? { ...n, b } : null
}

export const cleanTabName = (v: unknown): string | undefined =>
  typeof v === 'string' ? v.replace(/[\x00-\x1f\x7f]/g, ' ').trim().slice(0, TAB_NAME_MAX) || undefined : undefined

const tabExtras = (t: Partial<Tab>): Pick<Tab, 'name' | 'folded'> => ({ name: cleanTabName(t.name), folded: t.folded === true || undefined })

export function tabTitle(tab: Tab, paneOf: (key: string) => Pane | undefined, titleOf: (pane: Pane) => string): string {
  if (tab.name) return tab.name
  const groups = groupsOf(tab.root)
  if (groups.length > 1) return groups.map((g) => g.name || baseOf(g.project)).join(' + ')
  const primary = paneOf(primaryPaneKey(tab.root, paneOf))
  return primary ? titleOf(primary) : ''
}

const STORAGE_KEY = 'ccm.workspace.panes'
const RECENT_KEY = 'ccm.workspace.recent'
const PRESET_KEY = 'ccm.workspace.preset'
const RECENT_LIMIT = 8
// Survives app restarts (unlike the per-window session state): what "Restore previous workspace" rebuilds.
const LAST_KEY = 'ccm.workspace.last'
const PERSIST_MS = 400

interface ViewFields {
  kind?: PaneKind
  url?: string
  path?: string
  line?: number
}

interface LastPane extends ViewFields {
  key: string
  preset: PtyPreset
  cwd?: string
  /** The pane's starting folder when `cwd` is where its shell had moved to; used when that folder is gone. */
  home?: string
  project?: string
  resume?: string
}

export const isTerm = (p: Pick<Pane, 'kind'>): boolean => !p.kind || p.kind === 'term'

/** What the pane runs now: a shell pane running `claude` / `codex` shows and behaves as that agent, an agent pane whose agent quit as its shell; spawn keeps `preset`. */
export const presetOf = (p: Pick<Pane, 'preset' | 'agent'>): PtyPreset => p.agent ?? p.preset

/**
 * `Pane.agent` from the pane's linked monitor terminal: the live agent when it is not the preset; an agent pane
 * that was linked and whose session ended is back at the shell its preset runs in (until the next agent links).
 */
export function agentNow(p: Pick<Pane, 'preset' | 'agent' | 'terminalId'>, term: Pick<TerminalSession, 'status' | 'provider'> | undefined): PtyPreset | undefined {
  if (term && term.status !== 'stale') {
    const live = term.provider ?? 'claude'
    return live === p.preset ? undefined : live
  }
  return p.preset !== 'shell' && (p.terminalId || p.agent) ? 'shell' : undefined
}

export type SaveOutcome = { ok: true } | { ok: false; error: string }

/** Folder the pane works in now: links, quick open and the branch chip follow the shell's `cd`. */
export const workCwd = (p: Pick<Pane, 'cwd' | 'liveCwd'>): string | undefined => p.liveCwd ?? p.cwd

/** Pane that names a tab: first terminal in layout order, else the first leaf, so focus moves never relabel it. */
export function primaryPaneKey(root: LayoutNode, paneOf: (key: string) => Pick<Pane, 'kind'> | undefined): string {
  const keys = leaves(root)
  return keys.find((k) => {
    const p = paneOf(k)
    return !!p && isTerm(p)
  }) ?? keys[0]!
}

/** A tab's project; tabs saved before v38 fall back to their naming pane's starting folder. */
export function tabProject(tab: Tab, paneOf: (key: string) => Pick<Pane, 'kind' | 'cwd'> | undefined): string | undefined {
  return tab.project ?? paneOf(primaryPaneKey(tab.root, paneOf))?.cwd
}

export function paneProjectOf(tab: Tab | undefined, pane: Pick<Pane, 'project' | 'cwd'> & { key?: string }, paneOf: (key: string) => Pick<Pane, 'kind' | 'cwd'> | undefined): string | undefined {
  const group = tab && pane.key ? groupOf(tab.root, pane.key) : undefined
  return group || (pane.project ?? (tab ? tabProject(tab, paneOf) : undefined) ?? pane.cwd)
}

export const sameProject = (a: string, b: string): boolean => (!a || !b ? a === b : dirKey(a) === dirKey(b))

export function tabGroups(tab: Tab): Array<GroupMeta & { project: string; keys: string[] }> {
  return groupsOf(tab.root)
}

const nestedIn = (dir: string, root: string): boolean => dirKey(dir) === dirKey(root) || dirKey(dir).startsWith(dirKey(root) + '\\')
const related = (a: string, b: string): boolean => nestedIn(a, b) || nestedIn(b, a)

export function foreignProject(tab: Tab, dir: string | undefined, paneOf: (key: string) => Pick<Pane, 'kind' | 'cwd' | 'project'> | undefined): string | undefined {
  if (!dir) return undefined
  const home = tabProject(tab, paneOf)
  if (home && related(dir, home)) return undefined
  for (const k of leaves(tab.root)) {
    const own = paneOf(k)?.project
    if (own && related(dir, own)) return own
  }
  return dir
}

const isAbs = (p: unknown): p is string => typeof p === 'string' && (/^[A-Za-z]:[\\/]/.test(p) || p.startsWith('/') || p.startsWith('\\\\'))
const isWeb = (u: unknown): u is string => typeof u === 'string' && /^https?:\/\//i.test(u)

/** Keeps only what a pane of that kind needs; null when the stored entry is unusable. */
function viewFields(p: ViewFields): ViewFields | null {
  switch (p.kind) {
    case undefined:
    case 'term':
      return {}
    case 'browser':
      return p.url === undefined || p.url === '' || isWeb(p.url) || isAbs(p.url) ? { kind: 'browser', url: p.url ?? '' } : null
    case 'file':
      return isAbs(p.path) ? { kind: 'file', path: p.path, line: typeof p.line === 'number' && p.line > 0 ? Math.floor(p.line) : undefined } : null
    case 'changes':
      return { kind: 'changes' }
    case 'log':
      return { kind: 'log' }
    case 'run':
      return { kind: 'run' }
    default:
      return null
  }
}

function cleanPane<T extends ViewFields & { key: string; preset: unknown; cwd?: string }>(p: T): T | null {
  if (!p || typeof p.key !== 'string' || !isPtyPreset(p.preset)) return null
  const view = viewFields(p)
  if (!view) return null
  if ((view.kind === 'changes' || view.kind === 'log' || view.kind === 'run') && !isAbs(p.cwd)) return null
  return { ...p, kind: undefined, url: undefined, path: undefined, line: undefined, ...view, project: isAbs((p as { project?: unknown }).project) ? (p as { project?: string }).project : undefined }
}

function viewOf(p: ViewFields): ViewFields {
  return isTerm(p) ? {} : { kind: p.kind, url: p.url, path: p.path, line: p.line }
}

export interface LastWorkspace {
  at: string
  panes: LastPane[]
  tabs: Tab[]
}

export interface LastProject {
  key: string
  cwd?: string
  sessions: number
}

const projectKey = (cwd: string | undefined): string => cwd ?? ''

function lastTabProject(snap: LastWorkspace, tab: Tab): string | undefined {
  return tab.project ?? snap.panes.find((p) => p.key === leaves(tab.root)[0])?.cwd
}

export function lastProjectsOf(snap: LastWorkspace): LastProject[] {
  const byKey = new Map<string, LastProject>()
  const paneOf = new Map(snap.panes.map((p) => [p.key, p]))
  for (const tab of snap.tabs) {
    const cwd = lastTabProject(snap, tab)
    const key = projectKey(cwd)
    const entry = byKey.get(key) ?? { key, cwd, sessions: 0 }
    for (const k of leaves(tab.root)) {
      const p = paneOf.get(k)
      if (p && isTerm(p)) entry.sessions++
    }
    byKey.set(key, entry)
  }
  return [...byKey.values()]
}

function loadLast(): LastWorkspace | null {
  try {
    const raw = JSON.parse(readLocal(LAST_KEY) ?? 'null') as LastWorkspace | null
    if (!raw || !Array.isArray(raw.panes) || !Array.isArray(raw.tabs)) return null
    const panes = raw.panes
      .map(cleanPane)
      .filter((p): p is LastPane => !!p)
      .map((p) => ({ ...p, home: isAbs(p.home) && p.home !== p.cwd ? p.home : undefined }))
    const known = new Set(panes.map((p) => p.key))
    const tabs = raw.tabs
      .filter((t) => t && isLayout(t.root) && leaves(t.root).every((k) => known.has(k)))
      .map((t) => {
        const zoom = zoomOf(t, t.root)
        return { key: String(t.key), root: cleanGroups(t.root), focus: zoom ?? String(t.focus), project: isAbs(t.project) ? t.project : undefined, ...tabExtras(t), zoom }
      })
    return tabs.length ? { at: String(raw.at ?? ''), panes, tabs } : null
  } catch {
    return null
  }
}

function readLocal(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function writeLocal(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    // storage blocked: preference just is not remembered
  }
}

function loadRecent(): string[] {
  try {
    const raw = JSON.parse(readLocal(RECENT_KEY) ?? '[]') as unknown
    return Array.isArray(raw) ? raw.filter((d): d is string => typeof d === 'string').slice(0, RECENT_LIMIT) : []
  } catch {
    return []
  }
}

interface SavedPane extends ViewFields {
  key: string
  preset: PtyPreset
  cwd?: string
  project?: string
  /** Terms only. */
  sessionId?: string
  resume?: string
  agentSessionId?: string
  mintedSession?: string
  untouchedSession?: string
  ranCommand?: boolean
  command?: string
  openedBy?: string
  parked?: boolean
}

interface Saved {
  panes: SavedPane[]
  tabs: Tab[]
  active: string | null
}

// Only ids survive a reload; a fresh app start has a new PTY host, so attach answers "gone" and the pane is dropped.
function loadSaved(): Saved {
  const empty: Saved = { panes: [], tabs: [], active: null }
  try {
    const raw = JSON.parse(sessionStorage.getItem(STORAGE_KEY) ?? 'null') as Partial<Saved> | SavedPane[] | null
    if (!raw) return empty
    // P1 stored a bare pane list: one tab per pane.
    const list = Array.isArray(raw) ? raw : (raw.panes ?? [])
    const panes = list
      .map((p) => cleanPane(p))
      .filter((p): p is SavedPane => !!p && (!isTerm(p) || !!p.sessionId || p.parked === true))
      .map((p) => ({
        ...p,
        resume: agentId(p.resume),
        agentSessionId: agentId(p.agentSessionId),
        mintedSession: p.mintedSession && p.mintedSession === p.agentSessionId ? agentId(p.mintedSession) : undefined,
        untouchedSession: p.untouchedSession && p.untouchedSession === p.agentSessionId ? agentId(p.untouchedSession) : undefined,
        ranCommand: p.ranCommand === true || undefined,
        command: typeof p.command === 'string' ? p.command.slice(0, 300) : undefined,
        openedBy: typeof p.openedBy === 'string' && p.openedBy && p.openedBy !== p.key ? p.openedBy.slice(0, 64) : undefined,
        parked: isTerm(p) && !p.sessionId && p.parked === true ? true : undefined,
      }))
    const known = new Set(panes.map((p) => p.key))
    const rawTabs = Array.isArray(raw) ? panes.map((p) => ({ key: `t${p.key}`, root: { pane: p.key }, focus: p.key })) : (raw.tabs ?? [])
    const tabs: Tab[] = []
    for (const tab of rawTabs) {
      if (!tab || typeof tab.key !== 'string' || !isLayout(tab.root)) continue
      let root: LayoutNode | null = cleanGroups(tab.root)
      for (const key of leaves(tab.root)) if (!known.has(key)) root = root && removeLeaf(root, key)
      if (!root) continue
      const left = leaves(root)
      const zoom = zoomOf(tab as Partial<Tab>, root)
      tabs.push({
        key: tab.key,
        root,
        focus: zoom ?? (left.includes(tab.focus) ? tab.focus : left[0]!),
        project: isAbs((tab as Partial<Tab>).project) ? (tab as Tab).project : undefined,
        ...tabExtras(tab as Partial<Tab>),
        zoom,
      })
    }
    const placed = new Set(tabs.flatMap((t) => leaves(t.root)))
    return { panes: panes.filter((p) => placed.has(p.key)), tabs, active: Array.isArray(raw) ? null : (raw.active ?? null) }
  } catch {
    return empty
  }
}

const agentId = (v: unknown): string | undefined => (typeof v === 'string' && isAgentSessionId(v) ? v : undefined)

function baseOf(p: string): string {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i >= 0 ? p.slice(i + 1) : p
}

function fallbackName(p: Pane): string {
  if (p.kind === 'file') return baseOf(p.path ?? '') || 'file'
  if (p.kind === 'browser') return p.title || p.url || 'browser'
  return isTerm(p) ? p.preset : (p.kind ?? 'pane')
}

/** Agent session a later start continues; a session that never took a turn (minted, or new after /clear) has no transcript to `--resume`. */
export const untouched = (p: Pick<Pane, 'agentSessionId' | 'mintedSession' | 'untouchedSession'>): boolean =>
  !!p.agentSessionId && (p.mintedSession === p.agentSessionId || p.untouchedSession === p.agentSessionId)

export const resumeIdOf = (p: Pick<Pane, 'agentSessionId' | 'resume' | 'mintedSession' | 'untouchedSession'>): string | undefined => (untouched(p) ? p.resume : (p.agentSessionId ?? p.resume))

/** What the monitor link changes on a pane that shows session `id`; `turned` = the agent is working (a waiting dialog is no turn). */
export function sessionLinkPatch(p: Pick<Pane, 'agentSessionId' | 'mintedSession' | 'untouchedSession'>, id: string, turned: boolean): Partial<Pane> | null {
  if (p.agentSessionId !== id) return { agentSessionId: id, mintedSession: undefined, untouchedSession: turned ? undefined : id }
  return turned && (p.mintedSession || p.untouchedSession) ? { mintedSession: undefined, untouchedSession: undefined } : null
}

/** As `useTelegramInbound`: a pane keeps `agentSessionId` after its agent quits (a shell again), that session is over. */
export function agentLive(p: Pick<Pane, 'preset' | 'agent'>): boolean {
  return p.preset === 'shell' ? !!p.agent && p.agent !== 'shell' : p.agent !== 'shell'
}

/**
 * What a close-undo or the next app start reopens: an agent pane its preset (resumed when it ran); a shell pane where
 * claude / codex typed by hand still runs that agent with `--resume` (started from the shell's folder), else a shell.
 */
export function reopenAs(p: Pick<Pane, 'kind' | 'preset' | 'agent' | 'agentSessionId' | 'resume' | 'mintedSession' | 'untouchedSession' | 'cwd' | 'liveCwd'>): { preset: PtyPreset; cwd?: string; resume?: string } {
  if (!isTerm(p)) return { preset: p.preset, cwd: p.cwd }
  if (p.preset !== 'shell') {
    const resume = agentId(resumeIdOf(p))
    return { preset: p.preset, cwd: resume ? p.cwd : workCwd(p), resume }
  }
  const typed = p.agent === 'claude' || p.agent === 'codex' ? agentId(p.agentSessionId) : undefined
  return typed ? { preset: p.agent!, cwd: workCwd(p), resume: untouched(p) ? undefined : typed } : { preset: 'shell', cwd: workCwd(p) }
}

function closedPane(p: Pane, name: string): ClosedPane {
  const { preset, cwd, resume } = reopenAs(p)
  return { key: p.key, ...viewOf(p), preset, cwd, resume, name, ...(p.openedBy ? { openedBy: p.openedBy } : {}), ...(p.project ? { project: p.project } : {}) }
}

export type InsertRequest = { id: number; paneKey: string; text: string; image?: undefined } | { id: number; paneKey: string; image: true; text?: undefined }

function dirOfPath(p: string): string {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i > 0 ? p.slice(0, i) : p
}

let seq = 0
const newKey = (prefix: string): string => `${prefix}${Date.now().toString(36)}${(seq++).toString(36)}`

export const useWorkspaceStore = defineStore('workspace', () => {
  const saved = loadSaved()
  const panes = ref<Pane[]>(saved.panes.map(({ parked, ...p }) => ({ ...p, state: parked ? 'parked' : isTerm(p) ? 'starting' : 'running' })))
  const tabs = ref<Tab[]>(saved.tabs)
  const activeKey = ref<string | null>(panes.value.some((p) => p.key === saved.active) ? saved.active : (tabs.value[0]?.focus ?? null))

  const last = ref<LastWorkspace | null>(panes.value.length ? null : loadLast())
  const recentDirs = ref<string[]>(loadRecent())
  const storedPreset = readLocal(PRESET_KEY)
  const preset = ref<PtyPreset>(isPtyPreset(storedPreset) && presetEnabled(storedPreset, hasDeepSeek) ? storedPreset : 'claude')
  watch(preset, (p) => writeLocal(PRESET_KEY, p))

  const tabOf = (paneKey: string): Tab | undefined => tabs.value.find((t) => leaves(t.root).includes(paneKey))
  const startTab = activeKey.value ? tabOf(activeKey.value) : undefined
  if (startTab?.zoom && startTab.zoom !== activeKey.value) startTab.zoom = undefined
  const active = computed(() => panes.value.find((p) => p.key === activeKey.value) ?? null)
  const activeTab = computed(() => (activeKey.value ? tabOf(activeKey.value) : undefined) ?? null)
  const paneByTerminal = computed(() => new Map(panes.value.filter((p) => p.terminalId).map((p) => [p.terminalId!, p])))
  const attentionPanes = computed(() => panes.value.filter((p) => p.attention))
  /** Panes waiting on a yes before they (and their agents) are killed. */
  const pendingClose = ref<string[] | null>(null)
  let pendingQuiet = false

  let persistTimer = 0
  let persistDirty = false

  function persistNow(): void {
    window.clearTimeout(persistTimer)
    persistTimer = 0
    if (!persistDirty) return
    persistDirty = false
    const kept = panes.value.filter((p) => !isTerm(p) || p.state === 'parked' || (p.sessionId && p.state !== 'gone'))
    const data: Saved = {
      panes: kept.map((p) => ({
        key: p.key,
        preset: p.preset,
        cwd: p.cwd,
        project: p.project,
        sessionId: p.sessionId,
        resume: p.resume,
        agentSessionId: p.agentSessionId,
        mintedSession: p.mintedSession,
        untouchedSession: p.untouchedSession,
        ranCommand: p.ranCommand,
        command: p.command,
        openedBy: p.openedBy,
        parked: p.state === 'parked' || undefined,
        ...viewOf(p),
      })),
      tabs: tabs.value,
      active: activeKey.value,
    }
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(data))
    } catch {
      // storage blocked: panes just will not survive a reload
    }
    // Never prompts or output: preset, folder and the agent session id to resume are all that is kept.
    if (!panes.value.length) return
    const snapshot: LastWorkspace = {
      at: new Date().toISOString(),
      panes: panes.value.map((p) => {
        // As closedPane: a resumed agent must start where its session lives; a shell comes back where it was.
        const { preset: kind, cwd, resume } = reopenAs(p)
        return { key: p.key, preset: kind, cwd, home: cwd !== p.cwd ? p.cwd : undefined, project: p.project, resume, ...viewOf(p) }
      }),
      tabs: tabs.value,
    }
    writeLocal(LAST_KEY, JSON.stringify(snapshot))
  }

  watch(
    [panes, tabs, activeKey],
    () => {
      persistDirty = true
      if (!persistTimer) persistTimer = window.setTimeout(persistNow, PERSIST_MS)
    },
    { deep: true },
  )
  window.addEventListener('pagehide', persistNow)
  window.addEventListener('beforeunload', persistNow)

  type AgentOpen = Pick<Pane, 'openTicket' | 'prefill' | 'prefillSubmit' | 'openedBy'>

  function addPane(preset: PtyPreset, cwd?: string, resume?: string, extra: AgentOpen = {}): Pane {
    const id = resume && preset !== 'shell' && isAgentSessionId(resume) ? resume : undefined
    // Known before the first output, so Telegram links and ✈ work at once; a ticket's id is minted by the host instead.
    const minted = preset === 'claude' && !id && !extra.openTicket ? crypto.randomUUID() : undefined
    const pane: Pane = { key: newKey('p'), preset, cwd, resume: id, agentSessionId: id ?? minted, ...(minted ? { mintedSession: minted } : {}), state: 'starting', ...extra }
    panes.value.push(pane)
    return pane
  }

  function addView(view: ViewFields, cwd?: string): Pane {
    panes.value.push({ key: newKey('p'), preset: 'shell', cwd, state: 'running', ...view })
    // The reactive proxy: later writes through it must reach watchers.
    return panes.value[panes.value.length - 1]!
  }

  function addSpec(spec: PaneSpec, cwd?: string): Pane {
    switch (spec.kind) {
      case 'term':
        return addPane(spec.preset, cwd)
      case 'browser':
        return addView({ kind: 'browser', url: spec.url }, cwd)
      case 'file':
        return addView({ kind: 'file', path: spec.path, line: spec.line }, cwd)
      case 'changes':
        return addView({ kind: 'changes' }, cwd)
      case 'log':
        return addView({ kind: 'log' }, cwd)
      case 'run':
        return addView({ kind: 'run' }, cwd)
    }
  }

  type Placement = { key: string; dir: SplitDir }

  const unzoom = (tab: Tab | undefined): void => {
    if (tab?.zoom) tab.zoom = undefined
  }

  function placeNew(pane: Pane, split?: Placement): void {
    const tab = split ? tabOf(split.key) : undefined
    if (tab && split) {
      tab.root = splitLeaf(tab.root, split.key, pane.key, split.dir)
      settle(tab)
      tab.focus = pane.key
      unzoom(tab)
    } else tabs.value.push({ key: newKey('t'), root: { pane: pane.key }, focus: pane.key, project: pane.cwd })
    activate(pane.key)
  }

  const sameDir = (a: string | undefined, b: string | undefined): boolean => !!a && !!b && dirKey(a) === dirKey(b)

  /** Reuses a browser pane already showing `url`; else a new tab (or a split next to `split.key`). */
  function openBrowser(url: string, opts: { cwd?: string; split?: Placement } = {}): Pane {
    const same = url ? panes.value.find((p) => p.kind === 'browser' && (p.url === url || (isAbs(url) && sameDir(p.url, url)))) : undefined
    if (same) {
      activate(same.key)
      return same
    }
    const pane = addView({ kind: 'browser', url }, opts.cwd)
    placeNew(pane, opts.split)
    return pane
  }

  /** One viewer per file: an open one jumps to `line` instead; every other file gets a viewer of its own. */
  function openFile(path: string, line?: number, opts: { cwd?: string; split?: Placement } = {}): Pane {
    const same = panes.value.find((p) => p.kind === 'file' && sameDir(p.path, path))
    if (same) {
      same.line = line
      activate(same.key)
      return same
    }
    const pane = addView({ kind: 'file', path, line }, opts.cwd ?? dirOfPath(path))
    placeNew(pane, opts.split)
    return pane
  }

  /** One Changes pane per repo (git toplevel once known, else the folder it was opened for); `select` = file to show. */
  function openChanges(cwd: string, opts: { split?: Placement; select?: string } = {}): Pane {
    const same = panes.value.find((p) => p.kind === 'changes' && (sameDir(p.root, cwd) || sameDir(p.cwd, cwd)))
    if (same) {
      if (opts.select) same.select = opts.select
      activate(same.key)
      return same
    }
    const pane = addView({ kind: 'changes' }, cwd)
    if (opts.select) pane.select = opts.select
    placeNew(pane, opts.split)
    return pane
  }

  /** One Log pane per repo, like Changes; `select` = commit hash to show. */
  function openLog(cwd: string, opts: { split?: Placement; select?: string } = {}): Pane {
    const same = panes.value.find((p) => p.kind === 'log' && (sameDir(p.root, cwd) || sameDir(p.cwd, cwd)))
    if (same) {
      if (opts.select) same.select = opts.select
      activate(same.key)
      return same
    }
    const pane = addView({ kind: 'log' }, cwd)
    if (opts.select) pane.select = opts.select
    placeNew(pane, opts.split)
    return pane
  }

  /** One Run pane per project root (`cwd`). */
  function openRun(cwd: string, opts: { split?: Placement } = {}): Pane {
    const same = panes.value.find((p) => p.kind === 'run' && sameDir(p.cwd, cwd))
    if (same) {
      activate(same.key)
      return same
    }
    const pane = addView({ kind: 'run' }, cwd)
    placeNew(pane, opts.split)
    return pane
  }

  /** Text or a clipboard image (^V) for an agent pane's input line; never followed by Enter. */
  const insertRequest = shallowRef<InsertRequest | null>(null)
  let insertSeq = 0
  function insertInto(paneKey: string, text: string): void {
    insertRequest.value = { id: ++insertSeq, paneKey, text }
    activate(paneKey)
  }
  const restartRequest = shallowRef<{ id: number; paneKey: string } | null>(null)
  function requestRestart(paneKey: string): void {
    restartRequest.value = { id: ++insertSeq, paneKey }
  }
  function insertImage(paneKey: string): void {
    insertRequest.value = { id: ++insertSeq, paneKey, image: true }
    activate(paneKey)
  }

  /** Find in Files asked for from outside the workspace (device inspector); WorkspaceView runs it. */
  const searchRequest = shallowRef<{ id: number; query: SearchQuery; root: string | null } | null>(null)
  function requestSearch(query: SearchQuery, root: string | null = null): void {
    searchRequest.value = { id: ++insertSeq, query, root }
  }

  const isAgentPane = (p: Pane): boolean => isTerm(p) && presetOf(p) !== 'shell' && (p.state === 'running' || p.state === 'starting')

  /**
   * Where a browser pane's "Send to agent" lands: a Claude/Codex pane in the same tab (linked to a monitor
   * session first, then most recently focused), else the most recently focused agent anywhere, else a picker.
   */
  function resolveAgentPane(browserKey: string): { pane: Pane } | { choices: Pane[] } {
    const byRecent = (a: Pane, b: Pane): number => (b.terminalId ? 1 : 0) - (a.terminalId ? 1 : 0) || (b.focusedAt ?? 0) - (a.focusedAt ?? 0)
    const tab = tabOf(browserKey)
    const local = tab ? leaves(tab.root).map((k) => panes.value.find((p) => p.key === k)).filter((p): p is Pane => !!p && isAgentPane(p)) : []
    if (local.length) return { pane: local.sort(byRecent)[0]! }
    const all = panes.value.filter(isAgentPane).sort((a, b) => (b.focusedAt ?? 0) - (a.focusedAt ?? 0))
    const recent = all.find((p) => p.focusedAt)
    return recent ? { pane: recent } : { choices: all }
  }

  /** Pane canvas in px, written by WorkspaceView: decides which side a new pane splits off. Not saved. */
  const canvasSize = ref({ w: 0, h: 0 })

  const paneOf = (k: string): Pane | undefined => panes.value.find((p) => p.key === k)
  const within = (dir: string, root: string): boolean => dirKey(dir) === dirKey(root) || dirKey(dir).startsWith(dirKey(root) + '\\')

  /** The tab working on `cwd`'s project: the active tab when it is one, else the one focused last. Home tabs match no folder. */
  function projectTab(cwd?: string): Tab | undefined {
    const matches = (t: Tab): boolean => {
      const groups = groupsOf(t.root)
      if (cwd && groups.length) return groups.some((g) => !!g.project && within(cwd, g.project))
      const project = tabProject(t, paneOf)
      return cwd ? !!project && within(cwd, project) : !project
    }
    if (activeTab.value && matches(activeTab.value)) return activeTab.value
    const lastFocus = (t: Tab): number => Math.max(0, ...leaves(t.root).map((k) => paneOf(k)?.focusedAt ?? 0))
    return tabs.value.filter(matches).sort((a, b) => lastFocus(b) - lastFocus(a))[0]
  }

  /** `launch` / `openFromAgent` keep their old answers; `open` never refused, so a refusal falls back to a plain pane. */
  function open(preset: PtyPreset, cwd?: string, resume?: string, opts: { tab?: boolean } = {}): Pane {
    const res = openSession({ preset, cwd, resume, tab: opts.tab }, { source: 'restore' })
    return res.ok ? paneOf(res.paneKey)! : joinProject(addPane(preset, cwd, resume), opts)
  }

  /** `open` for a pane the owner picked: null when the agent is not set up (Settings › DeepSeek opens instead). */
  function launch(preset: PtyPreset, cwd?: string, resume?: string, opts: { tab?: boolean } = {}): Pane | null {
    const res = openSession({ preset, cwd, resume, tab: opts.tab }, { source: 'menu' })
    return res.ok ? paneOf(res.paneKey)! : null
  }

  /**
   * A new terminal joins its project's tab beside the focused pane; a tab of its own only when the project has none,
   * no pane has room left, or `opts.tab` asks for one.
   */
  function joinProject(pane: Pane, opts: { tab?: boolean } = {}): Pane {
    const tab = opts.tab ? undefined : projectTab(pane.cwd)
    if (tab && dockNear(tab, pane.key, tab.focus, pane.cwd)) {
      tab.focus = pane.key
      unzoom(tab)
    } else tabs.value.push({ key: newKey('t'), root: { pane: pane.key }, focus: pane.key, project: pane.cwd })
    activate(pane.key)
    return pane
  }

  const liveState = (p: Pane): boolean => p.state === 'running' || p.state === 'starting'

  function holdsSession(p: Pane, id: string): boolean {
    if (!isTerm(p) || !liveState(p) || !agentLive(p)) return false
    if (p.agentSessionId === id) return true
    const t = p.terminalId ? useTerminalsStore().byId[p.terminalId] : undefined
    return !!t && (t.claudeSessionId === id || t.codexSessionId === id || t.deepseekSessionId === id)
  }

  /** An agent session some process still runs: in a pane here, or in a terminal outside the workspace. */
  function sessionLive(id: string): boolean {
    if (panes.value.some((p) => holdsSession(p, id))) return true
    return useTerminalsStore().list.some((t) => (t.claudeSessionId === id || t.codexSessionId === id || t.deepseekSessionId === id) && t.status !== 'stale' && !t.endedAt)
  }

  type OpenCaller = {
    source: OrrerySource
    /** mcp / url: the caller found DeepSeek not installed / not logged in (it already told the owner). */
    setupBlocked?: boolean
    liveElsewhere?: (sessionId: string) => boolean
  }

  /** The one way an agent or shell pane is opened or resumed; OPEN_POLICY decides what each source may do. */
  function openSession(input: SessionOpenInput, caller: OpenCaller): SessionOpenResult {
    const fail = (error: SessionOpenError): SessionOpenResult => ({ ok: false, error })
    const policy = OPEN_POLICY[caller.source]
    const { preset, cwd } = input
    if (!isPtyPreset(preset) || !presetEnabled(preset, hasDeepSeek)) return fail('preset')
    if (preset === 'deepseek' && (caller.source === 'menu' || caller.source === 'palette' ? useDeepSeekStore().gate(preset) : !!caller.setupBlocked)) return fail('setup')
    // The limited sources come from outside the app and must name a folder; in-app callers may open the home folder.
    if (policy.limit ? typeof cwd !== 'string' || !cwd.trim() : cwd !== undefined && typeof cwd !== 'string') return fail('cwd')
    const resume = input.resume === undefined || input.resume === '' ? undefined : preset !== 'shell' && isAgentSessionId(input.resume) ? input.resume : null
    if (resume === null) return fail('resume')
    if (resume) {
      const running = panes.value.find((p) => holdsSession(p, resume))
      if (running) {
        activate(running.key)
        return { ok: true, paneKey: running.key }
      }
      if (sessionLive(resume) || caller.liveElsewhere?.(resume)) return fail('live')
    }
    if (policy.limit && panes.value.filter((p) => isTerm(p) && liveState(p)).length >= PANE_LIVE_MAX) return fail('limit')
    const from = policy.dock && typeof input.from === 'string' && input.from ? panes.value.find((p) => isTerm(p) && p.sessionId === input.from) : undefined
    const prompt = preset === 'shell' ? undefined : cleanPanePrompt(input.prompt)
    const extra: AgentOpen = {
      openTicket: typeof input.id === 'string' && input.id ? input.id : undefined,
      prefill: prompt,
      ...(prompt && (policy.submit ? input.submit !== false : policy.askSubmit && input.submit === true) ? { prefillSubmit: true } : {}),
      ...(from ? { openedBy: from.key } : {}),
    }
    const pane = markOpened(addPane(preset, cwd, resume, extra))
    const fromTab = from ? tabOf(from.key) : undefined
    if (!from || !fromTab) return { ok: true, paneKey: joinProject(pane, { tab: input.tab }).key }
    const size = canvasSize.value
    const spot = size.w && size.h ? dockSpot(fromTab.root, from.key, size) : null
    if (spot) {
      if (!dockNear(fromTab, pane.key, from.key, cwd)) attach(fromTab, { pane: pane.key }, cwd ?? '', from.key, spot.dir === 'col' ? 'bottom' : 'right')
      unzoom(fromTab)
    } else tabs.value.push({ key: newKey('t'), root: { pane: pane.key }, focus: pane.key, project: tabProject(fromTab, paneOf) ?? cwd })
    return { ok: true, paneKey: pane.key }
  }

  /** MCP `open_session` (has a calling pane) and `agent-monitor://open`. */
  function openFromAgent(req: PaneOpenRequest, opts: { liveElsewhere?: (sessionId: string) => boolean; setupBlocked?: boolean } = {}): PaneOpenResult {
    return openSession(req, { source: req.from ? 'mcp' : 'url', ...opts })
  }

  /**
   * open_in_workspace (v80): the file viewer, or Changes with the file selected, beside the calling pane. Nothing
   * is activated and the view is not switched, so the owner's keyboard focus stays where it is (like open_session).
   * `path` was already checked by the PTY host to be inside the caller's project.
   */
  function showForAgent(from: string, spec: { path: string; line?: number; diff: boolean; root?: string }): { paneKey: string; reused: boolean } | 'caller' | 'limit' {
    const caller = from ? panes.value.find((p) => isTerm(p) && p.sessionId === from) : undefined
    // Second check of the host's: the viewer must show a file of the caller's own folder.
    // `root` is the host's real path of the same pane's folder (a project opened through a junction differs from `cwd`).
    if (!caller?.cwd || !(within(spec.path, caller.cwd) || (!!spec.root && within(spec.path, spec.root)))) return 'caller'
    const callerTab = tabOf(caller.key)
    const project = paneProjectOf(callerTab, caller, paneOf)
    const existing = spec.diff
      ? panes.value.find((p) => p.kind === 'changes' && !!project && (sameDir(p.root, project) || sameDir(p.cwd, project)))
      : panes.value.find((p) => p.kind === 'file' && sameDir(p.path, spec.path))
    if (existing) {
      if (spec.diff) existing.select = spec.path
      else existing.line = spec.line
      return { paneKey: existing.key, reused: true }
    }
    if (!spec.diff) {
      // One viewer per calling agent: the next file replaces its last one unless that has unsaved edits.
      const mine = panes.value.filter((p) => p.kind === 'file' && p.openedBy === caller.key)
      const clean = mine.find((p) => !p.dirty)
      if (clean) {
        clean.path = spec.path
        clean.line = spec.line
        return { paneKey: clean.key, reused: true }
      }
      if (mine.length >= AGENT_VIEWERS_MAX) return 'limit'
    }
    const pane = spec.diff ? addView({ kind: 'changes' }, project) : addView({ kind: 'file', path: spec.path, line: spec.line }, project ?? dirOfPath(spec.path))
    if (spec.diff) pane.select = spec.path
    else pane.openedBy = caller.key
    const size = canvasSize.value
    if (callerTab && size.w && size.h && dockNear(callerTab, pane.key, caller.key, undefined)) unzoom(callerTab)
    else tabs.value.push({ key: newKey('t'), root: { pane: pane.key }, focus: pane.key, project })
    return { paneKey: pane.key, reused: false }
  }

  /**
   * hand_back (v81): queue `prompt` as a submitting prefill on the Claude / Codex pane that opened the calling
   * DeepSeek pane; TermPane sends it once that agent is idle. One pending prompt per origin: a second hand-back,
   * or one landing while a Compact flow owns the prefill, is refused instead of overwriting it.
   */
  function handBack(from: string, prompt: string): Pane | 'caller' | 'limit' {
    const caller = from ? panes.value.find((p) => isTerm(p) && p.sessionId === from) : undefined
    if (!caller || caller.preset !== 'deepseek' || !caller.openedBy) return 'caller'
    const origin = paneOf(caller.openedBy)
    if (!origin || !isTerm(origin) || origin.state !== 'running' || !HAND_BACK_TO.has(presetOf(origin))) return 'caller'
    if (origin.prefill || origin.compact) return 'limit'
    origin.prefill = prompt
    origin.prefillSubmit = true
    return origin
  }

  /** Agent panes (Claude / Codex) running in a project folder, without those of its sub-projects in `nested`. */
  function sessionPanes(project: string, nested: string[] = []): Pane[] {
    return panes.value.filter((p) => {
      const dir = isTerm(p) ? (workCwd(p) ?? p.cwd) : undefined
      return !!dir && p.state === 'running' && HAND_BACK_TO.has(presetOf(p)) && ownsDir(dir, project, nested)
    })
  }

  function ownsDir(dir: string, project: string, nested: string[] = []): boolean {
    return within(dir, project) && !nested.some((n) => within(n, project) && within(dir, n))
  }

  /** The one pane a session action is for: the targeted one (`gone` once it closed), else the project's only agent pane. */
  function resolveTarget(target: SessionTarget): Pane | 'none' | 'many' | 'gone' {
    const candidates = sessionPanes(target.project, target.nested)
    if (target.terminalId || target.sessionId || target.pane) {
      const pane = candidates.find((p) => (target.terminalId ? p.terminalId === target.terminalId : target.sessionId ? p.sessionId === target.sessionId : p.key === target.pane))
      return pane ?? 'gone'
    }
    if (candidates.length === 1) return candidates[0]!
    return candidates.length ? 'many' : 'none'
  }

  /** Prefills and submits a prompt; a pane with a prompt or Compact flow pending is `busy` and keeps it. */
  function sendPrompt(target: SessionTarget, prompt: string): Pane | 'none' | 'many' | 'gone' | { busy: Pane } {
    const pane = resolveTarget(target)
    if (typeof pane === 'string') return pane
    if (pane.prefill || pane.compact) return { busy: pane }
    pane.prefill = prompt
    pane.prefillSubmit = true
    return pane
  }

  function typeKeys(target: SessionTarget, keys: string): Pane | 'none' | 'many' | 'gone' | 'busy' {
    const pane = resolveTarget(target)
    if (typeof pane === 'string') return pane
    if (pane.typeKeys) return 'busy'
    pane.typeKeys = keys
    return pane
  }

  const telegramPanes = sessionPanes
  const telegramInbound = (project: string, prompt: string, target?: { terminalId?: string; sessionId?: string }, nested: string[] = []) => sendPrompt({ project, nested, ...target }, prompt)
  const telegramKeys = (project: string, keys: string, target?: { terminalId?: string; sessionId?: string }, nested: string[] = []) => typeKeys({ project, nested, ...target }, keys)

  function telegramTyped(paneKey: string, ms: number): Promise<boolean> {
    const state = (): 'gone' | 'wait' | 'typed' => {
      const p = paneOf(paneKey)
      return !p ? 'gone' : p.typeKeys ? 'wait' : 'typed'
    }
    if (state() !== 'wait') return Promise.resolve(state() === 'typed')
    return new Promise((resolve) => {
      const finish = (ok: boolean): void => {
        clearTimeout(timer)
        stop()
        if (!ok) {
          const p = paneOf(paneKey)
          if (p) p.typeKeys = undefined
        }
        resolve(ok)
      }
      const timer = setTimeout(() => finish(false), ms)
      const stop = watch(state, (s) => {
        if (s !== 'wait') finish(s === 'typed')
      })
    })
  }

  const freshOpened = new Map<string, number>()

  function markOpened(pane: Pane): Pane {
    if (pane.openedBy) freshOpened.set(pane.key, Date.now())
    return pane
  }

  const peers = usePeersStore()
  let openedSince = new Map<string, string>()
  const openedPairs = computed(() => {
    const out: Array<{ pane: string; id: string; a: string; b: string }> = []
    for (const p of panes.value) {
      if (!p.openedBy || !p.terminalId) continue
      const by = paneOf(p.openedBy)
      if (!by?.terminalId || by.terminalId === p.terminalId) continue
      const id = openedLinkId(by.terminalId, p.terminalId)
      if (!out.some((e) => e.id === id)) out.push({ pane: p.key, id, a: by.terminalId, b: p.terminalId })
    }
    return out
  })

  watch(
    () => openedPairs.value.map((e) => `${e.pane}:${e.id}`).join('|'),
    () => {
      const at = Date.now()
      const iso = new Date(at).toISOString()
      const since = new Map<string, string>()
      const entries = openedPairs.value.map((e) => {
        const first = openedSince.get(e.id) ?? iso
        since.set(e.id, first)
        const link: PeerLink = { id: e.id, kind: 'opened', a: e.a, b: e.b, ab: 1, ba: 0, firstAt: first, lastAt: first, lastDir: 'ab' }
        return { pane: e.pane, link }
      })
      openedSince = since
      const loud = new Set(entries.filter((e) => at - (freshOpened.get(e.pane) ?? -Infinity) < OPENED_NOTE_MS).map((e) => e.pane))
      for (const e of entries) freshOpened.delete(e.pane)
      for (const [key, t0] of freshOpened) if (at - t0 >= OPENED_NOTE_MS) freshOpened.delete(key)
      peers.setOpened(entries.filter((e) => !loud.has(e.pane)).map((e) => e.link), true)
      if (loud.size) peers.setOpened(entries.map((e) => e.link))
    },
    { immediate: true },
  )

  /** Same folder as the target, next to it; same agent unless one is picked (Windows Terminal's "duplicate" split). */
  function split(targetKey: string, dir: SplitDir, spec?: PaneSpec, opts: { cwd?: string; resume?: string } = {}): Pane | null {
    const tab = tabOf(targetKey)
    const target = panes.value.find((p) => p.key === targetKey)
    if (!tab || !target) return null
    const chosen: PaneSpec = spec ?? { kind: 'term', preset: isTerm(target) ? target.preset : preset.value }
    if (chosen.kind === 'term' && (!presetEnabled(chosen.preset, hasDeepSeek) || useDeepSeekStore().gate(chosen.preset))) return null
    if (opts.resume) {
      const running = panes.value.find((p) => holdsSession(p, opts.resume!))
      if (running) {
        activate(running.key)
        return running
      }
    }
    const cwd = opts.cwd ?? workCwd(target)
    const pane = chosen.kind === 'term' && opts.resume ? addPane(chosen.preset, cwd, opts.resume) : addSpec(chosen, cwd)
    if (opts.cwd) remember(opts.cwd)
    const group = groupOf(tab.root, targetKey) ?? homeOf(tab)
    if (!opts.cwd || (!!group && within(opts.cwd, group))) {
      tab.root = splitLeaf(tab.root, targetKey, pane.key, dir)
      settle(tab)
    } else attach(tab, { pane: pane.key }, opts.cwd, targetKey, dir === 'row' ? 'right' : 'bottom')
    tab.focus = pane.key
    unzoom(tab)
    activate(pane.key)
    return pane
  }

  /** One pane per agent session: an already open one is focused instead of starting a second writer. */
  function resumeSession(preset: PtyPreset, cwd: string, sessionId: string): Pane {
    const open_ = panes.value.find((p) => p.agentSessionId === sessionId && (p.state === 'running' || p.state === 'starting'))
    if (open_) {
      activate(open_.key)
      return open_
    }
    return open(preset, cwd, sessionId)
  }

  /** New shell in `dir` that types `run` + Enter once its prompt is up: only ever from the user picking a script. */
  function runScript(dir: string | undefined, run: string, opts: { split?: Placement } = {}): Pane {
    addPane('shell', dir)
    const pane = panes.value[panes.value.length - 1]!
    pane.command = run
    pane.pendingCommand = run
    placeNew(pane, opts.split)
    return pane
  }

  /**
   * Adds a saved layout's tabs. Browser / file panes already open stay where they are (their leaf is dropped);
   * agents resume only when the layout stored session ids.
   */
  function openLayout(layout: Pick<SavedLayout, 'panes' | 'tabs'> & { project?: string }): number {
    let first: string | null = null
    let reused: string | null = null
    for (const tab of layout.tabs) {
      const keys = new Map<string, string>()
      let root: LayoutNode | null = tab.root
      for (const id of leaves(tab.root)) {
        const spec = layout.panes[id]
        const same =
          spec?.kind === 'browser' && spec.url
            ? panes.value.find((p) => p.kind === 'browser' && p.url === spec.url)
            : spec?.kind === 'file'
              ? panes.value.find((p) => p.kind === 'file' && sameDir(p.path, spec.path))
              : spec?.kind === 'run'
                ? panes.value.find((p) => p.kind === 'run' && sameDir(p.cwd, spec.cwd))
                : undefined
        if (!spec || same) {
          reused ??= same?.key ?? null
          root = root && removeLeaf(root, id)
          continue
        }
        const pane =
          spec.kind === 'term'
            ? addPane(spec.preset ?? 'shell', spec.cwd, spec.resume)
            : spec.kind === 'browser'
              ? addView({ kind: 'browser', url: spec.url ?? '' }, spec.cwd)
              : spec.kind === 'file'
                ? addView({ kind: 'file', path: spec.path, line: spec.line }, spec.cwd)
                : addView({ kind: spec.kind === 'log' || spec.kind === 'run' ? spec.kind : 'changes' }, spec.cwd)
        keys.set(id, pane.key)
      }
      if (!root) continue
      const mapped = ungrouped(mapLeaves(root, (k) => keys.get(k)!))
      const focus = (tab.focus && keys.get(tab.focus)) || leaves(mapped)[0]!
      const made: Tab = { key: newKey('t'), root: mapped, focus, project: layout.project ?? panes.value.find((p) => p.key === leaves(mapped)[0])?.cwd }
      for (const k of leaves(mapped)) {
        const p = paneOf(k)
        if (p && !p.project) p.project = foreignProject(made, p.kind === 'changes' || p.kind === 'log' ? (p.root ?? p.cwd) : p.cwd, paneOf)
      }
      tabs.value.push(made)
      const added = tabs.value[tabs.value.length - 1]!
      settle(added)
      for (const meta of tab.groups ?? []) {
        const own = groupsOf(added.root).find((g) => !!g.project && (sameProject(g.project, meta.project) || within(g.project, meta.project)))
        if (own) added.root = setGroupLayoutMeta(added.root, own.project, { name: meta.name, tone: meta.tone, folded: meta.folded })
      }
      first ??= focus
    }
    const target = first ?? reused
    if (target) activate(target)
    return first ? tabs.value.length : 0
  }

  /** Last output / input per pane in this window: the sidebar's age when no monitor session is linked. Not persisted. */
  const lastSeen = ref<Record<string, number>>({})
  function touch(key: string, at = Date.now()): void {
    lastSeen.value[key] = at
  }

  const lastProjects = computed<LastProject[]>(() => (last.value ? lastProjectsOf(last.value) : []))

  /** Folders a restore must check first: shells saved where they had `cd`-ed to. */
  const lastMovedDirs = computed(() => (last.value?.panes ?? []).filter((p) => p.home && p.cwd).map((p) => p.cwd!))

  function restoreLast(run?: ReadonlySet<string>, gone?: ReadonlySet<string>): void {
    const snap = last.value
    last.value = null
    writeLocal(LAST_KEY, 'null')
    if (!snap) return
    const sleeping = new Set<string>()
    if (run) for (const tab of snap.tabs) if (!run.has(projectKey(lastTabProject(snap, tab)))) for (const k of leaves(tab.root)) sleeping.add(k)
    const keys = new Map<string, string>()
    for (const p of snap.panes) {
      if (!isTerm(p)) {
        const view = addView(viewOf(p), p.cwd)
        if (p.project) view.project = p.project
        keys.set(p.key, view.key)
        continue
      }
      const pane = addPane(p.preset, p.home && p.cwd && gone?.has(p.cwd) ? p.home : p.cwd, p.resume)
      if (p.project) pane.project = p.project
      if (sleeping.has(p.key)) update(pane.key, { state: 'parked' })
      keys.set(p.key, pane.key)
    }
    const remap = (n: LayoutNode): LayoutNode => mapLeaves(n, (k) => keys.get(k)!)
    const fresh = tabs.value.length
    for (const tab of snap.tabs) {
      const zoom = tab.zoom ? keys.get(tab.zoom) : undefined
      tabs.value.push({
        key: newKey('t'),
        root: remap(tab.root),
        focus: zoom ?? keys.get(tab.focus) ?? keys.get(leaves(tab.root)[0]!)!,
        project: tab.project ?? snap.panes.find((p) => p.key === leaves(tab.root)[0])?.cwd,
        ...tabExtras(tab),
        zoom,
      })
    }
    for (const tab of tabs.value.slice(fresh)) settle(tab)
    activeKey.value = tabs.value[0]?.focus ?? null
  }

  function wake(keys: readonly string[]): void {
    for (const key of keys) {
      const pane = panes.value.find((p) => p.key === key)
      if (pane?.state === 'parked') pane.state = 'starting'
    }
  }

  function dismissLast(): void {
    last.value = null
    writeLocal(LAST_KEY, 'null')
  }

  function update(key: string, patch: Partial<Pane>): void {
    const pane = panes.value.find((p) => p.key === key)
    if (pane) Object.assign(pane, patch)
  }

  function remove(key: string): void {
    const i = panes.value.findIndex((p) => p.key === key)
    if (i < 0) return
    panes.value.splice(i, 1)
    delete lastSeen.value[key]
    const t =tabs.value.findIndex((tab) => leaves(tab.root).includes(key))
    const tab = tabs.value[t]
    if (!tab) return
    unzoom(tab)
    const root = removeLeaf(tab.root, key)
    if (root) {
      tab.root = root
      settle(tab)
      if (tab.focus === key) tab.focus = leaves(tab.root)[0]!
      if (activeKey.value === key) activeKey.value = tab.focus
      return
    }
    tabs.value.splice(t, 1)
    if (activeKey.value === key) activeKey.value = tabs.value[Math.min(t, tabs.value.length - 1)]?.focus ?? null
  }

  function closeTab(tabKey: string): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (tab) for (const key of leaves(tab.root)) remove(key)
  }

  function activate(key: string): void {
    const tab = tabOf(key)
    if (!tab) return
    if (tab.zoom !== key) unzoom(tab)
    const folded = groupNodes(tab.root).find((g) => g.folded && leaves(g.node).includes(key))
    if (folded) tab.root = setGroupLayoutMeta(tab.root, folded.group, { folded: false })
    tab.focus = key
    activeKey.value = key
    const pane = panes.value.find((p) => p.key === key)
    if (pane) pane.focusedAt = Date.now()
    if (pane?.attention) pane.attention = undefined
  }

  /** Waiting before done, oldest tab first. */
  function nextAttention(): Pane | null {
    const order = tabs.value.flatMap((t) => leaves(t.root))
    const pick = (kind: 'waiting' | 'done'): Pane | undefined =>
      order.map((k) => panes.value.find((p) => p.key === k)).find((p) => p?.attention === kind && p.key !== activeKey.value)
    const next = pick('waiting') ?? pick('done') ?? null
    if (next) activate(next.key)
    return next
  }

  /** Closes right away unless a pane still runs something worth asking about (`busy`). */
  function requestClose(keys: string[], busy: (pane: Pane) => boolean, opts: { record?: boolean } = {}): void {
    const list = keys.map((k) => panes.value.find((p) => p.key === k)).filter((p): p is Pane => !!p)
    if (list.some(busy)) {
      pendingClose.value = list.map((p) => p.key)
      pendingQuiet = opts.record === false
      return
    }
    if (opts.record !== false) recordClosed(list.map((p) => p.key))
    for (const p of list) remove(p.key)
  }

  function confirmClose(): void {
    const keys = pendingClose.value ?? []
    pendingClose.value = null
    if (!pendingQuiet) recordClosed(keys)
    pendingQuiet = false
    for (const k of keys) remove(k)
  }

  function cancelClose(): void {
    pendingClose.value = null
    pendingQuiet = false
  }

  const closedStack = shallowRef<ClosedEntry[]>([])
  let closedSeq = 0
  let closedNamer: ((pane: Pane) => string) | null = null

  function setClosedNamer(nameOf: ((pane: Pane) => string) | null): void {
    closedNamer = nameOf
  }

  function nameFor(pane: Pane, nameOf?: (pane: Pane) => string): string {
    try {
      return (nameOf ?? closedNamer ?? fallbackName)(pane) || fallbackName(pane)
    } catch {
      return fallbackName(pane)
    }
  }

  function recordClosed(keys: readonly string[], nameOf?: (pane: Pane) => string): ClosedEntry | null {
    const gone = new Set(keys.filter((k) => paneOf(k)))
    if (!gone.size) return null
    const entry: ClosedEntry = { id: ++closedSeq, at: Date.now(), tabs: [], loose: [] }
    const snap = (k: string): ClosedPane => {
      const pane = paneOf(k)!
      return closedPane(pane, nameFor(pane, nameOf))
    }
    tabs.value.forEach((tab, index) => {
      const all = leaves(tab.root)
      const mine = all.filter((k) => gone.has(k))
      if (!mine.length) return
      if (mine.length === all.length) {
        entry.tabs.push({ root: copyTree(tab.root), focus: tab.focus, name: tab.name, project: tab.project, index, panes: all.map(snap) })
        return
      }
      let root: LayoutNode | null = copyTree(tab.root)
      for (const k of mine) {
        entry.loose.push({ pane: snap(k), anchor: root && anchorOf(root, k), project: tab.project })
        root = root && removeLeaf(root, k)
      }
    })
    if (!entry.tabs.length && !entry.loose.length) return null
    closedStack.value = [...closedStack.value, entry].slice(-CLOSED_MAX)
    return entry
  }

  function placeBack(paneKey: string, anchor: ClosedAnchor, map: ReadonlyMap<string, string>): boolean {
    const group = anchor.group.map((k) => map.get(k) ?? k)
    const sibling = map.get(anchor.siblingKey) ?? anchor.siblingKey
    const host = tabOf(sibling) ?? group.map((k) => tabOf(k)).find((t): t is Tab => !!t)
    if (!host) return false
    const leaf: LayoutNode = { pane: paneKey }
    const wrap = (n: LayoutNode): LayoutNode => ({ dir: anchor.dir, ratio: anchor.ratio, a: anchor.side === 'a' ? leaf : n, b: anchor.side === 'a' ? n : leaf })
    const exact = wrapGroup(host.root, new Set(group), wrap)
    if (exact) host.root = exact
    else {
      const at = [sibling, ...group].find((k) => tabOf(k) === host)!
      const edge: Edge = anchor.dir === 'row' ? (anchor.side === 'a' ? 'left' : 'right') : anchor.side === 'a' ? 'top' : 'bottom'
      host.root = dockTree(host.root, at, leaf, edge)
    }
    host.focus = paneKey
    unzoom(host)
    return true
  }

  function reopenClosed(): { keys: string[]; from: string[] } | null {
    const entry = closedStack.value.at(-1)
    if (!entry) return null
    closedStack.value = closedStack.value.slice(0, -1)
    const map = new Map<string, string>()
    const from: string[] = []
    const make = (c: ClosedPane): Pane | null => {
      if (c.resume && panes.value.some((p) => p.agentSessionId === c.resume && (p.state === 'running' || p.state === 'starting'))) return null
      const pane = isTerm(c) ? addPane(c.preset, c.cwd, c.resume, c.openedBy ? { openedBy: c.openedBy } : {}) : addView(viewOf(c), c.cwd)
      if (c.project) pane.project = c.project
      map.set(c.key, pane.key)
      from.push(c.key)
      return pane
    }
    let focus: string | undefined
    for (const tab of entry.tabs) {
      for (const c of tab.panes) make(c)
      let root: LayoutNode | null = tab.root
      for (const k of leaves(tab.root)) if (!map.has(k)) root = root && removeLeaf(root, k)
      if (!root) continue
      const mapped = remapTree(root, map)
      const first = map.get(tab.focus) ?? leaves(mapped)[0]!
      const at = Math.min(tab.index, tabs.value.length)
      tabs.value.splice(at, 0, { key: newKey('t'), root: mapped, focus: first, project: tab.project, name: tab.name })
      settle(tabs.value[at]!)
      focus ??= first
    }
    for (const item of [...entry.loose].reverse()) {
      const pane = make(item.pane)
      if (!pane) continue
      if (!item.anchor || !placeBack(pane.key, item.anchor, map)) {
        tabs.value.push({ key: newKey('t'), root: { pane: pane.key }, focus: pane.key, project: pane.project ?? item.project ?? pane.cwd })
        pane.project = undefined
      } else {
        const host = tabOf(pane.key)!
        const want = pane.project ?? item.project ?? ''
        const now = groupOf(host.root, pane.key)
        if (now !== undefined && !sameProject(now, want)) {
          const near = leaves(host.root).find((k) => k !== pane.key)!
          host.root = removeLeaf(host.root, pane.key)!
          attach(host, { pane: pane.key }, want, near, 'right')
        } else {
          pane.project = foreignProject(host, want || undefined, paneOf)
          settle(host)
        }
      }
      focus = pane.key
    }
    for (const p of panes.value) if (p.openedBy && map.has(p.openedBy)) p.openedBy = map.get(p.openedBy)
    if (focus) activate(focus)
    return { keys: from.map((k) => map.get(k)!), from }
  }

  function toggleZoom(paneKey: string): boolean {
    const tab = tabOf(paneKey)
    if (!tab) return false
    if (tab.zoom === paneKey || leaves(tab.root).length < 2) {
      unzoom(tab)
      return false
    }
    activate(paneKey)
    tab.zoom = paneKey
    return true
  }

  function clearZoom(tabKey: string): void {
    unzoom(tabs.value.find((t) => t.key === tabKey))
  }

  function setDirty(key: string, dirty: boolean): void {
    const pane = panes.value.find((p) => p.key === key)
    if (!pane || !!pane.dirty === dirty) return
    pane.dirty = dirty || undefined
  }

  /** Each mounted editor registers how to save its buffer; the close sheet and quit call them. Not reactive. */
  const savers = new Map<string, () => Promise<SaveOutcome>>()
  function registerSaver(key: string, save: (() => Promise<SaveOutcome>) | null): void {
    if (save) savers.set(key, save)
    else savers.delete(key)
  }
  const dirtyPanes = computed(() => panes.value.filter((p) => p.kind === 'file' && p.dirty))

  /** Saves the dirty panes among `keys` one by one; failures name the pane and the error key. */
  async function saveFiles(keys: readonly string[]): Promise<Array<{ key: string; error: string }>> {
    const failed: Array<{ key: string; error: string }> = []
    for (const key of keys) {
      const pane = panes.value.find((p) => p.key === key)
      if (!pane?.dirty) continue
      const save = savers.get(key)
      const res = save ? await save().catch((): SaveOutcome => ({ ok: false, error: 'failed' })) : { ok: false as const, error: 'unavailable' }
      if (!res.ok) failed.push({ key, error: res.error })
    }
    return failed
  }

  function activateTab(tabKey: string): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (tab) activeKey.value = tab.focus
  }

  /** Next / previous tab; `scope` (tab keys in display order) limits it to the tabs on screen. */
  function cycleTab(step: number, scope?: readonly string[]): void {
    const keys = scope?.length ? scope : tabs.value.map((t) => t.key)
    const n = keys.length
    if (!n) return
    const at = keys.indexOf(activeTab.value?.key ?? '')
    activateTab(keys[at < 0 ? (step > 0 ? 0 : n - 1) : (((at + step) % n) + n) % n]!)
  }

  function moveFocus(dir: FocusDir): void {
    const tab = activeTab.value
    if (!tab || !activeKey.value) return
    const next = neighbor(tab.root, activeKey.value, dir)
    if (next) activate(next)
  }

  /** Pulls a pane out of its tab (a tab left empty goes away); false when it was not in one. */
  function unplace(key: string): boolean {
    const i = tabs.value.findIndex((t) => leaves(t.root).includes(key))
    const tab = tabs.value[i]
    if (!tab) return false
    unzoom(tab)
    const root = removeLeaf(tab.root, key)
    if (!root) tabs.value.splice(i, 1)
    else {
      tab.root = root
      if (tab.focus === key) tab.focus = leaves(root)[0]!
      settle(tab)
    }
    return true
  }

  const homeOf = (tab: Tab): string => tabProject(tab, paneOf) ?? ''

  function settle(tab: Tab): void {
    const home = homeOf(tab)
    const res = settleGroups(tab.root, (k) => paneOf(k)?.project ?? home, sameProject)
    if (res.root !== tab.root) tab.root = res.root
    const groups = groupsOf(tab.root)
    if (res.project !== undefined) {
      if (res.project && !sameProject(res.project, home)) tab.project = res.project
    } else if (!groups.some((g) => sameProject(g.project, home)) && groups[0]?.project) tab.project = groups[0].project
    const own = tab.project ?? home
    const want = (project: string | undefined): string | undefined => (project && !sameProject(project, own) ? project : undefined)
    const parts = groups.length ? groups : [{ project: undefined, keys: leaves(tab.root) }]
    for (const g of parts)
      for (const k of g.keys) {
        const p = paneOf(k)
        const v = want(g.project)
        if (p && p.project !== v) p.project = v
      }
    if (tab.zoom && !leaves(tab.root).includes(tab.zoom)) tab.zoom = undefined
  }

  const fitsGroup = (group: string, dir: string | undefined): boolean => (group ? !!dir && within(dir, group) : !dir)

  function dockNear(tab: Tab, key: string, prefer: string | undefined, project: string | undefined): boolean {
    const size = canvasSize.value
    const groups = groupsOf(tab.root)
    if (!groups.length) {
      const home = homeOf(tab)
      if (project && !(home && within(project, home))) return false
      const spot = dockSpot(tab.root, prefer ?? tab.focus, size)
      if (!spot) return false
      tab.root = splitLeaf(tab.root, spot.key, key, spot.dir)
      settle(tab)
      return true
    }
    const pick = project !== undefined ? groups.find((g) => fitsGroup(g.project, project)) : (groups.find((g) => !!prefer && g.keys.includes(prefer)) ?? groups.find((g) => g.keys.includes(tab.focus)) ?? groups[0])
    if (!pick) return false
    const node = findGroup(tab.root, pick.project)!.node
    const box = groupBoxes(tab.root).get(pick.project)!
    const room = { w: Math.max(0, size.w * box.w - 2 * GROUP_CHROME.pad), h: Math.max(0, size.h * box.h - GROUP_CHROME.head - GROUP_CHROME.pad) }
    const spot = dockSpot(node, prefer && pick.keys.includes(prefer) ? prefer : undefined, size.w && size.h ? room : size)
    if (!spot) return false
    tab.root = splitLeaf(tab.root, spot.key, key, spot.dir)
    settle(tab)
    return true
  }

  function attach(tab: Tab, sub: LayoutNode, project: string, near: string, edge: Edge): void {
    const home = homeOf(tab)
    const at = groupOf(tab.root, near) ?? home
    const flat = ungrouped(sub)
    if (fitsGroup(at, project || undefined)) {
      const v = at && !sameProject(at, tab.project ?? home) ? at : undefined
      for (const k of leaves(flat)) {
        const p = paneOf(k)
        if (p) p.project = v
      }
      tab.root = dockTree(tab.root, near, flat, edge)
    } else {
      const own = groupsOf(tab.root).find((g) => fitsGroup(g.project, project || undefined))?.project ?? (home && project && within(project, home) ? home : undefined)
      tab.root = dockGroup(tab.root, home, near, own ?? project, flat, edge, sameProject)
    }
    settle(tab)
  }

  /** False when `src` would leave its project group for another one it cannot join. */
  function canDock(src: string, target: string): boolean {
    return src !== target && !!tabOf(src) && !!tabOf(target)
  }

  function setGroupMeta(tabKey: string, project: string, patch: GroupMeta): boolean {
    const tab = tabs.value.find((t) => t.key === tabKey)
    const list = tab ? groupsOf(tab.root) : []
    const own = list.find((g) => g.project === project)
    if (!tab || !own) return false
    if (patch.folded && list.filter((g) => !g.folded && g.project !== project).length === 0) return false
    tab.root = setGroupLayoutMeta(tab.root, project, patch)
    if (patch.folded) {
      unzoom(tab)
      const shown = groupsOf(tab.root).filter((g) => !g.folded)
      if (own.keys.includes(tab.focus) || (activeKey.value && own.keys.includes(activeKey.value))) {
        const next = lastFocused(shown.flatMap((g) => g.keys))
        if (next) {
          if (activeTab.value === tab) activate(next)
          else tab.focus = next
        }
      }
    }
    return true
  }

  const lastFocused = (keys: readonly string[]): string | undefined =>
    [...keys].sort((a, b) => (paneOf(b)?.focusedAt ?? 0) - (paneOf(a)?.focusedAt ?? 0))[0]

  function focusGroup(dir: 'left' | 'right'): boolean {
    const tab = activeTab.value
    const key = activeKey.value
    const from = tab && key ? groupOf(tab.root, key) : undefined
    const to = tab && from !== undefined ? neighborGroup(tab.root, from, dir) : null
    if (!tab || to === null) return false
    const keys = groupsOf(tab.root).find((g) => g.project === to)?.keys ?? []
    const next = lastFocused(keys)
    if (next) activate(next)
    return !!next
  }

  function shiftGroup(dir: 'left' | 'right'): boolean {
    const tab = activeTab.value
    const key = activeKey.value
    const from = tab && key ? groupOf(tab.root, key) : undefined
    const to = tab && from !== undefined ? neighborGroup(tab.root, from, dir) : null
    if (!tab || from === undefined || to === null) return false
    moveGroup(tab.key, from, to, dir)
    return true
  }

  /** Docks `src` onto `edge` of `target`, from the same tab or from another one. */
  function movePane(src: string, target: string, edge: Edge): void {
    const to = tabOf(target)
    const from = tabOf(src)
    if (!to || !from || src === target || !canDock(src, target)) return
    unzoom(from)
    unzoom(to)
    if (from === to) {
      const a = groupOf(to.root, src)
      const b = groupOf(to.root, target)
      if (a === b) to.root = moveLeaf(to.root, src, target, edge)
      else if (groupsOf(to.root).find((g) => g.project === a)?.keys.length === 1) to.root = moveGroupNode(to.root, a!, b!, edge)
      else {
        to.root = removeLeaf(to.root, src)!
        attach(to, { pane: src }, a ?? '', target, edge)
      }
      settle(to)
    } else {
      const pane = paneOf(src)
      const project = pane ? (paneProjectOf(from, pane, paneOf) ?? '') : ''
      unplace(src)
      attach(to, { pane: src }, project, target, edge)
    }
    activate(src)
  }

  /** Two panes of one tab trade places. */
  function swapPanes(a: string, b: string): void {
    const tab = tabOf(a)
    if (!tab || tab !== tabOf(b) || groupOf(tab.root, a) !== groupOf(tab.root, b)) return
    unzoom(tab)
    tab.root = swapLeaves(tab.root, a, b)
    activate(a)
  }

  /** Every pane of tab `tabKey` docks as one block onto `edge` of `target`; the target tab keeps its project. */
  function mergeTab(tabKey: string, target: string, edge: Edge): void {
    const i = tabs.value.findIndex((t) => t.key === tabKey)
    const src = tabs.value[i]
    const to = tabOf(target)
    if (!src || !to || src === to) return
    unzoom(to)
    const nodes = groupNodes(src.root)
    const parts = nodes.length ? nodes.map((g) => ({ project: g.group, node: g.node })) : [{ project: homeOf(src), node: src.root }]
    tabs.value.splice(i, 1)
    for (const p of edge === 'right' || edge === 'bottom' ? [...parts].reverse() : parts) attach(to, p.node, p.project, target, edge)
    activate(src.focus)
  }

  /** A project group leaves its tab for a tab of its own, right after it. */
  function groupToTab(tabKey: string, project: string): string | null {
    const tab = tabs.value.find((t) => t.key === tabKey)
    const took = tab ? takeGroup(tab.root, project) : null
    if (!tab || !took?.rest) return null
    unzoom(tab)
    const keys = leaves(took.node)
    tab.root = took.rest
    if (keys.includes(tab.focus)) tab.focus = leaves(took.rest)[0]!
    for (const k of keys) {
      const p = paneOf(k)
      if (p) p.project = undefined
    }
    settle(tab)
    const focus = activeKey.value && keys.includes(activeKey.value) ? activeKey.value : keys[0]!
    const made: Tab = { key: newKey('t'), root: took.node, focus, project: project || undefined }
    tabs.value.splice(tabs.value.indexOf(tab) + 1, 0, made)
    activate(focus)
    return made.key
  }

  /** Group `project` docks onto `edge` of group `near` in the same tab. */
  function moveGroup(tabKey: string, project: string, near: string, edge: Edge): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (!tab) return
    unzoom(tab)
    tab.root = moveGroupNode(tab.root, project, near, edge)
    settle(tab)
  }

  function groupKeys(tabKey: string, project: string): string[] {
    const tab = tabs.value.find((t) => t.key === tabKey)
    return (tab && groupsOf(tab.root).find((g) => g.project === project)?.keys) ?? []
  }

  /** A pane of a split tab gets a tab of its own, right after the one it left. */
  function detachPane(key: string): void {
    const from = tabOf(key)
    const pane = panes.value.find((p) => p.key === key)
    if (!from || !pane || leaves(from.root).length < 2) return
    const project = pane.project ?? from.project ?? pane.cwd
    unplace(key)
    pane.project = undefined
    const at = tabs.value.indexOf(from)
    tabs.value.splice(at + 1, 0, { key: newKey('t'), root: { pane: key }, focus: key, project })
    activate(key)
  }

  function renameTab(tabKey: string, name: string): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (tab) tab.name = cleanTabName(name)
  }

  function moveTab(tabKey: string, to: number): void {
    const from = tabs.value.findIndex((t) => t.key === tabKey)
    if (from < 0) return
    const at = Math.max(0, Math.min(to, tabs.value.length - 1))
    if (at === from) return
    const [tab] = tabs.value.splice(from, 1)
    tabs.value.splice(at, 0, tab!)
  }

  function setFolded(tabKey: string, folded: boolean): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (tab) tab.folded = folded || undefined
  }

  function ungroup(tabKey: string): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (!tab) return
    unzoom(tab)
    const keep = activeKey.value
    for (const key of leaves(tab.root).slice(1).reverse()) detachPane(key)
    if (keep) activate(keep)
  }

  function movePaneToTab(paneKey: string, tabKey: string): void {
    const target = tabs.value.find((t) => t.key === tabKey)
    if (!target || tabOf(paneKey) === target) return
    const spot = dockSpot(target.root, target.focus, canvasSize.value)
    movePane(paneKey, spot?.key ?? target.focus, spot?.dir === 'col' ? 'bottom' : 'right')
  }

  function groupPanes(keys: readonly string[]): string | null {
    const live = [...new Set(keys)].filter((k) => !!tabOf(k))
    const counts = new Map<Tab, number>()
    for (const k of live) counts.set(tabOf(k)!, (counts.get(tabOf(k)!) ?? 0) + 1)
    if (live.length < 2 || counts.size < 2) return null
    let to = tabOf(live[0]!)!
    for (const [tab, n] of counts) if (n > counts.get(to)!) to = tab
    for (const k of live) if (tabOf(k) !== to) movePaneToTab(k, to.key)
    activate(live[0]!)
    return to.key
  }

  function ungroupPanes(keys: readonly string[]): number {
    const keep = activeKey.value
    let n = 0
    for (const k of [...new Set(keys)].reverse()) {
      const tab = tabOf(k)
      if (!tab || leaves(tab.root).length < 2) continue
      detachPane(k)
      n++
    }
    if (keep && tabOf(keep)) activate(keep)
    return n
  }

  function bringProject(tabKeys: readonly string[], dir: string | null, target: string, edge: Edge): 'merged' | 'opened' | null {
    const to = tabOf(target)
    if (!to) return null
    const own = tabKeys.filter((k) => k !== to.key && tabs.value.some((t) => t.key === k))
    if (own.length) {
      for (const k of [...own].reverse()) mergeTab(k, target, edge)
      return 'merged'
    }
    if (!dir) return null
    const kind = preset.value
    if (!presetEnabled(kind, hasDeepSeek) || useDeepSeekStore().gate(kind)) return null
    const pane = addPane(kind, dir)
    remember(dir)
    unzoom(to)
    attach(to, { pane: pane.key }, dir, target, edge)
    to.focus = pane.key
    activate(pane.key)
    return 'opened'
  }

  /** The focused pane trades places with its neighbour on that side and keeps focus. */
  function swapWith(dir: FocusDir): void {
    const tab = activeTab.value
    const key = activeKey.value
    const next = tab && key ? neighbor(tab.root, key, dir) : null
    if (key && next) swapPanes(key, next)
  }

  function setRatio(tabKey: string, path: string, ratio: number): void {
    const tab = tabs.value.find((t) => t.key === tabKey)
    if (tab) tab.root = setTreeRatio(tab.root, path, ratio)
  }

  function remember(dir: string): void {
    const key = dirKey(dir)
    recentDirs.value = [dir, ...recentDirs.value.filter((d) => dirKey(d) !== key)].slice(0, RECENT_LIMIT)
    writeLocal(RECENT_KEY, JSON.stringify(recentDirs.value))
  }

  function forget(dir: string): void {
    const key = dirKey(dir)
    recentDirs.value = recentDirs.value.filter((d) => dirKey(d) !== key)
    writeLocal(RECENT_KEY, JSON.stringify(recentDirs.value))
  }

  for (const tab of tabs.value) settle(tab)

  return {
    persistNow,
    panes,
    tabs,
    canvasSize,
    activeKey,
    active,
    activeTab,
    paneByTerminal,
    recentDirs,
    preset,
    last,
    attentionPanes,
    pendingClose,
    tabOf,
    open,
    launch,
    openFromAgent,
    openSession,
    sessionLive,
    showForAgent,
    handBack,
    sessionPanes,
    resolveTarget,
    sendPrompt,
    typeKeys,
    telegramPanes,
    telegramInbound,
    telegramKeys,
    telegramTyped,
    openBrowser,
    openFile,
    openChanges,
    openLog,
    openRun,
    insertRequest,
    insertInto,
    insertImage,
    restartRequest,
    requestRestart,
    searchRequest,
    requestSearch,
    resolveAgentPane,
    nextAttention,
    requestClose,
    confirmClose,
    cancelClose,
    closedStack,
    setClosedNamer,
    recordClosed,
    reopenClosed,
    toggleZoom,
    clearZoom,
    setDirty,
    registerSaver,
    dirtyPanes,
    saveFiles,
    resumeSession,
    restoreLast,
    lastMovedDirs,
    lastProjects,
    wake,
    dismissLast,
    runScript,
    openLayout,
    lastSeen,
    touch,
    split,
    update,
    remove,
    closeTab,
    activate,
    activateTab,
    cycleTab,
    moveFocus,
    movePane,
    swapPanes,
    swapWith,
    mergeTab,
    detachPane,
    renameTab,
    moveTab,
    setFolded,
    ungroup,
    movePaneToTab,
    groupPanes,
    ungroupPanes,
    bringProject,
    canDock,
    setGroupMeta,
    focusGroup,
    shiftGroup,
    groupToTab,
    moveGroup,
    groupKeys,
    setRatio,
    remember,
    forget,
  }
})
