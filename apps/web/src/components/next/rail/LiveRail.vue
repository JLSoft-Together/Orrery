<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { waitReason, type PtyPreset } from '@ccm/shared'
import { ChevronDown, ChevronRight, Ellipsis, LoaderCircle, Plus } from 'lucide-vue-next'
import { useClusterDrag } from '../../../composables/useClusterDrag'
import { useOpenProject } from '../../../composables/useOpenProject'
import { useCompact } from '../../../composables/useCompact'
import { useProjectDisks } from '../../../composables/useDiskSizes'
import { usePaneActions } from '../../../composables/usePaneActions'
import { usePaneInfo } from '../../../composables/usePaneInfo'
import { useRail } from '../../../composables/useRail'
import { useRailDrag, type RailDrop } from '../../../composables/useRailDrag'
import { useTurnChanges } from '../../../composables/useTurnChanges'
import { hasBuildClean } from '../../../lib/buildClean'
import { hasCtxSources } from '../../../lib/ctxSources'
import { hasDeepSeek } from '../../../lib/desktop'
import type { MenuEntry } from '../../../lib/menu'
import { othersOf, relPath, summarize } from '../../../lib/clash'
import { buildLiveRows, collapsedProjects, freezeLayout, layoutOf, type LayoutSection, type LiveExtras, type LiveRow, type RowClash } from '../../../lib/next/railLive'
import { forgetSnoozed, isSnoozed } from '../../../lib/next/railSnooze'
import { NEXT_SHELL_KEY } from '../../../lib/next/shellContext'
import { leftRail, workspaceBridge, type DragHandoff } from '../../../lib/next/workspaceBridge'
import { hasLayouts, hasScripts, listLayouts, type SavedLayout } from '../../../lib/projects'
import { hasPermAdvisor } from '../../../lib/permAdvisor'
import { isMac } from '../../../lib/platform'
import { hasChanges, hasEnvCheck, hasExplorer, hasI18nScan, hasLog, hasPreview, hasReveal, pickFile, revealPath } from '../../../lib/preview'
import { contextPctOf } from '../../../lib/provider'
import { monogram, projectTone } from '../../../lib/rail/projectRow'
import type { RailDragSource } from '../../../lib/rail/railDrag'
import { isMenuKey, projectMenu as buildProjectMenu, rowMenu } from '../../../lib/rail/railMenu'
import { folderName } from '../../../lib/sidebarRows'
import { rowLines, type RailCluster, type RailRow } from '../../../lib/rail/railModel'
import { clickSelect, extendSelect, NO_SELECTION, pruneSelection, selectionFacts, selectionMenu, selectionPanes, type RailSelection } from '../../../lib/rail/railSelect'
import { railMoveIndexes } from '../../../lib/rail/railView'
import { leaves, type SplitDir } from '../../../lib/splitLayout'
import { waitDisplay } from '../../../lib/wait'
import { useAgentsStore } from '../../../stores/agents'
import { useAgentTouchesStore } from '../../../stores/agentTouches'
import { useRunStore } from '../../../stores/run'
import { useTurnEditsStore } from '../../../stores/turnEdits'
import { useUiStore } from '../../../stores/ui'
import { useUiGenStore } from '../../../stores/uiGen'
import { isTerm, presetOf, useWorkspaceStore, workCwd, type SplitChoice } from '../../../stores/workspace'
import ContextMenu from '../../ui/ContextMenu.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'
import RailCollapsed from './RailCollapsed.vue'
import RailRowItem from './RailRowItem.vue'

const props = defineProps<{ collapsed?: boolean }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()
const ui = useUiStore()
const uigen = useUiGenStore()
const { available: openable, picking, openProject } = useOpenProject()
const info = usePaneInfo()
const agents = useAgentsStore()
const touches = useAgentTouchesStore()
const run = useRunStore()
const rail = useRail()
const paneActions = usePaneActions()
const compactFlow = useCompact()
const turnEdits = useTurnEditsStore()
const turnChanges = useTurnChanges()
const shell = inject(NEXT_SHELL_KEY, null)

function rowTitle(row: RailRow): string {
  const pane = row.pane
  const first = rowLines(row, '').first
  const dir = workCwd(pane)
  if (!dir || !isTerm(pane) || (row.task ?? row.agent) !== folderName(dir)) return first
  return [t(`workspace.preset.${presetOf(pane)}`), row.twin].filter(Boolean).join(' ')
}

