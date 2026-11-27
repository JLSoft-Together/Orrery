<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { shown } from '../../../core/lib/shareMode'
import { Blocks, Columns2, GitCompareArrows, FileText, FolderOpen, GitBranch, ArrowUpRight, Globe, History as HistoryIcon, ListOrdered, LoaderCircle, Mail, Maximize2, MessageSquareText, Minimize2, Moon, Network, Paperclip, Play, RotateCw, Smartphone, ScanSearch, SquareTerminal, TriangleAlert, X } from 'lucide-vue-next'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { UnicodeGraphemesAddon } from '@xterm/addon-unicode-graphemes'
import { SearchAddon } from '@xterm/addon-search'
import '@xterm/xterm/css/xterm.css'
import type { IBufferCell, IDisposable } from '@xterm/xterm'
import type { PtyServerMessage } from '@ccm/shared'
import { DevServerDetector, devUrlLabel, rememberDevUrl } from '@features/resources'
import { devScanSlice } from '../lib/devServerUrl'
import { DevChipGate } from '../lib/devChipGate'
import { addCmdHistory, hasCmdHistory, loadCmdHistory, pickMode, removeCmdHistory, type CmdEntry } from '../lib/cmdHistory'
import { historyKey, markKeyAction } from '../lib/workspaceKeys'
import { useTermBookmarks } from '../composables/useTermBookmarks'
import CmdHistoryPicker from './CmdHistoryPicker.vue'
import { createFileLinkProvider, inLinks, joinWrappedLinks, oscLinkUri, selectionText, type LinkOpenOptions, type LinkTarget } from '../../files/lib/fileLinks'
import { fontEntry, termFontStack } from '../../../core/lib/fonts'
import { duration, now, shortPath } from '../../../core/lib/format'
import { folderName } from '../../rail/lib/sidebarRows'
import type { MenuEntry } from '../../../core/lib/menu'
import { PANE_LAYOUT_MENU } from '../lib/layoutPaneMenu'
import { newlineFor } from '../lib/termKeys'
import { installReplacementIme } from '../lib/termIme'
import { progressPct } from '../../monitor/lib/agentStatus'
import { meterState } from '../lib/paneV93'
import { headMode, isHeadBlank, processChip, STATE_BADGE, statusWordShown, suggestCompact, zoomKeys } from '../lib/paneHead'
import { contextPctOf } from '../../../core/lib/provider'
import { retireOverlay } from '../../../core/lib/motion'
import { registerSnapshot } from '../lib/paneSnapshots'
import { registerTermSource, type FindSeed, type RevealTarget } from '../../termSearch/registry'
import { MAX_ROWS, type TermDump, type TermRow } from '../../termSearch/search'
import { HiddenOutput, TERM_SCROLLBACK } from '../lib/termBuffer'
import { webglPool } from '../lib/webglLru'
import { isMac } from '../../../core/lib/platform'
import { PtyConnection, type PtyStart } from '../lib/ptyClient'
import { spawnQueue, type SpawnTicket } from '../../agent-tools/lib/spawnQueue'
import { imagePasteKey, usePaneAttachments, type PaneScreen } from '../../agent-tools/composables/usePaneAttachments'
import { SCAN_ROWS } from '../../agent-tools/lib/promptAttachments'
import { desktop, hasDeepSeek, presetEnabled } from '../../../core/lib/desktop'
import { clipboardImageBlob, imageData } from '../../../core/lib/imageData'
import { dragPaths, hasDragPath } from '../../files/lib/explorer/dragPath'
import { hasAttachments, pathsForFiles } from '../../../core/lib/preview'
import { useAgentsStore } from '../../../core/stores/agents'
import { fromPath, useAttachmentsStore, type NewAttachment } from '../../agent-tools/stores/attachments'
import { useSettingsStore } from '../../../core/stores/settings'
import { NEXT_CODE_FONT, useUiGenStore } from '../../next/stores/uiGen'
import PaneHeader from '../../next/components/ui/PaneHeader.vue'
import ClashChip from '../../agent-tools/components/ClashChip.vue'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useTipsStore } from '@features/update'
import { useUiStore } from '../../../core/stores/ui'
import { useRemoteStore } from '@features/remote'
import { stripControlChars, usePromptsMenu } from '@features/prompts'
import { RemotePhoneChip } from '@features/remote'
import { usePanePhone } from '../composables/usePanePhone'
import { parseOsc7, parseOsc9 } from '../lib/shellCwd'
import { hasScripts } from '../lib/projects'
import { bufferLines, parseTerminalProblems } from '../lib/terminalProblems'
import { BOOT_MAX_MS, BOOT_SLOW_MS, bootDone, shouldBoot } from '../lib/paneBoot'
import { useChecksStore } from '../../files/stores/checks'
import { presetOf, useWorkspaceStore, reopenAs, resumeIdOf, workCwd, type Pane, type SplitChoice } from '../stores/workspace'
import ContextGauge from '../../monitor/components/ContextGauge.vue'
import ContextTokens from '../../monitor/components/ContextTokens.vue'
import AttachmentsPanel from '../../agent-tools/components/AttachmentsPanel.vue'
import TermFind from './TermFind.vue'
import ChatView from '../../chat/ChatView.vue'
import { hasChatView, type ChatTarget } from '../../chat/feed'
import { useTurnChanges } from '@features/git'
import { useTurnEditsStore } from '../../agent-tools/stores/turnEdits'
import { demotedKeys, menuKeys, pinnedShown, type HeadActionCtx } from '../lib/paneHeadActions'
import StatusIcon from '../../../core/components/StatusIcon.vue'
import { statusMeta } from '../../../core/lib/status'
import { usePaneInfo, usePeerSweep } from '../composables/usePaneInfo'
import { useCompact } from '../../agent-tools/composables/useCompact'
import { HIDDEN_MENU_THROTTLE_MS, MENU_THROTTLE_MS, useTermMenu } from '../../click-menu/composables/useTermMenu'
import { useMenuQueueStore } from '../../click-menu/stores/menuQueue'
import { useTermMenuMouse } from '../../click-menu/composables/useTermMenuMouse'
import { needsUser } from '../../rail/lib/railModel'
import { leaves, viewportFits } from '../lib/splitLayout'
import PaneMore from './PaneMore.vue'
import TermMenuBar from '../../click-menu/components/TermMenuBar.vue'
import PresetIcon from './PresetIcon.vue'
import SplitMenu from './SplitMenu.vue'
import WorktreeMenu from '../../git/components/WorktreeMenu.vue'
import OverlapChip from '../../agent-tools/components/OverlapChip.vue'
import AgentStatusChip from './AgentStatusChip.vue'
import SubagentOrbit from './SubagentOrbit.vue'
import ContextMeter from '../../agent-tools/components/ContextMeter.vue'
import TurnCard from '../../agent-tools/components/TurnCard.vue'
import { useAgentStatusStore } from '../../monitor/stores/agentStatus'
import PromptQueueButton from '../../agent-tools/components/PromptQueueButton.vue'
import LimitResumeChip from '../../agent-tools/components/LimitResumeChip.vue'
import { shouldAutoFill } from '../../agent-tools/lib/promptQueue'
import { usePromptQueueStore } from '../../agent-tools/stores/promptQueue'
import { useResumeScheduleStore } from '../../agent-tools/stores/resumeSchedule'
import { useShellMarks, type FinishedCommand } from '../composables/useShellMarks'
import { useShellDone } from '../../shellDone/useShellDone'
import ContextMenu from '../../../core/components/ui/ContextMenu.vue'

const props = defineProps<{ pane: Pane; visible: boolean; focused: boolean; multi: boolean; zoomed: boolean; hiddenCount: number; dim: boolean; grouped?: boolean }>()
const emit = defineEmits<{ split: [choice: SplitChoice]; close: []; openLink: [target: LinkTarget, opts: Partial<LinkOpenOptions>]; scripts: []; zoom: [] }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()
const remote = useRemoteStore()
const settings = useSettingsStore()
const uiGen = useUiGenStore()
const isNext = computed(() => uiGen.isNext)
const codeFontId = (): string => (uiGen.isNext && settings.codeFont === 'system' ? NEXT_CODE_FONT : settings.codeFont)
const terminals = useTerminalsStore()
const agents = useAgentsStore()
const tips = useTipsStore()
const ui = useUiStore()

const host = ref<HTMLElement | null>(null)
let term: Terminal | null = null
let fit: FitAddon | null = null
let webgl: WebglAddon | null = null
let conn: PtyConnection | null = null
let search: SearchAddon | null = null
let linkProvider: IDisposable | null = null
let resizeObserver: ResizeObserver | null = null
let refitTimer: ReturnType<typeof setTimeout> | null = null
let restoredOnMount = false
let ticket: SpawnTicket | null = null
const queued = ref(false)
const findOpen = ref(false)
const find = ref<InstanceType<typeof TermFind> | null>(null)
const findSeed = ref<FindSeed | null>(null)
const searchAddon = ref<SearchAddon | null>(null)
const devUrls = ref<string[]>([])
const devGate = new DevChipGate()
let devBindTimer: ReturnType<typeof setTimeout> | null = null
onBeforeUnmount(() => devBindTimer && clearTimeout(devBindTimer))
const linkHint = ref<string | undefined>(undefined)
// OSC 8 links the program printed (newest last): a selection across a wrapped one copies as one line.
const OSC_LINKS_MAX = 50
const oscLinks: string[] = []
// Full text of the link under the mouse; right-click copies it.
let hoveredLink: string | null = null
const dev = new DevServerDetector()
const decoder = new TextDecoder('utf-8')
const output = new HiddenOutput({ write: (bytes, done) => (term ? term.write(bytes, done) : done?.()) })
let backlog: (() => void) | null = null

function showBacklog(): Promise<void> {
  backlog?.()
  return new Promise((resolve) => {
    const done = (): void => {
      if (backlog === done) backlog = null
      resolve()
    }
    backlog = done
    output.setVisible(true, done)
  })
}

// The sidebar age needs 'recently active', not every chunk: one store write per pane every few seconds at most.
const TOUCH_MS = 5000
let touchedAt = 0
const booting = ref(false)
const bootMs = ref(0)
let bootTimer: ReturnType<typeof setInterval> | null = null
let bootFrame = 0
function stopBoot(): void {
  booting.value = false
  if (bootTimer) clearInterval(bootTimer)
  bootTimer = null
  if (bootFrame) cancelAnimationFrame(bootFrame)
  bootFrame = 0
}
function checkBoot(): void {
  if (!booting.value || bootFrame) return
  bootFrame = requestAnimationFrame(() => {
    bootFrame = 0
    if (term && bootDone(bufferLines(term.buffer.active, 200))) stopBoot()
  })
}
function startBoot(): void {
  stopBoot()
  booting.value = true
  bootMs.value = 0
  const at = Date.now()
  bootTimer = setInterval(() => {
    bootMs.value = Date.now() - at
    if (bootMs.value >= BOOT_MAX_MS) stopBoot()
    else checkBoot()
  }, 500)
}
onBeforeUnmount(stopBoot)

function touch(): void {
  const at = Date.now()
  if (at - touchedAt < TOUCH_MS) return
  touchedAt = at
  workspace.touch(props.pane.key, at)
}

function feedDev(bytes: Uint8Array): void {
  if (preset.value !== 'shell') return
  const found = dev.feed(decoder.decode(devScanSlice(bytes), { stream: true }))
  if (!found.length) return
  devUrls.value = [...devUrls.value, ...found]
  bindDevGate(8)
  if (marksOn.value) bookmarks.addServer(devUrlLabel(found[0]!))
  for (const url of found) rememberDevUrl(cwd.value ?? '', url)
  if (settings.devPreviewAutoOpen && props.pane.state === 'running') {
    for (const url of found) workspace.autoOpenDevPreview(props.pane.key, url, cwd.value ?? undefined)
  }
}

