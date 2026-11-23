<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import PaneHeader from '../../../next/components/ui/PaneHeader.vue'
import PaneLayoutMore from '../../../next/components/ui/PaneLayoutMore.vue'
import { useUiGenStore } from '../../../next/stores/uiGen'
import { CircleAlert, CircleCheck, PanelBottom, PanelBottomClose, X } from 'lucide-vue-next'
import type { BranchOpResult, CommitFile, CommitSessionChip, LogCommit } from '@ccm/shared'
import { absoluteIn, fileStepKey, scrollSideways } from '../../lib/changesView'
import { expandableIds, type RefRow } from '../../lib/gitRefs'
import { hasFilters } from '../../lib/logQuery'
import { gitBranchCreate, gitBranchDelete, gitBranchUpdate, gitFetch, hasBranchOps, hasPush } from '../../../../core/lib/preview'
import { readFavorites, readRefsView, writeFavorites, writeRefsView } from '../../lib/refPrefs'
import { useGitLog } from '../../composables/useGitLog'
import LogCommitDetails from './LogCommitDetails.vue'
import LogDiffPanel from './LogDiffPanel.vue'
import LogCommitFiles from './LogCommitFiles.vue'
import LogFilterBar from './LogFilterBar.vue'
import LogRefsToolbar from './LogRefsToolbar.vue'
import LogRefsTree from './LogRefsTree.vue'
import LogTable from './LogTable.vue'
import NewBranchDialog from './NewBranchDialog.vue'
import PushDialog from './PushDialog.vue'
import PrPanel from '../PrPanel.vue'
import { useCiEvents } from '../../composables/useCiEvents'
import { AccessPrompt } from '@features/files'
import { ChatDialog, useCommitSessions, type ChatDialogRequest } from '@features/chat'

const props = withDefaults(defineProps<{ cwd: string; paneKey?: string; active?: boolean; select?: string }>(), { active: true })
const emit = defineEmits<{ root: [root: string]; selected: []; open: [path: string, line?: number] }>()
const uiGen = useUiGenStore()
const isNext = computed(() => uiGen.isNext)
const { t } = useI18n()
useCiEvents()
const log = useGitLog(
  () => props.cwd,
  () => props.active,
)

const COLS_KEY = 'ccm.workspace.logCols'
const readCols = (): { refs: number; side: number; split: number; refsOpen: boolean; preview: boolean; previewSplit: number } => {
  try {
    const v = JSON.parse(localStorage.getItem(COLS_KEY) ?? 'null') as Record<string, unknown> | null
    const n = (x: unknown, lo: number, hi: number, d: number): number => (typeof x === 'number' && x >= lo && x <= hi ? x : d)
    return { refs: n(v?.refs, 140, 480, 220), side: n(v?.side, 200, 720, 340), split: n(v?.split, 0.2, 0.8, 0.55), refsOpen: v?.refsOpen !== false, preview: v?.preview !== false, previewSplit: n(v?.previewSplit, 0.2, 0.8, 0.45) }
  } catch {
    return { refs: 220, side: 340, split: 0.55, refsOpen: true, preview: true, previewSplit: 0.45 }
  }
}
const cols = ref(readCols())
watch(cols, (v) => {
  try {
    localStorage.setItem(COLS_KEY, JSON.stringify(v))
  } catch {
    // session only
  }
}, { deep: true })

const rootEl = ref<HTMLElement | null>(null)
const sideEl = ref<HTMLElement | null>(null)
const midEl = ref<HTMLElement | null>(null)
const table = ref<InstanceType<typeof LogTable> | null>(null)
const selectedPath = ref<string | null>(null)