function extrasOf(row: RailRow): LiveExtras {
  const pane = row.pane
  const session = info.sessionOf(pane)
  const id = pane.terminalId
  const agent = id && session ? agents.mainOf(id) : undefined
  const pct = agent ? contextPctOf(agent) : undefined
  const display = session?.status === 'waiting' ? waitDisplay(session.waitingFor, agent?.currentTool, (k, p) => t(k, p ?? {})) : null
  return {
    model: isTerm(pane) ? info.modelOf(id) : null,
    ctxPct: pct ?? null,
    tool: session?.status === 'working' ? (agent?.currentTool ?? null) : null,
    waitReason: session?.status === 'waiting' ? waitReason(session.waitingFor) : null,
    waitText: display?.text ?? null,
    stateText: isTerm(pane) ? info.stateText(pane) : null,
    title: rowTitle(row),
    clash: id ? clashOf(id, workCwd(pane)) : null,
  }
}

function clashOf(id: string, cwd: string | undefined): RowClash | null {
  const list = touches.clashesOf.get(id)
  if (!list?.length) return null
  const lines = list.map((c) => t('v104.clash.tip', { peer: othersOf(c, id).map((p) => p.name).join(', '), file: relPath(c.path, cwd) }))
  const { shown, more } = summarize(lines)
  return { n: list.length, tip: [...shown, ...(more ? [t('v104.clash.more', { n: more })] : [])].join('\n') }
}

const rows = computed(() => buildLiveRows(rail.clusters.value, extrasOf, (c) => projectTone(c.key)))
const rowByKey = computed(() => new Map(rows.value.map((r) => [r.key, r] as const)))
const clusterOrder = computed(() => rail.clusters.value.map((c) => c.key))
const clusterByKey = computed(() => new Map(rail.clusters.value.map((c) => [c.key, c] as const)))
const mode = computed(() => uigen.railGroup)
const freshBody = computed(() => layoutOf(rows.value, mode.value, clusterOrder.value))

const hold = ref(false)
const body = ref<LayoutSection[]>([])
let lastMode = mode.value

function sync(force = false): void {
  const reset = force || lastMode !== mode.value
  lastMode = mode.value
  body.value = freezeLayout(reset ? null : body.value, freshBody.value, hold.value)
}
watch([freshBody, hold, mode], () => sync(), { immediate: true })

const sectionRows = (s: LayoutSection): LiveRow[] => s.keys.map((k) => rowByKey.value.get(k)).filter((r): r is LiveRow => !!r)
const collapsedList = computed(() => collapsedProjects(rows.value, clusterOrder.value, isSnoozed))

watch(rows, (list) => forgetSnoozed((k) => list.some((r) => r.key === k)))

const folded = ref<Set<string>>(new Set())
function toggleFold(key: string): void {
  const next = new Set(folded.value)
  if (!next.delete(key)) next.add(key)
  folded.value = next
}

const navEl = ref<HTMLElement | null>(null)
const listEl = ref<HTMLElement | null>(null)
const announce = ref('')
const flash = (text: string): void => {
  announce.value = text
}

function activateRow(row: LiveRow): void {
  workspace.activate(row.paneKey)
  ui.setView('workspace')
  shell?.closeRail()
}

const selection = ref<RailSelection>(NO_SELECTION)
const selectedSet = computed(() => new Set(selection.value.keys))
watch(rowByKey, (map) => {
  selection.value = pruneSelection(selection.value, (k) => map.has(k))
})

const visibleRowKeys = (): string[] =>
  [...(listEl.value?.querySelectorAll<HTMLElement>('[data-side-row]') ?? [])].filter((n) => n.offsetParent && !n.closest('[inert]')).flatMap((n) => (n.dataset.rowKey ? [n.dataset.rowKey] : []))

function onClickCapture(e: MouseEvent): void {
  const el = (e.target as HTMLElement).closest<HTMLElement>('[data-side-row]')
  const key = el?.dataset.rowKey
  if (!el || !key) return
  const toggle = isMac ? e.metaKey : e.ctrlKey
  if (!toggle && !e.shiftKey) {
    if (selection.value.keys.length) selection.value = NO_SELECTION
    return
  }
  e.preventDefault()
  e.stopPropagation()
  window.getSelection()?.removeAllRanges()
  const focused = rows.value.find((r) => r.focused)?.key ?? null
  selection.value = clickSelect(selection.value, key, visibleRowKeys(), { toggle, range: e.shiftKey }, focused)
}