function rememberLink(uri: string | null): void {
  if (!uri) return
  const at = oscLinks.indexOf(uri)
  if (at >= 0) oscLinks.splice(at, 1)
  oscLinks.push(uri)
  if (oscLinks.length > OSC_LINKS_MAX) oscLinks.shift()
}

function hoverLink(text: string): void {
  hoveredLink = text
  linkHint.value = t('workspace.open.linkHint', { key: isMac ? '⌘' : 'Ctrl' })
}

function leaveLink(): void {
  hoveredLink = null
  linkHint.value = undefined
}

function copyText(text: string): void {
  void navigator.clipboard.writeText(text).catch(() => undefined)
}

function copySelection(): void {
  if (!term) return
  copyText(selectionText(term, oscLinks))
  term.clearSelection()
}

// Edit menu / ⌘C reaching xterm's textarea: same text as the Ctrl+C path (xterm's own handler would copy the raw rows).
function onCopy(e: ClipboardEvent): void {
  if (!term?.hasSelection() || !e.clipboardData) return
  e.preventDefault()
  e.stopPropagation()
  e.clipboardData.setData('text/plain', selectionText(term, oscLinks))
}

// OSC 52: full-screen TUIs (Claude Code /tui fullscreen) select with the mouse themselves and copy through the terminal. Write only; a read request ('?') is ignored.
const OSC52_MAX = 1_000_000
function onOsc52(data: string): boolean {
  const b64 = data.slice(data.indexOf(';') + 1)
  if (!b64 || b64 === '?' || b64.length > OSC52_MAX) return true
  try {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
    copyText(joinWrappedLinks(new TextDecoder().decode(bytes), inLinks(oscLinks)))
  } catch {
    // not base64
  }
  return true
}

function onContextMenu(e: MouseEvent): void {
  if (!hoveredLink) return
  e.preventDefault()
  copyText(hoveredLink)
}

function bindDevGate(left: number): void {
  if (devBindTimer) clearTimeout(devBindTimer)
  devBindTimer = null
  const id = marks.runningId()
  if (id !== null) return devGate.bind(id)
  if (left > 0) devBindTimer = setTimeout(() => bindDevGate(left - 1), 200)
}

function resetDev(): void {
  dev.reset()
  devGate.reset()
  if (devBindTimer) clearTimeout(devBindTimer)
  devBindTimer = null
  devUrls.value = []
}

async function openFind(): Promise<void> {
  findOpen.value = true
  await nextTick()
  find.value?.focus()
}

function closeFind(): void {
  findOpen.value = false
  term?.focus()
}

const procChip = computed(() => {
  const kind = processChip(props.pane.state)
  if (kind === 'parked') return { kind, label: t('workspace.rail.state.parked'), title: t('workspace.parked.hint'), status: 'stale', cls: 'bg-fill text-ink-muted' }
  if (kind === 'starting' && queued.value) return { kind, label: t('workspace.queued.label'), title: t('workspace.queued.hint'), status: null, cls: 'bg-fill text-ink-muted' }
  if (kind === 'starting') return { kind, label: t('workspace.rail.state.starting'), title: undefined, status: null, cls: 'bg-fill text-ink-muted' }
  if (kind === 'exited') {
    const meta = statusMeta('completed')
    return { kind, label: t('workspace.rail.state.ended'), title: t('workspace.state.exited', { code: props.pane.exitCode ?? 0 }), status: 'completed', cls: `${meta.text} ${meta.chip}` }
  }
  if (kind === 'error') {
    const text = props.pane.error ?? t('workspace.state.error')
    const meta = statusMeta('error')
    return { kind, label: text, title: text, status: 'error', cls: `${meta.text} ${meta.chip}` }
  }
  return null
})

const session = computed(() => {
  const term = props.pane.terminalId ? terminals.byId[props.pane.terminalId] : undefined
  return term && term.status !== 'stale' ? term : undefined
})
const mainAgent = computed(() => (session.value ? agents.mainOf(session.value.id) : undefined))
const context = computed(() => {
  const a = mainAgent.value
  return a?.contextTokens && a.contextWindow ? { tokens: a.contextTokens, window: a.contextWindow, pct: contextPctOf(a) } : null
})
const branch = computed(() => session.value?.gitBranch || props.pane.branch)
const wtAnchor = ref<DOMRect | null>(null)
const wtButton = ref<HTMLButtonElement | null>(null)
const wtFrom = ref<HTMLElement | null>(null)

function toggleWorktrees(from?: HTMLElement): void {
  if (wtAnchor.value) {
    wtAnchor.value = null
    return
  }
  wtFrom.value = from ?? null
  wtAnchor.value = (wtButton.value ?? from ?? more.value?.button)?.getBoundingClientRect() ?? null
}

function closeWorktrees(): void {
  wtAnchor.value = null
  ;(wtButton.value ?? wtFrom.value)?.focus()
}

const splitAnchor = ref<DOMRect | null>(null)
const splitButton = ref<HTMLButtonElement | null>(null)

// Clicking into another pane has a chord (Alt+Arrows): counted for the shortcut tip.
function onPointerDown(): void {
  if (props.multi && !props.focused) tips.mouse('focus')
  workspace.activate(props.pane.key)
}

function toggleSplit(from?: HTMLElement): void {
  if (splitAnchor.value) {
    splitAnchor.value = null
    return
  }
  splitAnchor.value = (splitButton.value ?? from ?? more.value?.button)?.getBoundingClientRect() ?? null
}

// Menus are placed from a rect taken at open time: a hidden pane or a resized window would leave them floating.
function dropMenus(): void {
  splitAnchor.value = null
  wtAnchor.value = null
  more.value?.close()
  marks.closeMenu()
}

const ended = computed(() => props.pane.state === 'exited' || props.pane.state === 'error' || props.pane.state === 'gone')
const parked = computed(() => props.pane.state === 'parked')
const info = usePaneInfo()
const sessionChip = computed(() => {
  const s = session.value
  const state = info.railStateOf(props.pane)
  if (!s || !state || props.pane.state !== 'running') return null
  const badge = STATE_BADGE[state]
  return { state, badge, status: info.statusOf(props.pane) ?? s.status, label: info.stateText(props.pane) ?? '', subagents: info.subagentsOf(props.pane), meta: statusMeta(badge) }
})
const chipKey = computed(() => (sessionChip.value ? `s:${sessionChip.value.state}:${sessionChip.value.label}` : `p:${procChip.value?.kind ?? ''}:${procChip.value?.label ?? ''}`))

const agentStatus = useAgentStatusStore()
const workflowStatus = computed(() => (isShell.value ? undefined : agentStatus.statusOf(props.pane.key)))
const peer = computed(() => (props.pane.state === 'running' ? info.peerOf(props.pane) : null))
const peerBadge = computed(() => !!sessionChip.value && (isNext.value || needsUser(sessionChip.value.state)))
const workflowText = computed(() => {
  const w = workflowStatus.value
  if (!w) return ''
  const pct = progressPct(w)
  return pct === null ? w.phase : `${w.phase} ${pct}%`
})
const chipTitle = computed(() => {
  const label = sessionChip.value?.label ?? ''
  const extra = peerBadge.value ? [peer.value ? peerTip.value : coordinating.value?.label] : []
  if (isNext.value && workflowStatus.value) extra.push([workflowStatus.value.phase, workflowStatus.value.note].filter(Boolean).join(': '))
  return [label, ...extra].filter(Boolean).join('\n')
})
const coordinating = computed(() => (props.pane.state === 'running' ? info.coordinatingOf(props.pane) : null))
const peerTip = computed(() => [...new Set([peer.value?.label ?? '', ...info.peerLines(props.pane), peer.value?.action ?? ''].filter(Boolean))].join('\n'))
const peerAria = computed(() => (peer.value ? (peer.value.action ? t('peerWs.chipLabel', { text: peer.value.label, action: peer.value.action }) : peer.value.label) : ''))
const peerSweep = usePeerSweep(() => peer.value)
const hiddenNews = computed(() => {
  if (!props.zoomed || props.hiddenCount <= 0) return null
  const tab = workspace.tabOf(props.pane.key)
  return tab ? info.peerNewsOf(leaves(tab.root).filter((k) => k !== props.pane.key).map((k) => workspace.panes.find((p) => p.key === k))) : null
})
const hiddenLabel = computed(() =>
  [t('workspace.paneHead.hidden', { n: props.hiddenCount }, props.hiddenCount), hiddenNews.value?.label].filter(Boolean).join(', '),
)

const activity = computed(() => info.activityOf(props.pane))
const activityText = computed(() => {
  const tool = activity.value?.tool
  if (!tool) return null
  const time = duration(activity.value?.since, props.visible ? now.value : Date.now())
  return time ? t('workspace.activity.tool', { tool, time }) : tool
})
const diff = computed(() => {
  const d = activity.value?.diff
  return d && d.files > 0 ? d : null
})

const compactFlow = useCompact()
const compactSuggested = computed(() => props.pane.state === 'running' && !!context.value && suggestCompact(context.value.pct, compactFlow.stateOf(props.pane)))
const compactPct = computed(() => Math.round(context.value?.pct ?? 0))
const isClaude = computed(() => preset.value === 'claude')
const orbitAgents = computed(() => (isClaude.value && props.pane.terminalId && session.value ? agents.byTerminal[props.pane.terminalId] : undefined))
const meterOn = computed(() => !isShell.value && props.pane.state === 'running')
const meterOver = computed(() => meterState(mainAgent.value?.contextTokens, settings.contextWarnTokens)?.tone === 'over')
const GAUGE_FROM_PCT = 70
const REVEAL = 'opacity-0 transition-opacity duration-(--ccm-dur-fast) group-hover/head:opacity-100 group-focus-within/head:opacity-100 focus-visible:opacity-100'
const gaugeShown = computed(() => !isNext.value || ((context.value?.pct ?? 0) >= GAUGE_FROM_PCT && !compactSuggested.value))
const canMeterCompact = computed(() => compactFlow.stateOf(props.pane) === 'ready')

const barSlot = inject<Ref<HTMLElement | null>>('ccm-pane-bar', ref(null))
const inBar = computed(() => !isNext.value && headMode(props.multi, props.zoomed) === 'bar' && props.visible && !!barSlot.value)
const headTip = computed(() => shown([cwd.value, activityText.value, session.value ? sessionTitle.value : ''].filter(Boolean).join('\n')) || undefined)

const zoomLabel = computed(() => t(props.zoomed ? 'workspace.paneHead.unzoom' : 'workspace.paneHead.zoom', { key: zoomKeys(isMac) }))

function onHeadDblClick(e: MouseEvent): void {
  if (!(props.multi || props.zoomed) || !isHeadBlank(e.target, e.currentTarget as Element)) return
  e.preventDefault()
  emit('zoom')
}