let stopDrag: (() => void) | null = null
function drag(e: PointerEvent, kind: 'refs' | 'side' | 'split' | 'preview'): void {
  const startX = e.clientX
  const startY = e.clientY
  const start = { ...cols.value }
  const rect = (kind === 'preview' ? midEl.value : sideEl.value)?.getBoundingClientRect()
  const onMove = (m: PointerEvent): void => {
    if (kind === 'refs') cols.value.refs = Math.min(480, Math.max(140, start.refs + m.clientX - startX))
    else if (kind === 'side') cols.value.side = Math.min(720, Math.max(200, start.side - (m.clientX - startX)))
    else if (rect && kind === 'preview') cols.value.previewSplit = Math.min(0.8, Math.max(0.2, start.previewSplit + (m.clientY - startY) / rect.height))
    else if (rect) cols.value.split = Math.min(0.8, Math.max(0.2, start.split + (m.clientY - startY) / rect.height))
  }
  stopDrag?.()
  const onUp = (): void => {
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onUp)
    stopDrag = null
  }
  stopDrag = onUp
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
}

// The stored widths are a preference; a narrow split caps what is drawn so the table keeps room.
const MIN_MIDDLE = 240
const paneW = ref(0)
let sizeObserver: ResizeObserver | null = null
onMounted(() => {
  if (!rootEl.value || typeof ResizeObserver === 'undefined') return
  sizeObserver = new ResizeObserver((entries) => void (paneW.value = entries[0]?.contentRect.width ?? 0))
  sizeObserver.observe(rootEl.value)
})
const sideW = computed(() => (paneW.value ? Math.min(cols.value.side, Math.max(120, (paneW.value - MIN_MIDDLE - 8) * 0.55)) : cols.value.side))
const refsW = computed(() => (paneW.value ? Math.min(cols.value.refs, Math.max(100, paneW.value - MIN_MIDDLE - 8 - sideW.value)) : cols.value.refs))

watch(log.root, (r) => r && emit('root', r))
const commitSessions = useCommitSessions(log.root)
watch(log.root, () => commitSessions.reset())
const chatOpen = ref<ChatDialogRequest | null>(null)
function wantSessions(commits: LogCommit[]): void {
  commitSessions.want(commits)
}
function openSession(hash: string, chip: CommitSessionChip): void {
  chatOpen.value = { hash, chip }
}
watch(
  () => [props.select, log.commits.value.length] as const,
  ([want]) => {
    if (want && log.commits.value.some((c) => c.hash === want)) {
      log.select(want)
      emit('selected')
    }
  },
)
const empty = computed<'none' | 'filtered' | 'repo'>(() => {
  if (log.loading.value || log.commits.value.length) return 'none'
  return hasFilters(log.filters.value) ? 'filtered' : 'repo'
})
const remotes = computed(() => log.refs.value?.remotes.map((g) => g.name) ?? [])
const headBranch = computed(() => log.refs.value?.head.branch ?? null)
const headLocal = computed(() => log.refs.value?.local.find((l) => l.name === headBranch.value) ?? null)
const headSubject = computed(() => log.commits.value.find((c) => c.hash === log.refs.value?.head.hash)?.subject ?? '')

const panel = ref<InstanceType<typeof LogDiffPanel> | null>(null)
const shown = ref<{ hash: string; file: CommitFile } | null>(null)
const diffFull = ref(false)
let wantPath: string | null = null
const commitFiles = computed(() => (log.detail.value?.ok ? log.detail.value.files : []))
const diffOpen = computed(() => !!shown.value && !!log.root.value && (cols.value.preview || diffFull.value))
const shownIndex = computed(() => commitFiles.value.findIndex((f) => f.path === shown.value?.file.path))
const commitIndex = computed(() => log.commits.value.findIndex((c) => c.hash === log.selected.value))
const rootCommit = computed(() => !!log.detail.value?.ok && !log.detail.value.parents.length)

function syncDetail(): void {
  const d = log.detail.value
  if (!d?.ok || d.hash !== log.selected.value) return
  const keep = d.files.find((f) => f.path === wantPath) ?? d.files.find((f) => !!wantPath && f.origPath === wantPath) ?? null
  if (cols.value.preview || diffFull.value) {
    const f = keep ?? d.files[0] ?? null
    shown.value = f ? { hash: d.hash, file: f } : null
    selectedPath.value = f?.path ?? null
  } else selectedPath.value = keep?.path ?? null
}
watch(log.detail, syncDetail)
watch(log.selected, (hash) => {
  if (hash) return
  shown.value = null
  diffFull.value = false
  selectedPath.value = null
})