const tabStopKey = computed(() => {
  const keys = body.value.flatMap((s) => (folded.value.has(s.id.replace(/^p:/, '')) ? [] : s.keys))
  return keys.find((k) => rowByKey.value.get(k)?.focused) ?? keys[0] ?? null
})

const renaming = ref<string | null>(null)
let keyMenuAt = 0

function onRowKey(row: LiveRow, e: KeyboardEvent): void {
  if (isMenuKey(e)) {
    e.preventDefault()
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    keyMenuAt = performance.now()
    showRowMenu(row, new DOMRect(r.left + 24, r.bottom, 0, 0), e.currentTarget as HTMLElement)
    return
  }
  if (e.key === 'F2' && !e.shiftKey && !e.altKey && !e.ctrlKey) {
    e.preventDefault()
    renaming.value = row.tabKey
    return
  }
  if (e.altKey || e.ctrlKey || e.metaKey) return
  const items = [...(listEl.value?.querySelectorAll<HTMLElement>('[data-rail-open]') ?? [])].filter((n) => n.offsetParent && !n.closest('[inert]'))
  const at = items.indexOf(e.currentTarget as HTMLElement)
  let next: HTMLElement | undefined
  if (e.key === 'ArrowDown') next = items[(at + 1) % items.length]
  else if (e.key === 'ArrowUp') next = items[(at - 1 + items.length) % items.length]
  else if (e.key === 'Home') next = items[0]
  else if (e.key === 'End') next = items.at(-1)
  else return
  e.preventDefault()
  const key = next?.closest<HTMLElement>('[data-row-key]')?.dataset.rowKey
  if (e.shiftKey && key) selection.value = extendSelect(selection.value, row.key, key, visibleRowKeys())
  next?.focus()
}

async function onRenamed(tabKey: string, saved: boolean): Promise<void> {
  const was = !!navEl.value?.contains(document.activeElement)
  renaming.value = null
  if (saved) {
    const tab = workspace.tabs.find((x) => x.key === tabKey)
    if (tab) flash(t(tab.name ? 'workspace.tabName.renamed' : 'workspace.tabName.reset', { name: rail.tabTitleOf(tabKey) }))
  }
  await nextTick()
  if (was) navEl.value?.querySelector<HTMLElement>(`[data-tab-key="${CSS.escape(tabKey)}"] [data-rail-open]`)?.focus()
}

const closeKeys = (keys: string[]): void => workspace.requestClose(keys, info.isBusy)
const tabLeaves = (tabKey: string): string[] => {
  const tab = workspace.tabs.find((x) => x.key === tabKey)
  return tab ? leaves(tab.root) : []
}

function closeRowTab(row: LiveRow): void {
  if (row.key.startsWith('tab:')) closeKeys(tabLeaves(row.tabKey))
  else closeKeys([row.paneKey])
}

function movePane(paneKey: string, name: string, tabKey: string): void {
  const title = rail.tabTitleOf(tabKey)
  workspace.movePaneToTab(paneKey, tabKey)
  flash(t('workspace.paneDrop.moved', { pane: name, tab: title }))
}

function mergeTab(tabKey: string, targetKey: string): void {
  const target = workspace.tabs.find((x) => x.key === targetKey)
  if (!target) return
  const name = rail.tabTitleOf(tabKey)
  const title = rail.tabTitleOf(targetKey)
  workspace.mergeTab(tabKey, target.focus, 'right')
  flash(t('workspace.paneDrop.merged', { tab: name, target: title }))
}

function onRailDrop(src: RailDragSource, drop: RailDrop): void {
  if (drop.type === 'detach') {
    if (src.paneKey) {
      workspace.detachPane(src.paneKey)
      flash(t('workspace.paneDrop.detached', { pane: src.title }))
    }
    return
  }
  if (src.kind === 'tab') mergeTab(src.tabKey, drop.tabKey)
  else if (src.paneKey) movePane(src.paneKey, src.title, drop.tabKey)
}