const more = ref<InstanceType<typeof PaneMore> | null>(null)
const layoutMenu = inject(PANE_LAYOUT_MENU, null)
const reconnectable = computed(() => props.pane.state === 'error' && !!props.pane.sessionId)
const devChipUrl = computed(() => (ended.value || parked.value ? undefined : devUrls.value[devUrls.value.length - 1]))
const promptsMenu = usePromptsMenu()
const moreEntries = computed<MenuEntry[]>(() => {
  const out: MenuEntry[] = []
  const url = devUrls.value[devUrls.value.length - 1]
  if (url) out.push({ id: 'dev', label: t('workspace.devServer.open', { host: devUrlLabel(url) }), icon: Globe })
  if (compactSuggested.value) out.push({ id: 'compact', label: t('workspace.paneHead.compact.label', { pct: compactPct.value }), icon: TriangleAlert, hint: t('workspace.compact.hint') })
  if (isNext.value && peer.value?.other) out.push({ id: 'peer', label: peerAria.value, icon: Mail })
  out.push({ separator: true })
  for (const key of menuKeys(headCtx.value, headWidth.value)) {
    if (key === 'attach') out.push({ id: 'attach', label: attachLabel.value, icon: Paperclip })
    else if (key === 'chat') out.push({ id: 'chat', label: chatLabel.value, icon: chatOn.value ? SquareTerminal : MessageSquareText })
    else if (key === 'turn') out.push({ id: 'turn', label: turnLabel.value, icon: GitCompareArrows })
    else if (key === 'tools') out.push({ id: 'tools', label: t('sessionTools.menu'), icon: Blocks })
    else if (key === 'worktree' && cwd.value && props.pane.state === 'running')
      out.push({ id: 'worktree', label: branch.value ? t('workspace.paneHead.branch', { branch: branch.value }) : t('workspace.wt.title'), icon: GitBranch })
    else if (key === 'split') out.push({ id: 'split', label: t('workspace.splitPane'), icon: Columns2 })
    else if (key === 'phone') out.push({ id: 'phone', label: t('workspace.paneHead.phone.label'), icon: Smartphone })
    else if (key === 'prompts') out.push(promptsMenu.entry.value)
  }
  out.push({ separator: true })
  if (hasScripts && cwd.value) out.push({ id: 'scripts', label: t('workspace.scripts.open'), icon: Play })
  if (isShell.value) {
    out.push({ id: 'scan', label: t('workspace.termScan.button'), icon: ScanSearch, hint: t('workspace.termScan.hint') })
    if (marksOn.value) {
      if (canHistory.value) out.push({ id: 'history', label: t('termMarks.history'), icon: HistoryIcon, hint: t('termMarks.historyHint', { key: isMac ? '⌃R' : 'Ctrl+R' }) })
      out.push({ separator: true }, ...bookmarks.entries.value)
    }
  } else {
    const n = queueCount.value
    out.push({ id: 'queue', label: n ? t('workspace.promptQueue.count', { n }, n) : t('workspace.promptQueue.title'), icon: ListOrdered })
  }
  out.push({ id: 'restart', label: t(reconnectable.value ? 'workspace.reconnect' : 'workspace.restart'), icon: RotateCw })
  const layout = layoutMenu?.entries(props.pane.key) ?? []
  if (layout.length) out.push({ separator: true }, ...layout)
  return out
})

const canTools = computed(() => preset.value === 'claude' && !!desktop?.extProject && !!cwd.value && !ended.value)
const rootEl = ref<HTMLElement | null>(null)
const headWidth = ref<number | null>(null)
let headObserver: ResizeObserver | null = null
const turnChanges = useTurnChanges()
const turnEdits = useTurnEditsStore()
const turnCount = computed(() => (props.pane.terminalId ? turnEdits.countOf(props.pane.terminalId) : 0))
const canTurn = computed(() => !isShell.value && !!props.pane.terminalId && !!cwd.value && props.pane.state === 'running')
const panePhone = usePanePhone(() => props.pane.key)
const headCtx = computed<HeadActionCtx>(() => ({ agent: !isShell.value, attach: canAttach.value, chat: canChat.value, turn: canTurn.value, tools: canTools.value, phone: panePhone.available.value }))
const pinned = computed(() => new Set(pinnedShown(headCtx.value, headWidth.value)))
const demoted = computed(() => demotedKeys(headCtx.value))
const turnLabel = computed(() => (turnCount.value ? `${t('v104.turnChanges.menu')} (${turnCount.value})` : t('v104.turnChanges.menu')))

function openTurn(): void {
  if (props.pane.terminalId) turnChanges.open(props.pane.terminalId, cwd.value ?? undefined)
}
const attachLabel = computed(() => (attachCount.value ? t('workspace.paneHead.attachCount', { label: t('attach.button'), n: attachCount.value }) : t('attach.button')))

function openTools(): void {
  const dir = session.value?.cwd || cwd.value
  if (dir) ui.sessionTools = { dir, paneKey: props.pane.key }
}

function pickMore(id: string, anchor?: HTMLElement): void {
  if (promptsMenu.pick(id)) return
  if (layoutMenu?.pick(props.pane.key, id)) return
  if (bookmarks.pick(id)) return
  if (id === 'history') return void openHistory()
  if (id === 'attach') attachments.toggle(props.pane.key)
  else if (id === 'chat') toggleChat()
  else if (id === 'turn') openTurn()
  else if (id === 'tools') openTools()
  else if (id === 'worktree') toggleWorktrees(anchor)
  else if (id === 'split') toggleSplit(anchor)
  else if (id === 'phone') void panePhone.send()
  else if (id === 'dev') {
    const url = devUrls.value[devUrls.value.length - 1]
    if (url) emit('openLink', { kind: 'url', url }, {})
  } else if (id === 'compact') compactFlow.start(props.pane.key)
  else if (id === 'peer') info.goToPeer(peer.value?.other)
  else if (id === 'scripts') emit('scripts')
  else if (id === 'scan') scanErrors()
  else if (id === 'queue') queueButton.value?.open(more.value?.button?.getBoundingClientRect())
  else if (id === 'restart') {
    if (reconnectable.value) reconnect()
    else restart()
  }
}

const cwd = computed(() => workCwd(props.pane))
const ownProject = computed(() => (props.pane.project && !props.grouped ? folderName(props.pane.project) : null))
// What runs in the pane now (a shell can be running `claude`); spawn and restart keep `pane.preset`.
const preset = computed(() => presetOf(props.pane))
let reportedCwd: string | null = null

const isShell = computed(() => preset.value === 'shell')
const chatOn = ref(false)
const canChat = computed(() => hasChatView && (preset.value === 'claude' || preset.value === 'codex'))
const chatTarget = computed<ChatTarget | null>(() => {
  if (!canChat.value) return null
  const term = props.pane.terminalId ? terminals.byId[props.pane.terminalId] : undefined
  const agent = preset.value === 'codex' ? 'codex' : 'claude'
  const own = agent === 'claude' ? term?.claudeSessionId : (term?.codexSessionId ?? (term?.id.startsWith('codex:') ? term.id.slice(6) : undefined))
  return { key: props.pane.key, agent, session: own ?? props.pane.agentSessionId ?? '', cwd: term?.cwd ?? cwd.value ?? '' }
})
const chatLabel = computed(() => t(chatOn.value ? 'chatView.toTerminal' : 'chatView.toChat'))
watch(canChat, (ok) => {
  if (!ok) chatOn.value = false
})
function toggleChat(): void {
  chatOn.value = !chatOn.value
  if (chatOn.value) findOpen.value = false
  else void nextTick(() => term?.focus())
}
const termMenu = useTermMenu({
  term: () => term,
  enabled: () => settings.clickMenu && props.visible && props.pane.state === 'running' && (preset.value === 'claude' || preset.value === 'codex'),
  send: (keys) => {
    if (conn?.isOpen) conn.write(keys)
  },
  focus: () => term?.focus(),
  observe: () => props.pane.state === 'running' && (preset.value === 'claude' || preset.value === 'codex'),
  throttle: () => (props.visible ? MENU_THROTTLE_MS : HIDDEN_MENU_THROTTLE_MS),
  blocked: () => !!props.pane.typeKeys,
})
const menuQueue = useMenuQueueStore()
const unregisterMenu = menuQueue.register(props.pane.key, termMenu.pick)
watch(
  () => termMenu.seen.value,
  (seen) => menuQueue.report(props.pane.key, seen),
  { immediate: true },
)
watch(
  () => [settings.clickMenu, props.visible, props.pane.state, preset.value] as const,
  () => termMenu.refresh(),
)
const menuMouse = useTermMenuMouse({
  host: () => host.value ?? null,
  term: () => term,
  menu: () => termMenu.menu.value,
  click: termMenu.click,
})
watch(() => termMenu.menu.value, menuMouse.sync)
onBeforeUnmount(() => {
  menuMouse.dispose()
  termMenu.dispose()
  unregisterMenu()
})
const checks = useChecksStore()
const scanNotice = ref<string | null>(null)
let scanTimer: ReturnType<typeof setTimeout> | null = null
/** One-off read of the scrollback on click (no monitoring); results feed FilePane problems as source `terminal`. */
function scanErrors(): void {
  if (!term) return
  const scan = parseTerminalProblems(bufferLines(term.buffer.active), cwd.value ?? null)
  checks.setTerminal(props.pane.key, t('workspace.termScan.source', { name: props.pane.title || shortPath(cwd.value ?? '') || t('workspace.preset.shell') }), scan.errors)
  const n = scan.errors.length
  showNotice(n ? t('workspace.termScan.found', { n, files: t('workspace.termScan.files', { n: scan.files }, scan.files) }, n) : t('workspace.termScan.none'))
}
function showNotice(text: string): void {
  scanNotice.value = text
  if (scanTimer) clearTimeout(scanTimer)
  scanTimer = setTimeout(() => (scanNotice.value = null), 4000)
}
onBeforeUnmount(() => scanTimer && clearTimeout(scanTimer))

const marks = useShellMarks({
  paneKey: () => props.pane.key,
  paneTitle: () => props.pane.title || shortPath(cwd.value ?? '') || t('workspace.preset.shell'),
  cwd: () => cwd.value,
  running: () => props.pane.state === 'running',
  visible: () => props.visible,
  focused: () => props.focused,
  write: (data) => conn?.write(data),
  notice: showNotice,
  finished: onCommandFinished,
})
const bookmarks = useTermBookmarks({ notice: showNotice, mac: isMac })
const { active: marksOn, menu: markMenu, entries: markEntries } = marks
const histProject = computed(() => props.pane.project || cwd.value || '')
const historyOpen = ref(false)
const historyItems = ref<CmdEntry[]>([])
const historyLoading = ref(false)
const canHistory = computed(() => hasCmdHistory() && isShell.value && marksOn.value && props.pane.state === 'running' && !!histProject.value)

function onCommandFinished(done: FinishedCommand): void {
  if (devBindTimer !== null || devGate.endedBy(done.id)) resetDev()
  if (done.exitCode !== 0 && marksOn.value && done.line >= 0) bookmarks.addError(done.line, done.exitCode)
  if (!done.trusted || !hasCmdHistory() || !histProject.value) return
  void addCmdHistory(histProject.value, { cmd: done.command, code: done.exitCode, ms: done.elapsedMs, at: done.endedAt })
}

async function openHistory(): Promise<void> {
  if (!canHistory.value) return
  const project = histProject.value
  historyOpen.value = true
  historyLoading.value = true
  const items = await loadCmdHistory(project)
  if (historyOpen.value && project === histProject.value) historyItems.value = items
  historyLoading.value = false
}

function closeHistory(): void {
  if (!historyOpen.value) return
  historyOpen.value = false
  term?.focus()
}

function pickHistory(entry: CmdEntry, shift: boolean): void {
  const mode = pickMode(shift, entry, marks.atPrompt() && props.pane.state === 'running')
  historyOpen.value = false
  claimInput()
  if (mode === 'run') {
    conn?.write(entry.cmd + String.fromCharCode(13))
    term?.scrollToBottom()
  } else term?.paste(term.modes.bracketedPasteMode ? entry.cmd : entry.cmd.split(/\s*[\r\n]+\s*/).join(" "))
  term?.focus()
}