function openFile(file: CommitFile, full = false): void {
  const hash = log.selected.value
  if (!hash) return
  wantPath = file.path
  selectedPath.value = file.path
  if (full) diffFull.value = true
  if (cols.value.preview || diffFull.value) shown.value = { hash, file }
}

function pickFile(path: string): void {
  const file = commitFiles.value.find((f) => f.path === path)
  if (file) openFile(file)
}

function closeFull(): void {
  diffFull.value = false
  if (!cols.value.preview) shown.value = null
  void nextTick(() => table.value?.focus())
}

function togglePreview(): void {
  cols.value.preview = !cols.value.preview
  if (cols.value.preview) syncDetail()
  else if (!diffFull.value) shown.value = null
}

function stepFile(dir: 1 | -1): void {
  const next = commitFiles.value[shownIndex.value + dir]
  if (next && shownIndex.value >= 0) openFile(next)
}

function stepBlock(dir: 1 | -1): void {
  if (!panel.value?.stepBlock(dir)) stepFile(dir)
}

function stepCommit(dir: 1 | -1): void {
  const c = log.commits.value[commitIndex.value + dir]
  if (!c) {
    if (dir === 1 && log.more.value) void log.loadMore()
    return
  }
  log.select(c.hash)
  syncDetail()
}

function typing(e: KeyboardEvent): boolean {
  const el = e.target instanceof HTMLElement ? e.target : null
  return !!el && (el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName))
}

function inPane(): boolean {
  const a = document.activeElement
  return a === document.body || (!!a && !!rootEl.value?.contains(a))
}

function onKey(e: KeyboardEvent): void {
  if (!props.active || !inPane()) return
  if (menu.value) {
    if (e.key === 'Escape') {
      closeMenu()
      e.preventDefault()
    }
    return
  }
  if (e.defaultPrevented || !diffOpen.value || typing(e)) return
  const target = e.target instanceof HTMLElement ? e.target : null
  const plain = !e.ctrlKey && !e.metaKey && !e.altKey
  if (e.key === 'Escape' && diffFull.value) closeFull()
  else if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) stepCommit(e.key === 'ArrowUp' ? -1 : 1)
  else if (e.key === 'F7') stepBlock(e.shiftKey ? -1 : 1)
  else if (plain && (e.key === 'j' || e.key === 'k') && !target?.closest('[data-log-table]')) stepBlock(e.key === 'j' ? 1 : -1)
  else if (e.key === ']' && plain) stepFile(1)
  else if (e.key === '[' && plain) stepFile(-1)
  else if (scrollSideways(e)) return e.preventDefault()
  else if (fileStepKey(e) && !target?.closest('[role=tree],[role=toolbar],[role=menu],[role=dialog]')) stepFile(fileStepKey(e) as 1 | -1)
  else return
  e.preventDefault()
}

function openCommit(hash: string): void {
  diffFull.value = true
  log.select(hash)
  syncDetail()
}