const railDrag = useRailDrag(
  navEl,
  {
    merge: (tabKey) => t('workspace.paneDrop.merge', { name: rail.tabTitleOf(tabKey) }),
    detach: () => t('workspace.paneDrop.detach'),
    blocked: () => t('workspace.paneDrop.otherProject'),
  },
  onRailDrop,
)

const shownProjects = computed(() => body.value.filter((s) => s.id.startsWith('p:')).map((s) => s.id.slice(2)))
const clusterDrag = useClusterDrag(listEl, (from, to) => {
  const at = railMoveIndexes(shownProjects.value, clusterOrder.value, from, to)
  if (at) rail.move(at[0], at[1])
})

let handoff: { down: PointerEvent; target: DragHandoff; el: HTMLElement } | null = null

function endHandoff(): void {
  handoff = null
  window.removeEventListener('pointermove', onHandoffMove)
  window.removeEventListener('pointerup', endHandoff)
  window.removeEventListener('pointercancel', endHandoff)
}

function onHandoffMove(e: PointerEvent): void {
  if (!handoff || e.pointerId !== handoff.down.pointerId) return
  const box = navEl.value?.getBoundingClientRect()
  if (!box || !leftRail(box, e.clientX, e.clientY)) return
  const { down, target, el } = handoff
  endHandoff()
  const bridge = workspaceBridge.value
  if (!bridge) return
  railDrag.cancel()
  clusterDrag.cancel()
  if (target.kind === 'pane') bridge.paneDrag(down, target.paneKey, el)
  else bridge.projectDrag(down, target.src, el)
}

function armHandoff(e: PointerEvent): void {
  endHandoff()
  if (e.button !== 0 || !workspaceBridge.value) return
  const el = e.target as HTMLElement
  if (el.closest('[data-no-drag], input, textarea')) return
  const head = el.closest<HTMLElement>('[data-side-head]')
  if (head) {
    const c = clusterByKey.value.get(head.dataset.orderKey ?? '')
    if (c) handoff = { down: e, el: head, target: { kind: 'project', src: { key: c.key, dir: headDir(c) ?? c.history[0] ?? null, name: nameOf(c) } } }
  } else {
    const row = el.closest<HTMLElement>('[data-rail-drag]')
    if (row?.dataset.dragKind === 'pane' && row.dataset.dragPane) handoff = { down: e, el: row, target: { kind: 'pane', paneKey: row.dataset.dragPane } }
  }
  if (!handoff) return
  window.addEventListener('pointermove', onHandoffMove)
  window.addEventListener('pointerup', endHandoff)
  window.addEventListener('pointercancel', endHandoff)
}

onBeforeUnmount(endHandoff)

function onPointerDown(e: PointerEvent): void {
  if (props.collapsed) return
  armHandoff(e)
  if (mode.value !== 'project') return
  clusterDrag.onDown(e)
  railDrag.onDown(e)
}

const ctxMenu = ref<{ at: DOMRect; trigger: HTMLElement | null; label: string; entries: MenuEntry[]; pick: (id: string) => void } | null>(null)
const KEY_MENU_ECHO_MS = 300
const echo = (): boolean => !!ctxMenu.value && performance.now() - keyMenuAt < KEY_MENU_ECHO_MS
const cursorAt = (e: MouseEvent): DOMRect => new DOMRect(e.clientX, e.clientY, 0, 0)

function dropEntries(entries: MenuEntry[], ids: string[]): MenuEntry[] {
  const kept = entries.filter((e) => !('id' in e) || !ids.includes(e.id))
  return kept.filter((e, i) => !('separator' in e) || (i > 0 && i < kept.length - 1 && !('separator' in kept[i - 1]!)))
}