function dropHistory(entry: CmdEntry): void {
  historyItems.value = historyItems.value.filter((e) => e !== entry)
  void removeCmdHistory(histProject.value, entry)
}
const shellDone = useShellDone({ paneKey: () => props.pane.key, paneTitle: () => props.pane.title || shortPath(cwd.value ?? '') || t('workspace.preset.shell'), visible: () => props.visible })

const queues = usePromptQueueStore()
const queueButton = ref<InstanceType<typeof PromptQueueButton> | null>(null)
const queueCount = computed(() => queues.queues[props.pane.key]?.items.length ?? 0)
const canFillQueue = computed(() => props.pane.state === 'running')

/** Prefills the next queued prompt (never Enter); pasted here so a background pane does not take focus. */
async function fillQueued(): Promise<void> {
  if (props.pane.state !== 'running' || !term) return
  await att.idle()
  if (props.pane.state !== 'running' || !term || !conn?.isOpen) return
  const item = queues.take(props.pane.key)
  if (!item) return
  term.paste(item.text)
  const left = queues.get(props.pane.key).items.length
  showNotice(left ? t('workspace.promptQueue.filledLeft', { n: left }, left) : t('workspace.promptQueue.filled'))
}

watch(
  () => session.value?.status,
  (now, prev) => {
    if (isShell.value || props.pane.state !== 'running') return
    if (shouldAutoFill(queues.get(props.pane.key), prev, now)) void fillQueued()
  },
)

const resume = useResumeScheduleStore()
const limitHit = computed(() => (isShell.value ? undefined : session.value?.limitHit))
// A real reply clears the hit (Claude Code continued by itself, or the owner typed): the prefill would only get in the way.
watch(limitHit, (hit) => {
  if (hit || !session.value || !resume.get(props.pane.key)) return
  resume.cancel(props.pane.key)
  showNotice(t('limitResume.cancelledResumed'))
})
watch(
  () => resume.fired,
  (f) => f?.keys.includes(props.pane.key) && showNotice(t('limitResume.filled')),
)

/** The host validates (real, existing folder) and answers with `cwd`; each prompt repeats it, so send changes only. */
function onShellCwd(dir: string | null): void {
  sendPending()
  if (!dir || dir === reportedCwd) return
  reportedCwd = dir
  conn?.reportCwd(dir)
}

// Typed at the first prompt (its cwd report), or after a grace period for shells without the prompt hook.
const PENDING_GRACE_MS = 4000
let pendingTimer: ReturnType<typeof setTimeout> | null = null
function sendPending(): void {
  if (pendingTimer) clearTimeout(pendingTimer)
  pendingTimer = null
  const cmd = props.pane.pendingCommand
  if (!cmd || !conn?.isOpen || props.pane.state !== 'running') return
  workspace.update(props.pane.key, { pendingCommand: undefined, ranCommand: true })
  conn.write(`${cmd}\r`)
}

const PREFILL_FALLBACK_MS = 20_000
const SUBMIT_DELAY_MS = 200
let prefillTimer: ReturnType<typeof setTimeout> | null = null
let prefillLate = false
let prefilling = false

function stopPrefill(): void {
  if (prefillTimer) clearTimeout(prefillTimer)
  prefillTimer = null
}

function armPrefill(): void {
  stopPrefill()
  prefillLate = false
  prefillTimer = setTimeout(() => {
    prefillTimer = null
    prefillLate = true
    void sendPrefill()
  }, PREFILL_FALLBACK_MS)
  void sendPrefill()
}

function prefillReady(): boolean {
  const status = session.value?.status
  if (props.pane.prefillWaiting) return status === 'waiting' && !termMenu.menu.value && !termMenu.busy.value
  return status === 'idle' || (props.pane.prefillNow === true && status === 'working') || (prefillLate && status !== 'working' && status !== 'waiting')
}

// term.paste() re-enters onData; the Compact flow must not read its own prefill as the owner typing.
let selfInput = false

async function sendPrefill(): Promise<void> {
  if (prefilling || !props.pane.prefill || props.pane.state !== 'running' || !prefillReady()) return
  prefilling = true
  await att.idle()
  prefilling = false
  const text = stripControlChars(props.pane.prefill ?? '')
  const submit = props.pane.prefillSubmit
  if (!text || !term || !conn?.isOpen || props.pane.state !== 'running' || !prefillReady()) return
  stopPrefill()
  workspace.update(props.pane.key, { prefill: undefined, prefillSubmit: undefined, prefillNow: undefined, prefillWaiting: undefined, prefillAuto: undefined })
  if (text.startsWith('/') && !text.includes('\n')) conn.write(text)
  else {
    selfInput = true
    term.paste(term.modes.bracketedPasteMode ? text : text.replace(/\s*\n\s*/g, ' '))
    selfInput = false
  }
  if (submit)
    setTimeout(() => {
      if (conn?.isOpen && props.pane.state === 'running') conn.write('\r')
    }, SUBMIT_DELAY_MS)
}

const compactStep = computed(() => {
  const flow = props.pane.compact
  if (!flow) return null
  return flow.step === 'compact' ? 'compact' : flow.missing ? 'missing' : 'writing'
})
const compactLabel = computed(() => (compactStep.value === 'compact' && props.pane.compact?.auto ? 'compactAuto' : compactStep.value))
const COMPACT_TONE = { writing: 'bg-fill text-ink-muted', missing: 'bg-warning-soft text-warning', compact: 'bg-accent-soft text-accent-ink' } as const

const sessionTitle = computed(() => session.value?.alias || session.value?.title || '')

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

const ANSI = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white'] as const

function skyWell(): boolean {
  const sky = cssVar('--ccm-sky-pane')
  if (uiGen.isNext && uiGen.clearTerm) return true
  return sky !== '' && sky !== 'none'
}

function theme(): Terminal['options']['theme'] {
  const ansi: Record<string, string> = {}
  ANSI.forEach((name, i) => {
    ansi[name] = cssVar(`--ccm-ansi-${i}`)
    ansi[`bright${name.charAt(0).toUpperCase()}${name.slice(1)}`] = cssVar(`--ccm-ansi-${i + 8}`)
  })
  const well = cssVar('--ccm-well')
  return {
    ...ansi,
    background: skyWell() && /^#[0-9a-f]{6}$/i.test(well) ? `${well}00` : well,
    foreground: cssVar('--ccm-well-ink'),
    cursor: cssVar('--ccm-accent'),
    cursorAccent: cssVar('--ccm-well'),
    selectionBackground: cssVar('--ccm-well-selection'),
    selectionInactiveBackground: cssVar('--ccm-well-selection'),
  }
}

// WebGL contexts are scarce (browsers cap ~16) and heavy; only the pane on screen holds one.
function setRenderer(on: boolean): void {
  if (!term) return
  if (on && !webgl) {
    try {
      webgl = new WebglAddon()
      webgl.onContextLoss(dropRenderer)
      term.loadAddon(webgl)
    } catch {
      webgl = null
    }
  }
  if (on && webgl) webglPool.show(props.pane.key, dropRenderer)
  else if (!on && webgl) webglPool.hide(props.pane.key)
}

function dropRenderer(): void {
  webglPool.drop(props.pane.key)
  webgl?.dispose()
  webgl = null
}

// xterm measures the cell when the option changes: the face has to be loaded first or it measures the fallback.
async function applyCodeFont(): Promise<void> {
  const id = codeFontId()
  const size = settings.codeFontSize
  const entry = fontEntry('code', id)
  if (entry.family) await document.fonts.load(`${size}px '${entry.family}'`).catch(() => undefined)
  if (!term || id !== codeFontId() || size !== settings.codeFontSize) return
  const family = termFontStack(entry)
  if (term.options.fontFamily === family && term.options.fontSize === size) return
  term.options.fontFamily = family
  term.options.fontSize = size
  term.clearTextureAtlas()
  refit()
}

function refit(): void {
  if (backlog || !props.visible || !fit || !host.value?.offsetWidth || !viewportFits(window.innerWidth, window.innerHeight)) return
  try {
    const dims = fit.proposeDimensions()
    if (term && conn && dims && dims.cols < term.cols && term.buffer.active.type === 'normal') {
      conn.resize(dims.cols, dims.rows)
      if (refitTimer) clearTimeout(refitTimer)
      refitTimer = setTimeout(() => {
        refitTimer = null
        if (!viewportFits(window.innerWidth, window.innerHeight)) return
        try {
          fit?.fit()
        } catch {
          // pane closed meanwhile
        }
      }, 100)
      return
    }
    fit.fit()
  } catch {
    // container not laid out yet
  }
}

// Attachments tray (agent panes): inserts go through this pane's PTY, the prompt is read back from the screen.
const attachments = useAttachmentsStore()
const isWin = desktop?.platform === 'win32'
const canAttach = computed(() => hasAttachments && preset.value !== 'shell')
const attachOpen = computed(() => canAttach.value && !!attachments.open[props.pane.key])
const attachCount = computed(() => attachments.live(props.pane.key).length)
const dropping = ref(false)
let dragDepth = 0

function readScreen(): PaneScreen | null {
  if (!term) return null
  const b = term.buffer.active
  const rows: string[] = []
  const inverse: { x: number; y: number }[] = []
  let cell: IBufferCell | undefined
  for (let y = 0; y < term.rows; y++) {
    const line = b.getLine(b.baseY + y)
    rows.push(line?.translateToString(true) ?? '')
    if (!line || y < term.rows - SCAN_ROWS) continue
    for (let x = 0; x < line.length; x++) {
      cell = line.getCell(x, cell)
      if (cell?.isInverse()) inverse.push({ x, y })
    }
  }
  return { rows, cursor: { x: b.cursorX, y: b.cursorY }, inverse }
}

const offSnapshot = registerSnapshot(props.pane.key, (n) => {
  output.flush()
  const b = term?.buffer.active
  if (!b) return []
  // A margin above the cursor row so trailing blank screen rows can be trimmed.
  const end = b.baseY + b.cursorY + 1
  const out: string[] = []
  for (let y = Math.max(0, end - n - 8); y < end; y++) out.push(b.getLine(y)?.translateToString(true) ?? '')
  return out
})
onBeforeUnmount(offSnapshot)

function dumpTerm(): TermDump | null {
  const b = term?.buffer.active
  if (!term || !b) return null
  const end = b.length
  const start = Math.max(0, end - MAX_ROWS)
  const rows: TermRow[] = []
  for (let y = start; y < end; y++) {
    const line = b.getLine(y)
    const next = y + 1 < end ? b.getLine(y + 1) : undefined
    rows.push({ text: line?.translateToString(!next?.isWrapped) ?? '', wrapped: !!line?.isWrapped })
  }
  return { rows, firstRow: start, cols: term.cols }
}

function afterFlush(fn: () => void): void {
  let done = false
  let timer: ReturnType<typeof setTimeout> | null = null
  const run = (): void => {
    if (done) return
    done = true
    if (timer) clearTimeout(timer)
    fn()
  }
  timer = setTimeout(run, 1500)
  output.flush(run)
}

async function revealMatch(target: RevealTarget): Promise<void> {
  await nextTick()
  afterFlush(() => {
    if (!term || target.row >= term.buffer.active.length) return
    term.scrollToLine(Math.max(0, target.row - Math.floor(term.rows / 2)))
    term.select(target.col, target.row, target.len)
    findSeed.value = { query: target.query, caseSensitive: target.caseSensitive, regex: target.regex, n: (findSeed.value?.n ?? 0) + 1 }
    findOpen.value = true
  })
}