// Branches toolbar: the focused refs-tree row is its target; clicking a ref still filters the log.
const refsTree = ref<InstanceType<typeof LogRefsTree> | null>(null)
const selectedRef = ref<RefRow | null>(null)
// An unmounted tree cannot emit `select` any more, so the toolbar would keep targeting its last row.
watch([() => cols.value.refsOpen, log.root], () => (selectedRef.value = null))
const refsCollapsed = ref<Set<string>>(new Set())
const refsView = ref(readRefsView())
watch(refsView, writeRefsView, { deep: true })
const favorites = ref<Set<string>>(new Set())
watch(
  log.refs,
  (r) => {
    if (r && log.root.value) favorites.value = readFavorites(log.root.value, r)
  },
  { immediate: true },
)
const branchBusy = ref<'fetch' | 'update' | 'delete' | null>(null)
const branchNotice = ref<{ text: string; failed: boolean } | null>(null)
const newBranch = ref<{ base: string; baseLabel: string } | null>(null)
const push = ref<{ root: string; ref: string } | null>(null)
/** Push source: the selected local branch, else the current branch. */
const pushRef = computed(() => {
  const r = selRef()
  if (r) return r.refKind === 'local' ? r.ref : null
  const head = log.refs.value?.head.branch
  return head ? `refs/heads/${head}` : null
})
const shortRef = (ref: string): string => ref.replace(/^refs\/(heads|remotes|tags)\//, '')

function report(res: BranchOpResult, okKey: string, name = ''): void {
  if (res.ok) branchNotice.value = { text: t(`workspace.log.branches.done.${okKey}`, { name }), failed: false }
  else if (res.error !== 'cancelled') branchNotice.value = { text: [t(`workspace.log.branches.error.${res.error}`, { name }), res.detail].filter(Boolean).join(' - '), failed: true }
  void log.reload()
}

function selRef(): Extract<RefRow, { kind: 'ref' }> | null {
  return selectedRef.value?.kind === 'ref' ? selectedRef.value : null
}

function askNewBranch(): void {
  const r = selRef()
  const commit = log.selected.value
  newBranch.value = r ? { base: r.ref, baseLabel: r.name } : commit ? { base: commit, baseLabel: commit.slice(0, 8) } : { base: 'HEAD', baseLabel: 'HEAD' }
}
async function createBranch(name: string, checkout: boolean): Promise<void> {
  const req = newBranch.value
  newBranch.value = null
  if (req && log.root.value) report(await gitBranchCreate(log.root.value, name, req.base, checkout), checkout ? 'createdSwitched' : 'created', name)
}
function askPush(): void {
  if (log.root.value && pushRef.value && !branchBusy.value) push.value = { root: log.root.value, ref: pushRef.value }
}
function askPushHead(): void {
  if (log.root.value && headBranch.value && !branchBusy.value) push.value = { root: log.root.value, ref: `refs/heads/${headBranch.value}` }
}
function pushed(res: BranchOpResult, name: string): void {
  push.value = null
  report(res, 'pushed', name)
}
async function updateSelected(): Promise<void> {
  const r = selRef()
  if (!r || !log.root.value || branchBusy.value) return
  branchBusy.value = 'update'
  try {
    report(await gitBranchUpdate(log.root.value, r.ref), 'updated', r.name)
  } finally {
    branchBusy.value = null
  }
}
async function deleteSelected(): Promise<void> {
  const r = selRef()
  if (!r || !log.root.value || branchBusy.value) return
  branchBusy.value = 'delete'
  try {
    report(await gitBranchDelete(log.root.value, r.ref), 'deleted', r.name)
  } finally {
    branchBusy.value = null
  }
}
async function fetchRemotes(): Promise<void> {
  if (!log.root.value || branchBusy.value) return
  branchBusy.value = 'fetch'
  try {
    report(await gitFetch(log.root.value), 'fetched')
  } finally {
    branchBusy.value = null
  }
}
/** Branch filter before Compare set its own; Clear goes back to it so the log is not left on the compared branch. */
let branchBeforeCompare: string | null = null
function clearCompare(): void {
  log.setFilters({ branch: branchBeforeCompare, notRef: null })
  branchBeforeCompare = null
}
function compareSelected(): void {
  const r = selRef()
  if (!r) return
  if (!log.filters.value.notRef) branchBeforeCompare = log.filters.value.branch
  // The named current branch, not `HEAD`: after Swap the filter bar then shows a branch it lists.
  const head = log.refs.value?.head.branch
  log.setFilters({ branch: r.ref, notRef: head ? `refs/heads/${head}` : 'HEAD' })
}
function toggleFavorite(): void {
  const r = selRef()
  if (!r || !log.root.value) return
  const next = new Set(favorites.value)
  if (next.has(r.ref)) next.delete(r.ref)
  else next.add(r.ref)
  favorites.value = next
  writeFavorites(log.root.value, next)
}
async function navigateTo(): Promise<void> {
  const r = selRef()
  const refs = log.refs.value
  if (!r || !refs) return
  const hash = [...refs.local, ...refs.remotes.flatMap((g) => g.refs), ...refs.tags].find((x) => x.ref === r.ref)?.hash
  if (hash && (await log.reveal(hash))) {
    await nextTick()
    if (table.value?.scrollToHash(hash)) return
  }
  branchNotice.value = { text: t('workspace.log.branches.notLoaded', { name: r.name }), failed: true }
}
function collapseAllRefs(): void {
  const refs = log.refs.value
  if (refs) refsCollapsed.value = new Set(expandableIds(refs, { favorites: favorites.value, ...refsView.value }))
}

// Context menus (read-only copy / open actions).
const MENU_W = 208
const menu = ref<{ x: number; y: number; items: { id: string; label: string; run: () => void }[] } | null>(null)
const menuEl = ref<HTMLElement | null>(null)

function openMenu(x: number, y: number, items: { id: string; label: string; run: () => void }[]): void {
  const h = items.length * 28 + 8
  menu.value = { x: Math.max(0, Math.min(x, window.innerWidth - MENU_W)), y: Math.max(0, Math.min(y, window.innerHeight - h)), items }
  void nextTick(() => menuEl.value?.querySelector<HTMLElement>('button')?.focus())
}

function menuKey(e: KeyboardEvent): void {
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
  const items = [...(menuEl.value?.querySelectorAll<HTMLElement>('button') ?? [])]
  if (!items.length) return
  const at = items.indexOf(document.activeElement as HTMLElement)
  items[(at + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length]!.focus()
  e.preventDefault()
}
const copy = (s: string): void => void navigator.clipboard.writeText(s).catch(() => undefined)

function commitMenu(hash: string, x: number, y: number): void {
  const c = log.commits.value.find((k) => k.hash === hash)
  openMenu(x, y, [
    { id: 'hash', label: t('workspace.log.menu.copyHash'), run: () => copy(hash) },
    { id: 'msg', label: t('workspace.log.menu.copyMessage'), run: () => copy(log.detail.value?.ok && log.detail.value.hash === hash ? log.detail.value.message : (c?.subject ?? '')) },
    { id: 'diff', label: t('workspace.log.menu.showDiff'), run: () => openCommit(hash) },
  ])
}

function fileMenu(file: CommitFile, x: number, y: number): void {
  const root = log.root.value
  openMenu(x, y, [
    { id: 'diff', label: t('workspace.log.menu.showDiff'), run: () => openFile(file, true) },
    ...(root && file.kind !== 'deleted' ? [{ id: 'open', label: t('workspace.log.menu.openCurrent'), run: () => emit('open', absoluteIn(root, file.path)) }] : []),
    { id: 'path', label: t('workspace.log.menu.copyPath'), run: () => copy(file.path) },
  ])
}

const closeMenu = (): void => void (menu.value = null)
window.addEventListener('keydown', onKey)
window.addEventListener('pointerdown', closeMenu)
onBeforeUnmount(() => {
  stopDrag?.()
  sizeObserver?.disconnect()
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('pointerdown', closeMenu)
})
</script>

<template>
  <section ref="rootEl" class="flex min-h-0 min-w-0 flex-col bg-surface" :class="isNext ? 'ccm-next-viewer' : ''" :aria-label="t('workspace.log.title')" data-log-pane>
    <component :is="isNext ? PaneHeader : 'div'" class="@container select-none" :class="isNext ? 'ccm-next-head ccm-next-viewer-head' : 'flex h-9 min-w-0 shrink-0 items-center gap-1 border-b border-line px-2'" data-pane-head>
      <span class="min-w-0 truncate text-xs font-semibold text-ink" :title="log.root.value ?? cwd">{{ t('workspace.log.title') }}</span>
      <span class="flex-1" />
      <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :aria-pressed="cols.preview" :title="cols.preview ? t('workspace.log.diff.hidePreview') : t('workspace.log.diff.showPreview')" :aria-label="t('workspace.log.diff.preview')" data-log-preview-toggle @click="togglePreview">
        <component :is="cols.preview ? PanelBottomClose : PanelBottom" :size="14" aria-hidden="true" />
      </button>
      <PaneLayoutMore :pane-key="paneKey" />
      <slot name="end" />
    </component>

    <PrPanel
      v-if="log.root.value && !(log.error.value && !log.commits.value.length)"
      :root="log.root.value"
      :branch="headBranch"
      :head-hash="log.refs.value?.head.hash ?? null"
      :subject="headSubject"
      :upstream="headLocal?.upstream ?? null"
      :ahead="headLocal?.ahead ?? 0"
      :active="active"
      @push="askPushHead"
      @changed="log.reload()"
    />

    <div v-if="log.error.value && !log.commits.value.length" class="grid flex-1 place-items-center p-6 text-center" data-log-fatal>
      <AccessPrompt v-if="log.error.value === 'denied'" :path="log.root.value ?? cwd" @granted="log.reload()" />
      <div v-else class="flex flex-col items-center gap-2">
        <p class="text-sm text-ink-muted">{{ t(`workspace.log.error.${log.error.value}`) }}</p>
        <button type="button" class="ccm-btn" @click="log.reload()">{{ t('workspace.log.table.retry') }}</button>
      </div>
    </div>

    <div v-else class="flex min-h-0 min-w-0 flex-1">
      <LogRefsToolbar
        :open="cols.refsOpen"
        :selected="selectedRef"
        :busy="branchBusy"
        :view="refsView"
        :can-write="hasBranchOps && !!log.root.value"
        :can-push="hasPush && !!log.root.value && !!pushRef"
        @toggle="cols.refsOpen = !cols.refsOpen"
        @new-branch="askNewBranch"
        @update="updateSelected"
        @push="askPush"
        @remove="deleteSelected"
        @compare="compareSelected"
        @search="refsTree?.focusSearch()"
        @fetch="fetchRemotes"
        @favorite="toggleFavorite"
        @navigate="navigateTo"
        @view="(v) => (refsView = v)"
        @expand-all="refsCollapsed = new Set()"
        @collapse-all="collapseAllRefs"
      />
      <template v-if="cols.refsOpen">
        <LogRefsTree
          :key="log.root.value ?? ''"
          ref="refsTree"
          v-model:collapsed="refsCollapsed"
          class="shrink-0"
          :style="{ width: `${refsW}px` }"
          :refs="log.refs.value"
          :branch="log.filters.value.branch"
          :favorites="favorites"
          :view="refsView"
          @pick="(name) => log.setFilters({ branch: name, notRef: null })"
          @select="(row) => (selectedRef = row)"
        />
        <div class="w-1 shrink-0 cursor-col-resize border-l border-line hover:bg-fill" role="separator" aria-orientation="vertical" @pointerdown.prevent="drag($event, 'refs')" />
      </template>

      <div ref="midEl" class="flex min-h-0 min-w-0 flex-1 flex-col">
        <template v-if="!diffFull">
          <p v-if="log.filters.value.notRef && log.filters.value.branch" class="flex shrink-0 items-center gap-2 border-b border-line bg-accent-soft px-3 py-1 text-xs text-ink" role="status" data-log-compare>
            <span class="min-w-0 flex-1 truncate">{{ t('workspace.log.branches.comparing', { a: shortRef(log.filters.value.branch), b: shortRef(log.filters.value.notRef) }) }}</span>
            <button type="button" class="ccm-btn-plain h-6 shrink-0 px-2 text-xs" data-log-compare-swap @click="log.setFilters({ branch: log.filters.value.notRef, notRef: log.filters.value.branch })">{{ t('workspace.log.branches.swap') }}</button>
            <button type="button" class="ccm-btn-plain h-6 shrink-0 px-2 text-xs" data-log-compare-clear @click="clearCompare">{{ t('workspace.log.branches.clear') }}</button>
          </p>
          <p v-if="branchNotice" class="flex shrink-0 items-center gap-2 border-b border-line px-3 py-1 text-xs text-ink" :class="branchNotice.failed ? 'bg-st-error-soft' : 'bg-fill'" role="status" data-log-branch-notice>
            <component :is="branchNotice.failed ? CircleAlert : CircleCheck" :size="13" class="shrink-0" :class="branchNotice.failed ? 'text-st-error' : 'text-success'" aria-hidden="true" />
            <span class="min-w-0 flex-1 truncate" :title="branchNotice.text">{{ branchNotice.text }}</span>
            <button type="button" class="shrink-0 cursor-pointer rounded-inner text-ink-faint hover:text-ink" :title="t('workspace.log.branches.dismiss')" :aria-label="t('workspace.log.branches.dismiss')" @click="branchNotice = null"><X :size="12" aria-hidden="true" /></button>
          </p>
        </template>
        <LogFilterBar v-show="!diffFull" :filters="log.filters.value" :refs="log.refs.value" :authors="log.authors.value" :root="log.root.value ?? cwd" :flat="log.mode.value === 'flat'" @change="log.setFilters" @refresh="log.reload()" />
        <LogTable
          v-show="!diffFull"
          ref="table"
          :style="diffOpen ? { flex: `${cols.previewSplit} 1 0` } : undefined"
          :commits="log.commits.value"
          :rows="log.rows.value"
          :selected="log.selected.value"
          :head-hash="log.refs.value?.head.hash ?? null"
          :remotes="remotes"
          :loading="log.loading.value"
          :loading-more="log.loadingMore.value"
          :more="log.more.value"
          :empty="empty"
          :error="log.pageError.value"
          :filtered="hasFilters(log.filters.value)"
          @select="log.select"
          @open="openCommit"
          @load-more="log.loadMore()"
          @menu="commitMenu"
          @clear="log.setFilters({ text: '', branch: null, notRef: null, author: null, date: 'any', from: '', to: '', paths: [] })"
          @retry="log.reload()"
          :sessions="commitSessions.sessions.value"
          @visible="wantSessions"
          @session="openSession"
        />
        <div v-if="diffOpen && !diffFull" class="h-1 shrink-0 cursor-row-resize border-t border-line hover:bg-fill" role="separator" aria-orientation="horizontal" data-log-preview-split @pointerdown.prevent="drag($event, 'preview')" />
        <Transition name="ccm-fade">
          <LogDiffPanel
            v-if="diffOpen && shown && log.root.value"
            ref="panel"
            class="min-h-0"
            :style="{ flex: diffFull ? '1 1 0' : `${1 - cols.previewSplit} 1 0` }"
            :root="log.root.value"
            :hash="shown.hash"
            :file="shown.file"
            :root-commit="rootCommit"
            :full="diffFull"
            :index="shownIndex"
            :total="commitFiles.length"
            :has-newer="commitIndex > 0"
            :has-older="commitIndex >= 0 && (commitIndex < log.commits.value.length - 1 || log.more.value)"
            @back="closeFull"
            @expand="diffFull = true"
            @hide="togglePreview"
            @step="stepFile"
            @commit="stepCommit"
          />
        </Transition>
      </div>
      <div class="w-1 shrink-0 cursor-col-resize border-l border-line hover:bg-fill" role="separator" aria-orientation="vertical" @pointerdown.prevent="drag($event, 'side')" />
      <div ref="sideEl" class="flex min-h-0 shrink-0 flex-col" :style="{ width: `${sideW}px` }">
        <LogCommitFiles class="min-h-0" :style="{ flex: `${cols.split} 1 0` }" :detail="log.detail.value" :loading="log.detailLoading.value" :selected-path="selectedPath" @select="pickFile" @open="(f) => openFile(f, true)" @menu="fileMenu" />
        <div class="h-1 shrink-0 cursor-row-resize border-t border-line hover:bg-fill" role="separator" aria-orientation="horizontal" @pointerdown.prevent="drag($event, 'split')" />
        <LogCommitDetails class="min-h-0" :style="{ flex: `${1 - cols.split} 1 0` }" :detail="log.detail.value" :loading="log.detailLoading.value" :contains="log.contains" />
      </div>
    </div>

    <div v-if="menu" ref="menuEl" class="ccm-menu fixed z-50 flex w-52 flex-col p-1" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" role="menu" @pointerdown.stop @keydown="menuKey">
      <button v-for="m in menu.items" :key="m.id" type="button" role="menuitem" class="flex h-7 w-full items-center rounded-inner px-2 text-left text-xs text-ink hover:bg-fill focus-visible:bg-accent focus-visible:text-on-accent" @click="closeMenu(); m.run()">
        {{ m.label }}
      </button>
    </div>
    <NewBranchDialog :request="newBranch" @create="createBranch" @close="newBranch = null" />
    <PushDialog :request="push" @done="pushed" @close="push = null" />
    <ChatDialog :request="chatOpen" @close="chatOpen = null" @open-file="(p) => emit('open', p)" />
  </section>
</template>