function showRowMenu(row: LiveRow, at: DOMRect, trigger: HTMLElement | null): void {
  if (selection.value.keys.length > 1 && selectedSet.value.has(row.key)) return showSelectionMenu(at, trigger)
  if (selection.value.keys.length) selection.value = NO_SELECTION
  const pane = row.src.pane
  const tab = workspace.tabs.find((x) => x.key === row.tabKey)
  const cwd = workCwd(pane) ?? null
  const bridge = workspaceBridge.value
  const entries = dropEntries(
    rowMenu(
      {
        tabRow: row.key.startsWith('tab:'),
        cwd,
        explorer: hasExplorer && !!bridge,
        agent: isTerm(pane) && (row.src.kind === 'agent' || !!pane.terminalId),
        linked: !!pane.terminalId,
        context: hasCtxSources && isTerm(pane) && presetOf(pane) === 'claude',
        others: tab ? leaves(tab.root).length - 1 : 0,
        targets: rail.projectTabs(row.tabKey),
        compact: compactFlow.stateOf(pane),
        deepseek: hasDeepSeek,
        telegram: paneActions.telegramOf(pane),
        telegramOpen: paneActions.telegramOpenOf(pane),
        turnChanges: isTerm(pane) && pane.terminalId && hasChanges ? turnEdits.countOf(pane.terminalId) : undefined,
      },
      t,
    ),
    bridge ? [] : ['split-right', 'split-down'],
  )
  ctxMenu.value = { at, trigger, label: t('workspace.rail.menu.label', { name: row.title }), entries, pick: (id) => pickRow(id, row, cwd) }
}

function copy(text: string): void {
  void navigator.clipboard?.writeText(text).then(
    () => flash(t('workspace.rail.menu.copied')),
    () => flash(t('workspace.rail.menu.copyFailed')),
  )
}

function pickRow(id: string, row: LiveRow, cwd: string | null): void {
  if (id.startsWith('split:')) {
    const [, dir, choice] = id.split(':') as [string, SplitDir, SplitChoice]
    workspaceBridge.value?.split(row.paneKey, dir, choice)
    shell?.closeRail()
    return
  }
  if (id.startsWith('move-to:')) {
    const target = id.slice('move-to:'.length)
    if (row.key.startsWith('tab:')) mergeTab(row.tabKey, target)
    else movePane(row.paneKey, row.title, target)
    return
  }
  const shared = paneActions.run(
    id,
    { key: row.paneKey, pane: row.src.pane, name: row.title, cwd },
    {
      rename: () => (renaming.value = row.tabKey),
      close: () => closeRowTab(row),
      closeOthers: () => {
        const others = tabLeaves(row.tabKey).filter((k) => k !== row.paneKey)
        if (others.length) closeKeys(others)
      },
      copy,
    },
  )
  if (shared) return
  if (id === 'move-out') {
    workspace.detachPane(row.paneKey)
    flash(t('workspace.paneDrop.detached', { pane: row.title }))
  } else if (id === 'explorer') {
    workspaceBridge.value?.explorer(row.paneKey)
    shell?.closeRail()
  } else if (id === 'changesOfTurn' && row.src.pane.terminalId) {
    turnChanges.open(row.src.pane.terminalId, cwd ?? undefined)
    shell?.closeRail()
  } else if (id === 'context' && cwd) ui.contextSources = { cwd, terminalId: row.src.pane.terminalId ?? null }
}

function showSelectionMenu(at: DOMRect, trigger: HTMLElement | null): void {
  const panes = selectionPanes(
    selection.value.keys,
    (k) => {
      const r = rowByKey.value.get(k)?.src
      return r && { paneKey: r.paneKey, tabKey: r.tabKey, tabRow: r.key.startsWith('tab:') }
    },
    tabLeaves,
  )
  const facts = selectionFacts(panes, (k) => {
    const tab = workspace.tabOf(k)
    return tab && { key: tab.key, size: leaves(tab.root).length }
  })
  ctxMenu.value = {
    at,
    trigger,
    label: t('railSelect.menuLabel', { n: panes.length }),
    entries: selectionMenu({ count: panes.length, ...facts }, t),
    pick: (id) => {
      if (id === 'sel-group') {
        if (workspace.groupPanes(panes)) flash(t('railSelect.grouped', { n: panes.length }))
      } else if (id === 'sel-ungroup') {
        const n = workspace.ungroupPanes(panes)
        if (n) flash(t('railSelect.ungrouped', { n }, n))
      } else if (id === 'sel-close') closeKeys(panes)
      selection.value = NO_SELECTION
    },
  }
}