const offTermSource = registerTermSource(props.pane.key, {
  read: () => new Promise((resolve) => (term ? afterFlush(() => resolve(dumpTerm())) : resolve(null))),
  reveal: (target) => void revealMatch(target),
})
onBeforeUnmount(offTermSource)

const att = usePaneAttachments({
  paneKey: () => props.pane.key,
  preset: () => preset.value,
  cwd: () => cwd.value,
  running: () => props.pane.state === 'running',
  visible: () => props.visible,
  write: (data) => conn?.write(data),
  paste: (text) => {
    term?.paste(text)
    term?.focus()
  },
  screen: readScreen,
  appCursorKeys: () => !!term?.modes.applicationCursorKeysMode,
  win: isWin,
  clipboardPixels: async () => {
    const blob = await clipboardImageBlob()
    const img = blob ? await imageData(blob) : null
    return img && { thumb: img.thumb, view: img.view }
  },
})

watch(
  () => props.visible,
  (shown) => shown && att.schedule(),
)

/** Files from the OS, or paths dragged out of the Explorer. */
const canDrop = (e: DragEvent): boolean => !!e.dataTransfer?.types.includes('Files') || hasDragPath(e.dataTransfer)

function onDragEnter(e: DragEvent): void {
  if (!hasAttachments || !canDrop(e) || props.pane.state !== 'running') return
  e.preventDefault()
  dragDepth++
  dropping.value = true
}

function onDragOver(e: DragEvent): void {
  if (!dropping.value) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
}

function onDragLeave(): void {
  if (dragDepth && --dragDepth === 0) dropping.value = false
}

/** Files carry a path (desktop); images dragged out of a web page have none and travel as pixels. */
async function onDrop(e: DragEvent): Promise<void> {
  if (!dropping.value) return
  e.preventDefault()
  dragDepth = 0
  dropping.value = false
  workspace.activate(props.pane.key)
  const items: NewAttachment[] = []
  const keep = (it: NewAttachment): boolean => it.kind === 'file' || att.imagesSupported.value || !att.agent.value
  const paths = dragPaths(e.dataTransfer)
  for (const path of paths) {
    const it = fromPath(path, 'drop')
    if (keep(it)) items.push(it)
  }
  const files = paths.length ? [] : Array.from(e.dataTransfer?.files ?? [])
  const filePaths = await pathsForFiles(files)
  for (const [i, file] of files.entries()) {
    const path = filePaths[i]
    if (path) {
      const it = fromPath(path, 'drop')
      if (keep(it)) items.push(it)
    } else if (file.type.startsWith('image/') && att.imagesSupported.value) {
      const img = await imageData(file)
      if (img) items.push({ kind: 'image', name: file.name || t('attach.image'), source: 'drop', ...img })
    }
  }
  if (!items.length) return
  // Shell panes: the path is typed at the prompt, no tray.
  attachments.add(props.pane.key, items, att.agent.value ? undefined : true)
  if (!att.agent.value) attachments.toggle(props.pane.key, false)
}

async function paste(textOnly: boolean): Promise<void> {
  claimInput()
  // Agents read clipboard images themselves on their paste key (Alt+V on Windows for Claude Code, §11p).
  const key = textOnly ? null : imagePasteKey(preset.value, isWin)
  const blob = key ? await clipboardImageBlob() : null
  if (key && blob) {
    const img = await imageData(blob)
    att.notePaste({ kind: 'image', name: t('attach.clipboard'), source: 'paste', thumb: img?.thumb, view: img?.view })
    conn?.write(key)
    return
  }
  try {
    const text = await navigator.clipboard.readText()
    if (text) term?.paste(text)
  } catch {
    // clipboard permission denied or empty
  }
}

const MODIFIER_KEYS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'AltGraph', 'Dead'])

function claimInput(): void {
  if (conn?.claimIfNeeded()) remote.setSized(props.pane.key, false)
}

onBeforeUnmount(() => remote.setSized(props.pane.key, false))

function onKey(e: KeyboardEvent): boolean {
  if (e.type !== 'keydown') return true
  if (parked.value) {
    if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) wake()
    return false
  }
  if (!MODIFIER_KEYS.has(e.key)) claimInput()
  if (isShell.value && marksOn.value) {
    const markKey = markKeyAction(e, isMac)
    if (markKey) {
      e.preventDefault()
      if (markKey.t === 'toggle') bookmarks.toggle()
      else bookmarks.jump(markKey.dir)
      return false
    }
    if (canHistory.value && historyKey(e, isMac)) {
      e.preventDefault()
      void openHistory()
      return false
    }
  }
  const newline = newlineFor(e, preset.value)
  if (newline) {
    e.preventDefault()
    conn?.write(newline)
    return false
  }
  // Plain Ctrl+F stays with the shell (readline forward-char).
  if (e.code === 'KeyF' && !e.altKey && (isMac ? e.metaKey && !e.ctrlKey && !e.shiftKey : e.ctrlKey && e.shiftKey && !e.metaKey)) {
    e.preventDefault()
    void openFind()
    return false
  }
  if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) att.noteSubmit()
  if (marksOn.value && (e.key === 'ArrowUp' || e.key === 'ArrowDown') && !e.altKey && !e.shiftKey && (isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey)) {
    if (marks.jump(e.key === 'ArrowUp' ? -1 : 1)) {
      e.preventDefault()
      return false
    }
  }
  // ⌘V is image-aware too (xterm's own paste only knows text).
  if (isMac && e.metaKey && !e.ctrlKey && !e.altKey && e.key.toLowerCase() === 'v') {
    e.preventDefault()
    void paste(e.shiftKey)
    return false
  }
  const key = e.key.toLowerCase()
  if (key === 'c' && !e.altKey && (isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey) && term?.hasSelection()) {
    e.preventDefault()
    copySelection()
    return false
  }
  if (isMac) return true
  if (e.ctrlKey && !e.altKey && key === 'v') {
    e.preventDefault()
    void paste(e.shiftKey)
    return false
  }
  return true
}

function onMessage(msg: PtyServerMessage): void {
  const key = props.pane.key
  switch (msg.t) {
    case 'ready':
      // Only folders the user picked go to the recent list; the home default is always one click away.
      if (props.pane.cwd && !msg.replay) workspace.remember(msg.cwd)
      workspace.update(key, { sessionId: msg.id, pid: msg.pid || undefined, cwd: msg.cwd, state: 'running', error: undefined })
      marks.attach(msg.si, msg.replay)
      ticket?.release()
      if (term) conn?.resize(term.cols, term.rows)
      if (props.pane.pendingCommand && !msg.replay) pendingTimer = setTimeout(sendPending, PENDING_GRACE_MS)
      if (!msg.replay && props.pane.openTicket) workspace.update(key, { openTicket: undefined })
      if (!msg.replay && props.pane.prefill) armPrefill()
      if (props.visible && props.focused) term?.focus()
      break
    case 'pid':
      workspace.update(key, { pid: msg.pid })
      break
    case 'resized':
      remote.setSized(key, true)
      break
    case 'branch':
      workspace.update(key, { branch: msg.branch ?? undefined })
      break
    case 'cwd':
      if (msg.cwd) workspace.update(key, { liveCwd: msg.cwd })
      break
    case 'cmd':
      shellDone.onCmd(msg)
      break
    case 'exit':
      stopBoot()
      resetDev()
      ticket?.release(false)
      workspace.update(key, { state: 'exited', exitCode: msg.code })
      break
    case 'gone':
      stopBoot()
      // A new app run has a new PTY host: a pane restored from a previous run has nothing to attach to.
      if (restoredOnMount) workspace.remove(key)
      else workspace.update(key, { state: 'gone', error: t('workspace.state.gone') })
      break
    case 'error':
      stopBoot()
      ticket?.release(false)
      if (msg.message === 'cwd' && props.pane.cwd) workspace.forget(props.pane.cwd)
      workspace.update(key, { state: 'error', error: t(`workspace.error.${msg.message}`) })
      break
  }
}

async function connect(start: PtyStart): Promise<void> {
  const own = ticket
  conn?.close()
  output.clear()
  if (shouldBoot(start)) startBoot()
  else stopBoot()
  conn = new PtyConnection({
    onOutput: (bytes) => {
      output.push(bytes)
      touch()
      feedDev(bytes)
      checkBoot()
    },
    onMessage,
    onClose: () => {
      stopBoot()
      own?.release(false)
      if (props.pane.state === 'running' || props.pane.state === 'starting')
        workspace.update(props.pane.key, { state: 'error', error: t('workspace.state.disconnected') })
    },
  })
  try {
    await conn.open(start, term?.cols ?? 80, term?.rows ?? 24)
  } catch {
    own?.release(false)
    workspace.update(props.pane.key, { state: 'error', error: t('workspace.state.unavailable') })
  }
}

function killSession(): void {
  const id = props.pane.sessionId
  if (conn?.isOpen) conn.kill()
  else if (id) void PtyConnection.killById(id)
}

function restart(keepSession = false): void {
  const kept = keepSession && props.pane.preset === 'claude' ? resumeIdOf(props.pane) : undefined
  killSession()
  restoredOnMount = false
  marks.reset()
  bookmarks.clear()
  historyOpen.value = false
  output.clear()
  term?.reset()
  resetDev()
  oscLinks.length = 0
  reportedCwd = null
  // A new conversation: reusing the last minted id would hit claude's 'session id already in use' if that one ran.
  // A shell running claude / codex by hand restarts as that agent (resumed once it took a turn), else stays a shell.
  const again = props.pane.preset === 'shell' ? reopenAs(props.pane) : null
  const asAgent = again && again.preset !== 'shell' ? { preset: again.preset, cwd: again.cwd, resume: again.resume, agentSessionId: again.resume } : null
  const next = kept ? { ...props.pane, resume: kept } : (asAgent ?? props.pane)
  const minted = next.preset === 'claude' && !next.resume && !props.pane.openTicket ? crypto.randomUUID() : undefined
  const keep = kept ? { resume: kept, agentSessionId: kept, mintedSession: undefined } : {}
  workspace.update(props.pane.key, { ...asAgent, ...keep, state: 'starting', sessionId: undefined, pid: undefined, terminalId: undefined, agent: undefined, branch: undefined, liveCwd: undefined, exitCode: undefined, error: undefined, ranCommand: undefined, untouchedSession: undefined, ...(minted ? { agentSessionId: minted, mintedSession: minted } : {}) })
  void spawn()
}

async function spawn(): Promise<void> {
  ticket?.release(false)
  if (!presetEnabled(props.pane.preset, hasDeepSeek)) {
    workspace.update(props.pane.key, { state: 'error', error: t('workspace.error.preset') })
    return
  }
  const mine = spawnQueue.enter(props.visible)
  ticket = mine
  queued.value = true
  const ok = await mine.started
  if (ticket !== mine) return
  queued.value = false
  if (ok) void connect(spawnStart())
}

function spawnStart(): PtyStart {
  const shell = props.pane.preset === 'shell'
  return { spawn: { preset: props.pane.preset, cwd: props.pane.cwd, resume: props.pane.resume, open: props.pane.openTicket, session: props.pane.mintedSession, integrate: shell && settings.shellIntegration, shim: shell && settings.agentShim, launch: props.pane.launch } }
}

function wake(): void {
  workspace.wake([props.pane.key])
  term?.focus()
}