const headDir = (c: RailCluster): string | null => (c.checkouts.find((k) => k.active) ?? c.checkouts[0])?.path ?? c.root
const diskDirOf = (c: RailCluster | undefined): string | null => (!c || c.key === 'home' ? null : (c.root ?? headDir(c)))
const projectDisks = useProjectDisks(() => rail.clusters.value.map(diskDirOf).filter((d): d is string => !!d))
const headTip = (key: string): string => {
  const c = clusterByKey.value.get(key)
  return [c?.root, branchOf(key), ...projectDisks.linesOf(diskDirOf(c))].filter(Boolean).join('\n')
}
const nameOf = (c: RailCluster): string => (c.key === 'home' ? t('workspace.side.home') : c.name)
const clusterName = (key: string): string => {
  const c = clusterByKey.value.get(key)
  return c ? nameOf(c) : ''
}

const anchorOf = (c: RailCluster): RailRow | undefined => {
  const rowsOf = c.checkouts.flatMap((k) => k.rows)
  return rowsOf.find((r) => r.focused) ?? rowsOf.find((r) => r.tabActive) ?? rowsOf[0]
}

let projectMenuSeq = 0

async function showProjectMenu(c: RailCluster, at: DOMRect, trigger: HTMLElement | null): Promise<void> {
  const dir = headDir(c)
  const seq = ++projectMenuSeq
  const bridge = workspaceBridge.value
  const settle = <T,>(p: Promise<T>, fallback: T): Promise<T> => Promise.race([p.catch(() => fallback), new Promise<T>((r) => setTimeout(() => r(fallback), 400))])
  const [layouts] = await Promise.all([
    bridge && hasLayouts && dir ? settle(listLayouts(dir), []) : Promise.resolve(null),
    bridge && dir && !run.detected(dir) ? settle(run.detect(dir), null) : Promise.resolve(null),
  ])
  if (seq !== projectMenuSeq) return
  const anchor = anchorOf(c)
  const index = shownProjects.value.indexOf(c.key)
  const entries = buildProjectMenu(
      {
        mac: isMac,
        deepseek: hasDeepSeek,
        dir,
        home: c.key === 'home',
        live: c.live,
        anchored: !!anchor && !!bridge,
        viewers: { file: hasPreview, changes: hasChanges, log: hasLog },
        scripts: !!bridge && hasScripts,
        run: !!bridge && !!dir && run.runnable(dir),
        layouts,
        explorer: !!bridge && hasExplorer,
        env: hasEnvCheck,
        i18n: hasI18nScan,
        perms: hasPermAdvisor,
        buildClean: hasBuildClean,
        reveal: hasReveal,
        canUp: index > 0,
        canDown: index >= 0 && index < shownProjects.value.length - 1,
        collapsed: folded.value.has(c.key),
      },
      t,
    )
  ctxMenu.value = { at, trigger, label: t('workspace.rail.menu.label', { name: nameOf(c) }), entries, pick: (id) => void pickProject(id, c, dir, anchor, layouts ?? []) }
}

async function pickProject(id: string, c: RailCluster, dir: string | null, anchor: RailRow | undefined, layouts: SavedLayout[]): Promise<void> {
  const cwd = dir ?? undefined
  const bridge = workspaceBridge.value
  if (id.startsWith('new:')) {
    const choice = id.slice(4)
    if (choice === 'browser') workspace.openBrowser('', { cwd })
    else if (choice === 'changes' && dir) workspace.openChanges(dir)
    else if (choice === 'log' && dir) workspace.openLog(dir)
    else if (choice === 'file') {
      const path = await pickFile('any', cwd)
      if (path && /\.html?$/i.test(path)) workspace.openBrowser(path, { cwd })
      else if (path) workspace.openFile(path, undefined, { cwd })
    } else workspace.launch(choice as PtyPreset, cwd)
    return
  }
  if (id.startsWith('layout:')) {
    const layout = layouts.find((l) => `layout:${l.id}` === id)
    if (layout) bridge?.openLayout(layout)
    return
  }
  switch (id) {
    case 'run':
      if (dir) bridge?.run(dir)
      break
    case 'scripts':
      if (dir) bridge?.scripts(dir)
      break
    case 'plan':
      if (dir) ui.planBoard = { dir }
      break
    case 'layout-save':
      if (dir) bridge?.saveLayout(dir, nameOf(c))
      break
    case 'layout-manage':
      if (dir) bridge?.manageLayouts(dir, nameOf(c))
      break
    case 'explorer':
      if (anchor) bridge?.explorer(anchor.paneKey)
      break
    case 'env':
      if (dir) ui.envCheck = { dir }
      break
    case 'i18n':
      if (dir) ui.i18nScan = { dir }
      break
    case 'perms':
      if (dir) ui.permAdvisor = { dir }
      break
    case 'build-clean':
      if (dir) ui.buildClean = { dir }
      break
    case 'copy-path':
      if (dir) copy(dir)
      break
    case 'reveal':
      if (dir) void revealPath(dir)
      break
    case 'move-up':
    case 'move-down': {
      const from = shownProjects.value.indexOf(c.key)
      const at = railMoveIndexes(shownProjects.value, clusterOrder.value, from, from + (id === 'move-up' ? -1 : 1))
      if (at) rail.move(at[0], at[1])
      break
    }
    case 'toggle':
      toggleFold(c.key)
      break
    case 'close-project':
      closeKeys(c.tabKeys.flatMap(tabLeaves))
      break
    case 'forget':
      for (const d of c.history) workspace.forget(d)
      break
  }
}