function reconnect(): void {
  if (!props.pane.sessionId) return restart()
  restoredOnMount = false
  marks.reset()
  bookmarks.clear()
  historyOpen.value = false
  output.clear()
  term?.reset()
  workspace.update(props.pane.key, { state: 'starting', error: undefined })
  void connect({ attach: props.pane.sessionId })
}

function close(): void {
  emit('close')
}

onMounted(() => {
  term = new Terminal({
    allowProposedApi: true,
    cursorBlink: false,
    cursorStyle: 'bar',
    // The chosen face may still be loading; applyCodeFont swaps it in once it is.
    fontFamily: termFontStack(fontEntry('code', 'system')),
    fontSize: settings.codeFontSize,
    lineHeight: 1.25,
    scrollback: TERM_SCROLLBACK,
    smoothScrollDuration: 0,
    // CLIs paint with ANSI black/dim colours that vanish on the dark well; xterm lifts them per cell.
    minimumContrastRatio: 4.5,
    // Apps that track the mouse (full-screen TUIs) get drags; Shift-drag (xterm default) / Option-drag on macOS still selects.
    macOptionClickForcesSelection: true,
    allowTransparency: skyWell(),
    theme: theme(),
    // OSC 8 links (Claude Code prints sign-in URLs this way): xterm's default handler opens about:blank, which the app denies.
    linkHandler: {
      activate: (event, uri) => {
        if (!(isMac ? event.metaKey : event.ctrlKey) || !/^https?:\/\//i.test(uri)) return
        emit('openLink', { kind: 'url', url: uri }, {})
      },
      hover: (_event, uri) => hoverLink(uri),
      leave: leaveLink,
    },
  })
  fit = new FitAddon()
  term.loadAddon(fit)
  // Graphemes, not unicode11: Vietnamese NFD output from Claude otherwise drifts the cursor.
  term.loadAddon(new UnicodeGraphemesAddon())
  term.unicode.activeVersion = '15-graphemes'
  term.open(host.value!)
  term.attachCustomKeyEventHandler(onKey)
  installReplacementIme(term)
  term.onWriteParsed(() => att.schedule())
  termMenu.attach(term)
  menuMouse.install()
  term.parser.registerOscHandler(7, (data) => (onShellCwd(parseOsc7(data)), true))
  // Not consumed (false): xterm still turns the sequence into a link.
  term.parser.registerOscHandler(8, (data) => (rememberLink(oscLinkUri(data)), false))
  term.parser.registerOscHandler(52, onOsc52)
  term.parser.registerOscHandler(9, (data) => {
    const dir = parseOsc9(data)
    if (dir) onShellCwd(dir)
    return !!dir
  })
  marks.install(term)
  bookmarks.install(term)
  linkProvider = term.registerLinkProvider(
    createFileLinkProvider(term, {
      cwd: () => cwd.value,
      isMac,
      onOpen: (target, opts) => emit('openLink', target, opts),
      onHover: (target) => hoverLink(target.kind === 'url' ? target.url : target.path),
      onLeave: leaveLink,
    }),
  )
  term.onData((data) => {
    conn?.write(data)
    touch()
    if (!selfInput && !data.startsWith('\x1b') && (props.pane.compact?.auto || props.pane.prefillAuto)) compactFlow.stopAuto(props.pane.key)
    if (preset.value === 'shell' && !props.pane.ranCommand && data.includes('\r')) workspace.update(props.pane.key, { ranCommand: true })
  })
  term.onBinary((data) => conn?.writeBinary(data))
  term.onResize(({ cols, rows }) => conn?.resize(cols, rows))
  term.textarea?.addEventListener('focus', () => {
    if (term) term.options.cursorBlink = true
    workspace.activate(props.pane.key)
  })
  term.textarea?.addEventListener('blur', () => {
    if (term) term.options.cursorBlink = false
  })
  host.value!.addEventListener('contextmenu', onContextMenu)
  host.value!.addEventListener('copy', onCopy, true)
  setRenderer(props.visible)
  refit()
  void applyCodeFont()
  search = new SearchAddon({ highlightLimit: 1000 })
  term.loadAddon(search)
  searchAddon.value = search
  resizeObserver = new ResizeObserver(() => {
    if (refitTimer) clearTimeout(refitTimer)
    refitTimer = setTimeout(() => {
      refitTimer = null
      refit()
    }, 150)
  })
  resizeObserver.observe(host.value!)
  if (rootEl.value) {
    headObserver = new ResizeObserver((entries) => {
      const w = Math.round(entries[0]?.contentRect.width ?? 0)
      if (w > 0 && w !== headWidth.value) headWidth.value = w
    })
    headObserver.observe(rootEl.value)
  }
  window.addEventListener('resize', dropMenus)

  output.setVisible(props.visible)
  restoredOnMount = !!props.pane.sessionId
  if (props.pane.state === 'parked') return
  if (props.pane.sessionId) void connect({ attach: props.pane.sessionId })
  else void spawn()
})

watch(
  () => props.pane.state,
  (state, was) => {
    if (was === 'parked' && state === 'starting' && !conn) void spawn()
  },
)

watch(
  () => props.visible,
  async (visible) => {
    setRenderer(visible)
    if (visible) ticket?.promote()
    if (!visible) {
      output.setVisible(false)
      return dropMenus()
    }
    const written = showBacklog()
    await nextTick()
    if (props.focused) term?.focus()
    await written
    if (!props.visible) return
    refit()
  },
)

watch(
  () => workspace.restartRequest,
  (req) => {
    if (!req || req.paneKey !== props.pane.key || props.pane.state !== 'running') return
    restart(true)
  },
)

// Browser → agent: lands in the input line only; the user presses Enter.
watch(
  () => workspace.insertRequest,
  async (req) => {
    if (!req || req.paneKey !== props.pane.key || props.pane.state !== 'running') return
    await att.idle()
    if (props.pane.state !== 'running') return
    if (req.image) {
      const key = imagePasteKey(preset.value, isWin)
      if (key) {
        conn?.write(key)
        att.notePaste({ kind: 'image', name: t('attach.screenshot'), source: 'paste' })
      }
    } else term?.paste(req.text)
    term?.focus()
  },
)

watch(
  () => [props.pane.prefill, props.pane.state, session.value?.status, !!termMenu.menu.value, termMenu.busy.value] as const,
  () => void sendPrefill(),
)

watch(
  () => [props.pane.typeKeys, termMenu.busy.value] as const,
  ([keys, menuBusy]) => {
    if (!keys || menuBusy || !conn?.isOpen || props.pane.state !== 'running') return
    conn.write(keys)
    workspace.update(props.pane.key, { typeKeys: undefined })
    termMenu.schedule()
  },
  { immediate: true },
)

watch(
  () => props.pane.state,
  (state) => {
    if (state !== 'exited' && state !== 'error' && state !== 'gone') return
    stopPrefill()
    if (props.pane.prefill || props.pane.openTicket) workspace.update(props.pane.key, { prefill: undefined, openTicket: undefined })
  },
)

watch(
  () => props.focused,
  (focused) => {
    if (focused && props.visible) term?.focus()
  },
)

watch(
  () => [settings.theme, settings.stars, settings.starsPanels, uiGen.isNext, uiGen.clearTerm, uiGen.nextTheme, uiGen.dark, uiGen.customScheme, uiGen.inlineVars] as const,
  async () => {
    await nextTick()
    if (!term) return
    term.options.allowTransparency = skyWell()
    term.options.theme = theme()
  },
)

watch(() => [settings.codeFont, settings.codeFontSize, uiGen.isNext] as const, () => void applyCodeFont())

// Leaving the view only detaches; a pane gone from the store was closed, so its process goes too.
onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  if (refitTimer) clearTimeout(refitTimer)
  if (pendingTimer) clearTimeout(pendingTimer)
  ticket?.release(false)
  stopPrefill()
  window.removeEventListener('resize', dropMenus)
  linkProvider?.dispose()
  if (!workspace.panes.some((p) => p.key === props.pane.key)) {
    killSession()
    attachments.drop(props.pane.key)
  }
  headObserver?.disconnect()
  conn?.close()
  marks.dispose()
  bookmarks.dispose()
  output.dispose()
  backlog?.()
  dropRenderer()
  term?.dispose()
})
</script>