function onRowMenu(row: LiveRow, e: MouseEvent): void {
  if (echo()) return
  showRowMenu(row, cursorAt(e), null)
}

function onHeadMenu(key: string, e: MouseEvent): void {
  const c = clusterByKey.value.get(key)
  if (c && !echo()) showProjectMenu(c, cursorAt(e), null)
}

function onHeadButton(key: string, e: MouseEvent): void {
  const c = clusterByKey.value.get(key)
  if (!c) return
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  showProjectMenu(c, new DOMRect(r.left, r.bottom, 0, 0), e.currentTarget as HTMLElement)
}

function onNavKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && selection.value.keys.length && !ctxMenu.value && !e.defaultPrevented) {
    e.preventDefault()
    e.stopPropagation()
    selection.value = NO_SELECTION
  }
}

const groupOptions = computed(() => [
  { value: 'project' as const, label: t('mist.rail.group.project') },
  { value: 'status' as const, label: t('mist.rail.group.status') },
])

const empty = computed(() => rows.value.length === 0)
const branchOf = (key: string): string => clusterByKey.value.get(key)?.checkouts.find((k) => k.branch)?.branch ?? ''
const statusLabel = (id: string): string => t(`mist.rail.status.${id.slice(2)}`)
const clusterKeyOf = (id: string): string => id.slice(2)
</script>