<template>
  <section
    ref="rootEl"
    class="@container flex min-h-0 min-w-0 flex-col transition-opacity duration-(--ccm-dur-base) ease-out-quint"
    :class="[`ccm-agent-${preset}`, isNext ? 'ccm-next-pane gap-0' : 'gap-1.5', multi && !focused ? 'ccm-pane-idle' : '', isNext ? 'ccm-pane-ring' : '', isNext && multi && focused ? 'ccm-pane-active' : '']"
    :data-next-pane-card="isNext || undefined"
    :data-pane="pane.key"
    :style="visible ? `view-transition-name: ccm-pane-${pane.key}; view-transition-class: ccm-pane` : undefined"
    :aria-label="t(`workspace.preset.${preset}`)"
    @pointerdown="onPointerDown"
  >
    <Teleport :to="barSlot ?? 'body'" :disabled="!inBar">
    <component
      :is="isNext ? PaneHeader : 'header'"
      v-bind="isNext ? { active: multi && focused } : {}"
      class="relative select-none"
      :class="[isNext ? 'ccm-next-head group/head' : 'flex h-7 min-w-0 items-center gap-1.5 px-1 text-xs @max-[420px]:gap-1', multi ? 'cursor-grab' : '', inBar ? 'max-w-full' : '', !isNext && multi && focused ? 'ccm-pane-head-on' : '']"
      :title="multi ? t('workspace.drag.paneHint', { key: isMac ? '⌘' : 'Ctrl' }) : inBar ? headTip : undefined"
      :data-pane-grip="multi || undefined"
      :data-pane-of="pane.key"
      :data-tour="focused ? 'ws-pane' : undefined"
      data-pane-head
      @dblclick="onHeadDblClick"
    >
      <span v-if="peerSweep" :key="peerSweep" class="ccm-peer-sweep-x absolute inset-x-1 top-0 h-0.5" aria-hidden="true" data-pane-peer-sweep />
      <template v-if="!inBar">
        <span class="grid size-[18px] shrink-0 place-items-center rounded-inner bg-[color-mix(in_srgb,var(--ccm-pane-accent)_16%,transparent)]" data-pane-mark aria-hidden="true">
          <PresetIcon :preset="preset" :size="12" class="shrink-0" />
        </span>
        <span
          class="min-w-0 shrink-0 truncate font-semibold transition-colors duration-(--ccm-dur-fast) ease-out-quint @max-[420px]:sr-only"
          :class="multi && !focused ? 'text-ink-muted' : 'text-ink'"
          :title="headTip"
          data-pane-name
          >{{ t(`workspace.preset.${preset}`) }}</span
        >
      </template>
      <span
        v-if="ownProject"
        class="inline-flex min-w-0 shrink items-center gap-1 text-2xs font-medium text-accent-ink @max-[360px]:hidden"
        :title="shown(`${t('splitV97.paneProject', { name: ownProject })}\n${pane.project}`)"
        data-pane-project
        data-pane-chip
      >
        <FolderOpen :size="11" class="shrink-0" aria-hidden="true" /><span class="truncate">{{ ownProject }}</span>
      </span>
      <Transition name="ccm-fade">
        <span
          v-if="diff"
          class="inline-flex shrink-0 items-center gap-1 text-2xs font-medium tabular-nums @max-[480px]:hidden"
          :class="isNext ? REVEAL : ''"
          :title="t('workspace.activity.diffTitle', { files: diff.files, ins: diff.insertions, del: diff.deletions }, diff.files)"
          data-pane-diff
          data-pane-chip
        >
          <span class="text-diff-add">+{{ diff.insertions }}</span>
          <span class="text-diff-del">−{{ diff.deletions }}</span>
        </span>
      </Transition>
      <Teleport to="body">
        <Transition name="ccm-menu" @before-leave="retireOverlay">
          <WorktreeMenu v-if="wtAnchor && cwd" :cwd="cwd" :anchor="wtAnchor" :trigger="wtButton ?? more?.button" @close="closeWorktrees" />
        </Transition>
      </Teleport>
      <div class="ml-auto flex min-w-0 items-center gap-1">
        <span v-if="isNext" class="inline-flex min-w-0 items-center empty:hidden" :class="REVEAL" data-pane-reveal><RemotePhoneChip :pane-key="pane.key" /></span>
        <RemotePhoneChip v-else :pane-key="pane.key" />
        <ClashChip v-if="isNext && pane.terminalId" :terminal-id="pane.terminalId" :can-worktree="!!cwd && pane.state === 'running'" @worktree="wtAnchor || toggleWorktrees()" />
        <SubagentOrbit v-if="isClaude" :agents="orbitAgents" :flat="isNext" data-pane-chip />
        <ContextGauge v-if="context && gaugeShown" class="w-24 shrink-0 @max-[600px]:hidden" :class="inBar ? 'max-lg:hidden' : ''" :tokens="context.tokens" :window="context.window" :pct="context.pct" compact data-pane-chip />
        <ContextTokens v-if="!isNext" class="@max-[600px]:hidden" :class="inBar ? 'max-lg:hidden' : ''" :tokens="mainAgent?.contextTokens" data-pane-chip />
        <button
          v-if="cwd && pane.state === 'running' && !demoted.includes('worktree')"
          ref="wtButton"
          type="button"
          class="inline-flex h-5 shrink-[10] cursor-pointer items-center gap-1 rounded-inner bg-[var(--ccm-control)] px-1.5 font-mono text-2xs text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink aria-expanded:bg-fill aria-expanded:text-ink"
          :title="shown([branch ? t('workspace.paneHead.branch', { branch }) : t('workspace.wt.title'), cwd].join('\n'))"
          aria-haspopup="dialog"
          :aria-expanded="!!wtAnchor"
          data-pane-worktrees
          @click="toggleWorktrees()"
        >
          <GitBranch :size="12" class="shrink-0" aria-hidden="true" />
          <span v-if="branch" class="max-w-[16ch] min-w-[4ch] truncate @max-[440px]:sr-only @max-[440px]:min-w-0">{{ branch }}</span>
          <span v-else class="sr-only">{{ t('workspace.wt.title') }}</span>
        </button>
        <button
          v-if="canAttach && pinned.has('attach')"
          type="button"
          class="relative inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink aria-pressed:bg-fill aria-pressed:text-ink"
          :class="isNext ? `${REVEAL} aria-pressed:opacity-100` : ''"
          :title="attachLabel"
          :aria-label="attachLabel"
          :aria-pressed="attachOpen"
          data-no-drag
          data-pane-attach
          @click="attachments.toggle(pane.key)"
        >
          <Paperclip :size="12" aria-hidden="true" />
          <span
            v-if="attachCount"
            class="absolute -top-1 -right-1 grid h-3 min-w-3 place-items-center rounded-full bg-accent px-0.5 text-[9px] leading-none font-bold text-on-accent tabular-nums"
            aria-hidden="true"
            >{{ attachCount }}</span
          >
        </button>
        <button
          v-if="canChat && pinned.has('chat')"
          type="button"
          class="inline-flex h-5 shrink-0 cursor-pointer items-center gap-1 rounded-inner px-1 text-2xs font-medium text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink aria-pressed:bg-fill aria-pressed:text-ink"
          :class="isNext ? `${REVEAL} aria-pressed:opacity-100` : ''"
          :title="chatLabel"
          :aria-label="chatLabel"
          :aria-pressed="chatOn"
          data-no-drag
          data-pane-chat-toggle
          @click="toggleChat"
        >
          <SquareTerminal v-if="chatOn" :size="12" class="shrink-0" aria-hidden="true" />
          <MessageSquareText v-else :size="12" class="shrink-0" aria-hidden="true" />
          <span class="@max-[560px]:hidden" aria-hidden="true">{{ chatOn ? t('chatView.terminal') : t('chatView.chat') }}</span>
        </button>
        <button
          v-if="canTurn && pinned.has('turn')"
          type="button"
          class="relative inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink"
          :class="isNext ? REVEAL : ''"
          :title="turnLabel"
          :aria-label="turnLabel"
          data-no-drag
          data-pane-turn
          @click="openTurn"
        >
          <GitCompareArrows :size="12" aria-hidden="true" />
          <span
            v-if="turnCount"
            class="absolute -top-1 -right-1 grid h-3 min-w-3 place-items-center rounded-full bg-accent px-0.5 text-[9px] leading-none font-bold text-on-accent tabular-nums"
            aria-hidden="true"
            data-pane-turn-count
            >{{ turnCount }}</span
          >
        </button>
        <button
          v-if="canTools && pinned.has('tools')"
          type="button"
          class="inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink"
          :class="isNext ? REVEAL : ''"
          :title="t('sessionTools.menu')"
          :aria-label="t('sessionTools.menu')"
          data-no-drag
          data-pane-tools
          @click="openTools"
        >
          <Blocks :size="12" aria-hidden="true" />
        </button>
        <span class="grid min-w-6 justify-items-end empty:hidden *:[grid-area:1/1] @max-[300px]:hidden" data-more="compact" data-pane-chip>
          <Transition name="ccm-fade">
            <button
              v-if="compactSuggested"
              type="button"
              class="inline-flex h-5 max-w-full min-w-6 cursor-pointer items-center gap-1 rounded-inner bg-warning-soft px-1.5 text-2xs font-medium whitespace-nowrap text-warning transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:text-ink"
              :title="t('workspace.paneHead.compact.title', { pct: compactPct })"
              :aria-label="t('workspace.paneHead.compact.label', { pct: compactPct })"
              data-compact-suggest
              @click="compactFlow.start(pane.key)"
            >
              <TriangleAlert :size="12" class="shrink-0" aria-hidden="true" />
              <span class="min-w-0 truncate @max-[460px]:hidden">{{ t('workspace.paneHead.compact.label', { pct: compactPct }) }}</span>
            </button>
          </Transition>
        </span>
        <span class="grid min-w-6 justify-items-end empty:hidden *:[grid-area:1/1]" data-pane-chip>
          <Transition name="ccm-fade">
            <span
              v-if="compactStep && pane.state === 'running'"
              :key="compactStep"
              class="inline-flex h-5 max-w-full min-w-6 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium whitespace-nowrap"
              :class="COMPACT_TONE[compactStep]"
              :title="pane.compact?.file"
              :data-compact-step="compactStep"
            >
              <LoaderCircle v-if="compactStep === 'writing'" :size="12" class="ccm-spin shrink-0" aria-hidden="true" />
              <FileText v-else :size="12" class="shrink-0" aria-hidden="true" />
              <span class="min-w-0 truncate @max-[460px]:sr-only">{{ t(`workspace.compact.step.${compactLabel}`) }}</span>
            </span>
          </Transition>
        </span>
        <!-- One grid cell: the old chip fades out under the new one instead of the header reflowing twice. -->
        <span class="grid min-w-6 justify-items-end empty:hidden *:[grid-area:1/1]" data-pane-chip>
          <Transition name="ccm-fade">
            <span
              v-if="sessionChip"
              :key="chipKey"
              class="inline-flex h-5 max-w-full min-w-6 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium whitespace-nowrap"
              :class="[sessionChip.meta.text, sessionChip.meta.chip]"
              :title="chipTitle"
              :data-pane-status="sessionChip.status"
              :data-subagents="sessionChip.subagents || undefined"
            >
              <span class="relative inline-flex shrink-0">
                <StatusIcon :status="sessionChip.badge" :size="12" />
                <Transition name="ccm-fade">
                  <span v-if="peerBadge && peer" class="absolute -top-1 -right-1 inline-flex rounded-full bg-surface p-px" aria-hidden="true" :data-pane-peer-badge="peer.dir">
                    <component :is="peer.glyph" :size="8" :class="peer.tone" />
                  </span>
                  <span v-else-if="peerBadge && coordinating" class="absolute -top-1 -right-1 inline-flex rounded-full bg-surface p-px" aria-hidden="true" :data-pane-coordinating-badge="coordinating.n">
                    <Network :size="8" class="text-ink-faint" />
                  </span>
                </Transition>
              </span>
              <span class="min-w-0 truncate" :class="statusWordShown(sessionChip.state) ? '@max-[460px]:sr-only' : 'sr-only'">{{ sessionChip.label }}</span>
              <span v-if="peerBadge && peer" class="sr-only">, {{ peer.label }}</span>
              <span v-else-if="peerBadge && coordinating" class="sr-only">, {{ coordinating.label }}</span>
              <span v-if="activityText" class="max-w-[22ch] min-w-0 truncate font-normal text-ink-muted tabular-nums @max-[620px]:hidden" data-pane-activity>- {{ activityText }}</span>
              <span v-if="isNext && workflowText" class="max-w-[16ch] min-w-0 truncate font-normal text-ink-muted tabular-nums @max-[520px]:hidden" data-agent-status="merged">- {{ workflowText }}</span>
            </span>
            <span
              v-else-if="procChip"
              :key="chipKey"
              class="inline-flex h-5 max-w-full min-w-6 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium whitespace-nowrap"
              :class="procChip.cls"
              :title="procChip.title"
              :data-pane-process="procChip.kind"
            >
              <LoaderCircle v-if="!procChip.status" :size="12" class="ccm-spin shrink-0" aria-hidden="true" />
              <StatusIcon v-else :status="procChip.status" :size="12" class="shrink-0" />
              <span class="min-w-0 truncate" :class="procChip.kind === 'exited' || procChip.kind === 'error' ? '@max-[460px]:sr-only' : 'sr-only'">{{ procChip.label }}</span>
            </span>
          </Transition>
        </span>
        <span class="grid min-w-5 shrink-[1000] justify-items-end empty:hidden *:[grid-area:1/1]" data-pane-chip data-pane-peer-cell>
          <Transition name="ccm-fade">
            <button
              v-if="peer && !peerBadge"
              :key="`p:${peer.kind}:${peer.dir}:${peer.text}`"
              type="button"
              class="inline-flex h-5 max-w-full min-w-5 cursor-pointer items-center justify-center gap-1 rounded-inner px-1 text-2xs font-medium whitespace-nowrap text-ink transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:bg-fill disabled:cursor-default disabled:hover:bg-transparent"
              :disabled="!peer.other"
              :title="peerTip"
              :aria-label="peerAria"
              :data-pane-peer="peer.dir"
              :data-peer-kind="peer.kind"
              data-no-drag
              @click="info.goToPeer(peer.other)"
            >
              <component :is="peer.glyph" :size="12" class="shrink-0" :class="peer.tone" aria-hidden="true" />
              <span class="max-w-[17ch] min-w-0 truncate @max-[720px]:hidden" data-pane-peer-text>{{ peer.text }}</span>
            </button>
            <span
              v-else-if="coordinating && !peerBadge"
              key="coordinating"
              role="img"
              class="inline-flex h-5 min-w-5 items-center justify-center text-ink-faint"
              :title="peerTip || coordinating.label"
              :aria-label="coordinating.label"
              :data-pane-coordinating="coordinating.n"
            >
              <Network :size="12" aria-hidden="true" />
            </span>
          </Transition>
        </span>
        <span class="grid min-w-0 shrink justify-items-end empty:hidden *:[grid-area:1/1]" data-pane-chip data-dev-chip-cell>
          <Transition name="ccm-fade">
            <button
              v-if="devChipUrl"
              :key="devChipUrl"
              type="button"
              class="inline-flex h-5 max-w-full min-w-5 cursor-pointer items-center gap-1 rounded-inner bg-accent-soft px-1.5 text-2xs font-medium whitespace-nowrap text-accent-ink transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:text-ink"
              :title="t('workspace.devServer.chip', { host: devUrlLabel(devChipUrl) })"
              :aria-label="t('workspace.devServer.chip', { host: devUrlLabel(devChipUrl) })"
              data-dev-chip
              data-no-drag
              @click="emit('openLink', { kind: 'url', url: devChipUrl }, {})"
            >
              <Globe :size="12" class="shrink-0" aria-hidden="true" />
              <span class="min-w-0 truncate tabular-nums @max-[520px]:sr-only">{{ devUrlLabel(devChipUrl) }}</span>
              <ArrowUpRight :size="12" class="shrink-0" aria-hidden="true" />
            </button>
          </Transition>
        </span>
        <OverlapChip v-if="cwd && preset !== 'shell'" :cwd="cwd" :branch="branch" :compact="isNext" data-pane-chip />
        <AgentStatusChip v-if="workflowStatus && !(isNext && sessionChip)" :status="workflowStatus" data-pane-chip />
        <LimitResumeChip v-if="!isShell && pane.state === 'running'" :pane-key="pane.key" :hit="limitHit" data-pane-chip />
        <span v-if="!isShell" class="hidden"><PromptQueueButton ref="queueButton" :pane-key="pane.key" :can-fill="canFillQueue" @fill="fillQueued" /></span>
        <PaneMore ref="more" class="@max-[300px]:min-w-6 @max-[300px]:px-1" :entries="moreEntries" :label="t('workspace.paneHead.more')" @pick="pickMore" />
        <Transition name="ccm-fade">
          <button
            v-if="zoomed && hiddenCount > 0"
            type="button"
            class="inline-flex h-5 shrink-0 cursor-pointer items-center rounded-inner bg-accent-soft px-1.5 text-2xs font-medium text-accent-ink tabular-nums transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:text-ink @max-[300px]:hidden"
            :title="hiddenLabel"
            :aria-label="hiddenLabel"
            data-pane-hidden
            @click="emit('zoom')"
          >
            <Transition name="ccm-fade">
              <Mail v-if="hiddenNews" :size="10" class="mr-1 shrink-0 text-peer" aria-hidden="true" data-pane-hidden-peer />
            </Transition>
            +{{ hiddenCount }}
          </button>
        </Transition>
        <button
          v-if="multi || zoomed"
          type="button"
          class="ccm-btn-plain shrink-0 @max-[300px]:min-w-6 @max-[300px]:px-1"
          :class="isNext && !zoomed ? REVEAL : ''"
          :aria-pressed="zoomed"
          :title="zoomLabel"
          :aria-label="zoomLabel"
          data-pane-zoom
          @click="emit('zoom')"
        >
          <span class="grid *:[grid-area:1/1]">
            <Transition name="ccm-fade">
              <Minimize2 v-if="zoomed" :size="14" aria-hidden="true" />
              <Maximize2 v-else :size="14" aria-hidden="true" />
            </Transition>
          </span>
        </button>
        <button
          v-if="!demoted.includes('split')"
          ref="splitButton"
          type="button"
          class="ccm-btn-plain shrink-0 aria-expanded:bg-fill @max-[300px]:min-w-6 @max-[300px]:px-1"
          :class="isNext ? `${REVEAL} aria-expanded:opacity-100` : ''"
          :title="t('workspace.splitPane')"
          :aria-label="t('workspace.splitPane')"
          aria-haspopup="menu"
          data-pane-split
          :aria-expanded="!!splitAnchor"
          :data-tour="focused ? 'ws-split' : undefined"
          @click="toggleSplit()"
        >
          <Columns2 :size="14" aria-hidden="true" />
        </button>
        <Teleport to="body">
          <Transition name="ccm-menu" @before-leave="retireOverlay">
            <SplitMenu v-if="splitAnchor" :anchor="splitAnchor" :current="pane.preset" :trigger="splitButton ?? more?.button" @pick="(p) => emit('split', p)" @close="splitAnchor = null" />
          </Transition>
        </Teleport>
        <button
          type="button"
          class="ccm-btn-plain shrink-0 @max-[300px]:min-w-6 @max-[300px]:px-1"
          :title="t('workspace.close')"
          :aria-label="t('workspace.close')"
          data-pane-close
          @click="close"
        >
          <X :size="14" aria-hidden="true" />
        </button>
      </div>
    </component>
    </Teleport>
    <ContextMeter v-if="meterOn && (!isNext || meterOver)":tokens="mainAgent?.contextTokens" :limit="settings.contextWarnTokens" :can-compact="canMeterCompact" @compact="compactFlow.start(pane.key)" />
    <div class="flex min-h-0 flex-1 gap-1.5" data-pane-body @dragenter="onDragEnter" @dragover="onDragOver" @dragleave="onDragLeave" @drop="onDrop">
    <div
      class="ccm-pane-ring ccm-sky-well relative flex min-h-0 min-w-0 flex-1 flex-col rounded-card bg-well shadow-card transition-[outline-color] duration-(--ccm-dur-fast) ease-out-quint"
      :class="!isNext && multi && focused ? 'ccm-pane-active' : ''"
      data-pane-well
      :data-term-clear="(isNext && uiGen.clearTerm) || undefined"
    >
      <!-- Dimmed when the process has ended; the well fades, the terminal text itself never animates. -->
      <!-- Shell command marks sit in the left padding, outside the cell grid. -->
      <div class="relative min-h-0 w-full flex-1 overflow-hidden rounded-card p-2 pr-0.5 transition-opacity duration-(--ccm-dur-slow) ease-out-quint" :class="[ended ? 'opacity-60' : '', marksOn ? 'pl-4' : '']">
        <div ref="host" class="size-full" data-code-zoom :title="linkHint" />
        <div v-if="menuMouse.hover.value" class="pointer-events-none absolute rounded-inner bg-accent/20" :style="{ left: menuMouse.hover.value.left + 'px', top: menuMouse.hover.value.top + 'px', width: menuMouse.hover.value.width + 'px', height: menuMouse.hover.value.height + 'px' }" data-term-menu-hover />
      </div>
      <TermMenuBar v-if="termMenu.menu.value" :class="chatOn && canChat ? 'relative z-12 rounded-b-card bg-well' : ''" :menu="termMenu.menu.value" :busy="termMenu.busy.value" @pick="termMenu.pick" />
      <Transition name="ccm-fade">
        <div v-if="chatOn && canChat" class="absolute inset-0 z-11 overflow-hidden rounded-card bg-well" :class="termMenu.menu.value ? 'rounded-b-none' : ''" data-pane-chat>
          <ChatView :target="chatTarget" :active="visible && chatOn" :agent-label="t(`workspace.preset.${preset}`)" @open-file="(path) => emit('openLink', { kind: 'file', path }, {})" />
        </div>
      </Transition>
      <Transition name="ccm-fade">
        <div v-if="booting" class="absolute inset-0 z-11 flex items-center justify-center rounded-card bg-well p-4" data-pane-boot>
          <div class="flex max-w-80 flex-col items-center gap-3 text-center" role="status" aria-live="polite">
            <span class="ccm-boot-orbit" aria-hidden="true"><span class="ccm-boot-sun" /><span class="ccm-boot-arm"><span class="ccm-boot-planet" /></span></span>
            <p class="text-sm font-medium text-ink tabular-nums">
              {{ bootMs >= BOOT_SLOW_MS ? t('paneV93.boot.slow', { agent: t(`workspace.preset.${preset}`), s: Math.floor(bootMs / 1000) }) : t('paneV93.boot.starting', { agent: t(`workspace.preset.${preset}`) }) }}
            </p>
            <p v-if="bootMs >= BOOT_SLOW_MS" class="text-xs text-ink-muted">{{ t('paneV93.boot.hint') }}</p>
          </div>
        </div>
      </Transition>
      <Transition name="ccm-fade">
        <div v-if="parked" class="absolute inset-0 z-11 flex items-center justify-center rounded-card bg-well p-4" data-pane-parked>
          <div class="flex max-w-80 flex-col items-center gap-2 text-center">
            <Moon :size="20" class="text-ink-faint" aria-hidden="true" />
            <p class="text-sm font-semibold text-ink">{{ t('workspace.parked.title') }}</p>
            <p class="text-xs text-ink-muted">{{ t('workspace.parked.hint') }}</p>
            <button type="button" class="ccm-btn ccm-btn-primary mt-1" data-pane-wake @click="wake">
              <Play :size="14" aria-hidden="true" />{{ pane.resume ? t('workspace.parked.resume') : t('workspace.parked.start') }}
            </button>
          </div>
        </div>
      </Transition>
      <div
        class="pointer-events-none absolute inset-0 z-12 rounded-card bg-well transition-opacity duration-(--ccm-dur-base) ease-out-quint"
        :class="dim ? 'opacity-45' : 'opacity-0'"
        :data-on="dim || undefined"
        data-pane-dim
        aria-hidden="true"
      />
      <Transition name="ccm-fade">
        <p v-if="scanNotice" class="ccm-sheet pointer-events-none absolute bottom-3 left-1/2 z-13 max-w-[90%] -translate-x-1/2 truncate px-3 py-1.5 text-xs text-ink" role="status" data-term-scan-notice>
          {{ scanNotice }}
        </p>
      </Transition>
      <ContextMenu v-if="markMenu" :at="markMenu.at" :entries="markEntries" :label="t('shellMarks.menuLabel')" :trigger="markMenu.trigger" @pick="marks.pick" @close="marks.closeMenu" />
      <CmdHistoryPicker v-if="historyOpen" :items="historyItems" :loading="historyLoading" @pick="pickHistory" @remove="dropHistory" @close="closeHistory" />
      <TermFind v-if="findOpen" ref="find" :addon="searchAddon" :seed="findSeed" class="absolute top-1 right-2 z-10" @close="closeFind" />
      <span
        v-if="pane.attention"
        :key="`${pane.attention}:${visible}`"
        class="ccm-flash pointer-events-none absolute inset-0 rounded-card"
        :class="pane.attention === 'waiting' ? 'text-st-waiting' : 'text-st-done'"
        aria-hidden="true"
      />
      <Transition name="ccm-fade">
        <div
          v-if="dropping"
          class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center rounded-card bg-accent-soft outline-2 -outline-offset-2 outline-focus outline-dashed"
          data-drop-overlay
        >
          <span class="ccm-sheet inline-flex items-center gap-2 px-3 py-2 text-sm font-medium text-ink"><Paperclip :size="15" aria-hidden="true" />{{ t('attach.drop') }}</span>
        </div>
      </Transition>
    </div>
    <AttachmentsPanel
      v-if="attachOpen"
      :pane-key="pane.key"
      :images-supported="att.imagesSupported.value"
      :erase-for="att.eraseFor"
      :remove="att.remove"
      @close="attachments.toggle(pane.key, false)"
      @open="(path) => emit('openLink', { kind: 'file', path }, {})"
    />
    </div>
    <TurnCard v-if="meterOn" :cwd="session?.cwd" :status="session?.status" @open="(path) => emit('openLink', { kind: 'file', path }, {})" />
  </section>
</template>