<template>
  <nav
    ref="navEl"
    class="flex min-h-0 flex-1 flex-col"
    :aria-label="t('mist.shell.sessions')"
    data-live-rail
    @pointerenter="hold = true"
    @pointerleave="hold = false"
    @pointerdown="onPointerDown"
    @click.capture="onClickCapture"
    @keydown="onNavKey"
  >
    <div v-if="!props.collapsed" class="flex shrink-0 items-center gap-2 px-2 pt-1 pb-1">
      <span class="shrink-0 text-2xs font-medium text-ink-faint" aria-hidden="true">{{ t('mist.rail.group.name') }}</span>
      <SegmentedControl v-model="uigen.railGroup" :options="groupOptions" :label="t('mist.rail.group.label')" size="sm" fill class="min-w-0 flex-1" data-rail-group />
      <button
        v-if="openable"
        type="button"
        class="ccm-btn-plain inline-flex size-7 shrink-0 items-center justify-center disabled:cursor-default disabled:text-ink-faint"
        :title="t('workspace.rail.openProjectHint')"
        :aria-label="t('workspace.rail.openProject')"
        :disabled="picking"
        data-no-drag
        data-rail-open-project
        @click="openProject"
      >
        <LoaderCircle v-if="picking" :size="15" class="ccm-spin" aria-hidden="true" />
        <Plus v-else :size="15" aria-hidden="true" />
      </button>
    </div>
    <div ref="listEl" class="ccm-scroll min-h-0 flex-1 overflow-y-auto px-1.5 py-1" data-rail-list>
      <p v-if="empty && !props.collapsed" class="px-2 py-3 text-2xs text-ink-faint">{{ t('mist.rail.empty') }}</p>
      <RailCollapsed v-else-if="props.collapsed" :projects="collapsedList" @open="activateRow" @menu="onRowMenu" />
      <template v-else>
        <template v-for="s in body" :key="s.id">
          <section v-if="s.id.startsWith('p:')" class="mb-1.5" :data-group="clusterKeyOf(s.id)">
            <div
              class="group/head flex h-8 min-w-0 items-center gap-1.5 rounded-control px-1.5"
              data-side-head
              :data-order-key="clusterKeyOf(s.id)"
              :title="headTip(clusterKeyOf(s.id))"
              @contextmenu.prevent="onHeadMenu(clusterKeyOf(s.id), $event)"
            >
              <button
                type="button"
                class="inline-grid size-5 shrink-0 cursor-pointer place-items-center rounded-control text-ink-faint outline-none hover:text-ink focus-visible:outline-2 focus-visible:outline-focus"
                :aria-expanded="!folded.has(clusterKeyOf(s.id))"
                :aria-label="t('mist.rail.project', { name: clusterName(clusterKeyOf(s.id)) })"
                data-no-drag
                @click="toggleFold(clusterKeyOf(s.id))"
              >
                <ChevronRight v-if="folded.has(clusterKeyOf(s.id))" :size="14" aria-hidden="true" />
                <ChevronDown v-else :size="14" aria-hidden="true" />
              </button>
              <span class="ccm-next-rail-mono inline-grid size-5 shrink-0 place-items-center rounded-[6px] text-[11px] font-bold" :data-tone="sectionRows(s)[0]?.tone ?? 0" aria-hidden="true">{{
                monogram(clusterName(clusterKeyOf(s.id)))
              }}</span>
              <span class="min-w-0 truncate text-xs font-semibold text-ink">{{ clusterName(clusterKeyOf(s.id)) }}</span>
              <span v-if="branchOf(clusterKeyOf(s.id))" class="min-w-0 shrink truncate text-2xs text-ink-faint">{{ branchOf(clusterKeyOf(s.id)) }}</span>
              <span class="ccm-tabular ml-auto shrink-0 text-2xs text-ink-faint">{{ s.keys.length }}</span>
              <button
                type="button"
                class="inline-grid size-6 shrink-0 cursor-pointer place-items-center rounded-control text-ink-muted opacity-0 outline-none group-hover/head:opacity-100 hover:bg-fill focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-focus"
                :aria-label="t('mist.rail.projectMenu')"
                data-no-drag
                @click="onHeadButton(clusterKeyOf(s.id), $event)"
              >
                <Ellipsis :size="14" aria-hidden="true" />
              </button>
            </div>
            <div v-if="!folded.has(clusterKeyOf(s.id))" class="ccm-next-rail-children ml-[17px] flex flex-col gap-0.5 border-l border-line pl-1" data-rail-children>
              <RailRowItem
                v-for="r in sectionRows(s)"
                :key="r.key"
                :row="r"
                :selected="selectedSet.has(r.key)"
                :renaming="renaming === r.tabKey"
                :tab-stop="tabStopKey === r.key"
                @open="activateRow"
                @menu="onRowMenu"
                @renamed="onRenamed"
                @key="onRowKey"
                @close="closeRowTab"
              />
            </div>
          </section>
          <section v-else class="mb-1.5" :data-status="clusterKeyOf(s.id)">
            <div class="ccm-label flex h-6 items-center gap-1.5 px-2 text-2xs font-semibold text-ink-muted">
              <span>{{ statusLabel(s.id) }}</span>
              <span class="ccm-tabular text-ink-faint">{{ s.keys.length }}</span>
            </div>
            <div class="flex flex-col gap-0.5">
              <RailRowItem
                v-for="r in sectionRows(s)"
                :key="r.key"
                :row="r"
                show-project
                :selected="selectedSet.has(r.key)"
                :renaming="renaming === r.tabKey"
                :tab-stop="tabStopKey === r.key"
                @open="activateRow"
                @menu="onRowMenu"
                @renamed="onRenamed"
                @key="onRowKey"
                @close="closeRowTab"
              />
            </div>
          </section>
        </template>
      </template>
    </div>
    <p class="sr-only" role="status" aria-live="polite">{{ announce }}</p>
    <ContextMenu v-if="ctxMenu" :at="ctxMenu.at" :entries="ctxMenu.entries" :label="ctxMenu.label" :trigger="ctxMenu.trigger" @pick="(id) => ctxMenu?.pick(id)" @close="ctxMenu = null" />
  </nav>
</template>
