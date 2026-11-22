<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Pin, PinOff, X } from 'lucide-vue-next'
import ExplorerPanel from '../../../files/components/ExplorerPanel.vue'
import { useProjectRoots } from '../../../workspace/composables/useProjectRoots'
import { withViewTransition } from '../../../../core/lib/motion'
import { renamedPanePatches, trashedPaneKeys } from '../../../files/lib/explorer/tree'
import { hasDragPath } from '../../../files/lib/explorer/dragPath'
import { dragWidth, EXPLORER_DRAWER_W, EXPLORER_MAX_W, EXPLORER_MIN_W, EXPLORER_PIN_GAP, fitPinnedWidth, fitsPush, PIN_MIN_CANVAS, pinnedRoom } from '../../lib/explorerPin'
import { isMac } from '../../../../core/lib/platform'
import { workspaceAction } from '../../../workspace/lib/workspaceKeys'
import { paneProjectOf, useWorkspaceStore, type Pane } from '../../../workspace/stores/workspace'
import { useExplorerStore } from '../../../files/stores/explorer'
import { useToastsStore } from '../../../../core/stores/toasts'
import { useUiStore } from '../../../../core/stores/ui'
import { useUiGenStore } from '../../stores/uiGen'
import { captureFocus, trapTab } from '../ui/focus'
import { useExplorerDrawer } from './useExplorerDrawer'

const FALLBACK_TOAST = 'explorer-pin-fallback'

const { t } = useI18n()
const ui = useUiStore()
const uiGen = useUiGenStore()
const workspace = useWorkspaceStore()
const explorer = useExplorerStore()
const toasts = useToastsStore()
const { rootOf } = useProjectRoots()
const { open } = useExplorerDrawer()

const root = ref<HTMLElement | null>(null)
const panelEl = ref<HTMLElement | null>(null)
const panel = ref<{ showSearch: (query?: never) => Promise<void> } | null>(null)
const canvasTotal = ref(0)
const dragging = ref(false)
const resizing = ref(false)
let restore: (() => void) | null = null
let observer: ResizeObserver | null = null
let fellBack = false

const inWorkspace = computed(() => ui.view === 'workspace' && !ui.compact && workspace.tabs.length > 0)

function paneOf(key: string): Pane | undefined {
  return workspace.panes.find((p) => p.key === key)
}

const activeProject = computed(() => {
  const pane = workspace.active
  if (!pane) return null
  const dir = paneProjectOf(workspace.tabOf(pane.key), pane, paneOf)
  return dir ? (rootOf(dir) ?? dir) : null
})
const activeFile = computed(() => (workspace.active?.kind === 'file' ? (workspace.active.path ?? null) : null))

const active = computed(() => inWorkspace.value && uiGen.isNext)
const pinFit = computed(() => fitPinnedWidth(canvasTotal.value, uiGen.explorerWidth))
const pinned = computed(() => active.value && uiGen.explorerPinned && pinFit.value !== null)
const colWidth = computed(() => (pinned.value ? (pinFit.value ?? uiGen.explorerWidth) : uiGen.explorerWidth))
const canPin = computed(() => pinned.value || pinFit.value !== null)
const pinTip = computed(() => (pinned.value ? t('mist.explorer.unpin') : canPin.value ? t('mist.explorer.pin') : t('mist.explorer.pinNoRoomCanvas', { min: PIN_MIN_CANVAS })))
const shown = computed(() => active.value && (pinned.value || open.value))
const asDrawer = computed(() => shown.value && !pinned.value)
const pushed = computed(() => asDrawer.value && fitsPush(canvasTotal.value, uiGen.explorerWidth))
const overlay = computed(() => asDrawer.value && !pushed.value)

function measure(): void {
  const parent = root.value?.parentElement
  if (parent) canvasTotal.value = parent.clientWidth
}

function openFile(path: string, _keep?: boolean, line?: number): void {
  const cwd = activeProject.value ?? undefined
  withViewTransition(() => workspace.openFile(path, line, { cwd }))
}

function openSplit(path: string): void {
  const cwd = activeProject.value ?? undefined
  const key = workspace.activeKey
  withViewTransition(() => workspace.openFile(path, undefined, key ? { cwd, split: { key, dir: 'row' } } : { cwd }))
}

function onTrashed(path: string): void {
  const keys = trashedPaneKeys(workspace.panes, path)
  if (keys.length) workspace.requestClose(keys, () => false, { record: false })
}

function onRenamed(from: string, to: string): void {
  for (const { key, patch } of renamedPanePatches(workspace.panes, from, to)) workspace.update(key, patch)
}

function focusFirst(): void {
  const el = root.value
  if (!el) return
  const target = el.querySelector<HTMLElement>('[data-explorer-tree] [tabindex="0"]') ?? el.querySelector<HTMLElement>('[data-explorer] [role="tab"][tabindex="0"]') ?? panelEl.value
  target?.focus()
}

function show(): void {
  if (!open.value) restore = captureFocus()
  open.value = true
  void nextTick(focusFirst)
}

function close(giveFocus = true): void {
  if (!open.value) return
  open.value = false
  const back = restore
  restore = null
  if (giveFocus) back?.()
}

function notifyNoRoom(): void {
  toasts.push({
    id: FALLBACK_TOAST,
    kind: 'info',
    text: t('mist.explorer.pinNoRoomCanvas', { min: PIN_MIN_CANVAS }),
    attrs: { 'data-explorer-pin-toast': 'canvas' },
  })
}

function togglePin(): void {
  if (pinned.value) {
    uiGen.explorerPinned = false
    open.value = true
    return
  }
  if (!canPin.value) {
    notifyNoRoom()
    return
  }
  uiGen.explorerPinned = true
  open.value = false
  restore = null
}

function onWindowKey(e: KeyboardEvent): void {
  if (!active.value) return
  if (workspaceAction(e, isMac, false, false)?.t !== 'explorer') return
  e.preventDefault()
  e.stopPropagation()
  if (pinned.value) focusFirst()
  else if (open.value) close()
  else show()
}

function onKeydown(e: KeyboardEvent): void {
  if (!asDrawer.value) return
  if (e.key === 'Escape' && !e.defaultPrevented) {
    const tag = (e.target as HTMLElement | null)?.tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA') return
    e.preventDefault()
    e.stopPropagation()
    close()
    return
  }
  const card = root.value?.querySelector<HTMLElement>('[data-explorer-drawer-card]')
  if (card) trapTab(e, card)
}

watch(
  () => workspace.searchRequest,
  (req) => {
    if (!req || !active.value) return
    if (req.root) explorer.setRoot(req.root)
    if (!open.value) restore = captureFocus()
    open.value = true
    void nextTick(() => panel.value?.showSearch(req.query as never))
  },
)

watch([active, () => uiGen.explorerPinned, pinned, canvasTotal], () => {
  if (!active.value || !uiGen.explorerPinned || !canvasTotal.value) {
    fellBack = false
    return
  }
  if (pinned.value) {
    fellBack = false
    return
  }
  if (fellBack) return
  fellBack = true
  notifyNoRoom()
})

watch(
  colWidth,
  (w) => document.documentElement.style.setProperty('--ccm-explorer-w', `${w}px`),
  { immediate: true },
)

watch(
  pinned,
  (on) => {
    const el = document.documentElement
    if (on) el.setAttribute('data-explorer-pinned', 'on')
    else el.removeAttribute('data-explorer-pinned')
    void nextTick(measure)
  },
  { immediate: true },
)

watch(
  pushed,
  (on) => {
    const el = document.documentElement
    if (on) el.setAttribute('data-explorer-pushed', 'on')
    else el.removeAttribute('data-explorer-pushed')
    void nextTick(measure)
  },
  { immediate: true },
)

watch(inWorkspace, (on) => {
  if (!on) open.value = false
})

watch(
  () => workspace.active?.state,
  (state) => {
    if (state === 'running' && active.value) void explorer.retryFailed()
  },
)

watch(shown, (on) => {
  if (on) void explorer.retryFailed()
})

function roomFor(): number {
  const parent = root.value?.parentElement
  if (!parent) return EXPLORER_MAX_W
  return pinned.value ? pinnedRoom(parent.clientWidth) : parent.clientWidth - 48
}

let resizeFrom: { x: number; width: number; room: number } | null = null

function onResizeDown(e: PointerEvent): void {
  if (e.button !== 0) return
  e.preventDefault()
  resizeFrom = { x: e.clientX, width: colWidth.value, room: roomFor() }
  resizing.value = true
  document.documentElement.setAttribute('data-explorer-resizing', 'on')
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
}

function onResizeMove(e: PointerEvent): void {
  if (!resizeFrom) return
  uiGen.explorerWidth = dragWidth(resizeFrom.width, resizeFrom.x, e.clientX, resizeFrom.room)
}

function onResizeUp(e: PointerEvent): void {
  if (!resizeFrom) return
  resizeFrom = null
  resizing.value = false
  document.documentElement.removeAttribute('data-explorer-resizing')
  const el = e.currentTarget as HTMLElement
  if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId)
  void nextTick(measure)
}

function onResizeKey(e: KeyboardEvent): void {
  const step = e.shiftKey ? 48 : 16
  if (e.key === 'ArrowLeft') uiGen.setExplorerWidth(colWidth.value + step, roomFor())
  else if (e.key === 'ArrowRight') uiGen.setExplorerWidth(colWidth.value - step, roomFor())
  else if (e.key === 'Home') uiGen.setExplorerWidth(EXPLORER_MIN_W)
  else if (e.key === 'End') uiGen.setExplorerWidth(EXPLORER_MAX_W, roomFor())
  else return
  e.preventDefault()
  void nextTick(measure)
}

function resetWidth(): void {
  uiGen.setExplorerWidth(EXPLORER_DRAWER_W, roomFor())
  void nextTick(measure)
}

function onDragStart(e: DragEvent): void {
  if (hasDragPath(e.dataTransfer)) dragging.value = true
}

function onDragDone(): void {
  dragging.value = false
}

onMounted(() => {
  measure()
  document.addEventListener('dragstart', onDragStart)
  document.addEventListener('dragend', onDragDone, true)
  document.addEventListener('drop', onDragDone)
  window.addEventListener('keydown', onWindowKey, true)
  window.addEventListener('resize', measure)
  const parent = root.value?.parentElement
  if (parent && typeof ResizeObserver !== 'undefined') {
    observer = new ResizeObserver(measure)
    observer.observe(parent)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('dragstart', onDragStart)
  document.removeEventListener('dragend', onDragDone, true)
  document.removeEventListener('drop', onDragDone)
  window.removeEventListener('keydown', onWindowKey, true)
  window.removeEventListener('resize', measure)
  observer?.disconnect()
  document.documentElement.removeAttribute('data-explorer-pinned')
  document.documentElement.removeAttribute('data-explorer-pushed')
  document.documentElement.removeAttribute('data-explorer-resizing')
  document.documentElement.style.removeProperty('--ccm-explorer-w')
})
</script>

<template>
  <div ref="root" class="ccm-explorer-drawer" :data-state="shown ? (pinned ? 'pinned' : 'open') : 'closed'" data-explorer-drawer @keydown="onKeydown">
    <div v-if="overlay" class="ccm-explorer-scrim" :class="{ 'is-dragging': dragging }" aria-hidden="true" data-explorer-scrim @click="close()" />
    <aside
      v-show="shown"
      class="ccm-explorer-card glass-pop"
      :class="{ 'is-pinned': pinned, 'is-pushed': pushed, 'is-resizing': resizing }"
      :style="{ width: `${colWidth}px` }"
      :role="asDrawer ? 'dialog' : 'complementary'"
      :aria-label="t('mist.explorer.drawer')"
      :aria-modal="asDrawer ? 'false' : undefined"
      data-explorer-drawer-card
    >
      <div
        class="ccm-explorer-grip"
        role="separator"
        aria-orientation="vertical"
        tabindex="0"
        :aria-label="t('mist.explorer.resize')"
        :title="t('mist.explorer.resizeTip')"
        :aria-valuemin="EXPLORER_MIN_W"
        :aria-valuemax="EXPLORER_MAX_W"
        :aria-valuenow="colWidth"
        data-explorer-grip
        @pointerdown="onResizeDown"
        @pointermove="onResizeMove"
        @pointerup="onResizeUp"
        @pointercancel="onResizeUp"
        @dblclick="resetWidth"
        @keydown="onResizeKey"
      />
      <div class="ccm-explorer-bar">
        <span class="ccm-explorer-title">{{ t('mist.explorer.title') }}</span>
        <button
          type="button"
          class="ccm-explorer-btn"
          :aria-pressed="pinned"
          :aria-label="pinTip"
          :title="pinTip"
          :data-pin-blocked="canPin ? undefined : 'canvas'"
          data-explorer-pin
          @click="togglePin"
        >
          <component :is="pinned ? PinOff : Pin" class="size-3.5" aria-hidden="true" />
        </button>
        <button v-if="asDrawer" type="button" class="ccm-explorer-btn" :aria-label="t('mist.explorer.close')" :title="t('mist.explorer.close')" data-explorer-close @click="close()">
          <X class="size-3.5" aria-hidden="true" />
        </button>
      </div>
      <div ref="panelEl" class="ccm-explorer-body" tabindex="-1">
        <ExplorerPanel
          ref="panel"
          :root="activeProject"
          :active-path="activeFile"
          :visible="shown"
          @open="openFile"
          @open-diff="(r, p) => withViewTransition(() => workspace.openChanges(r, { select: p }))"
          @open-log="(r) => withViewTransition(() => workspace.openLog(r))"
          @open-split="openSplit"
          @terminal="(dir) => withViewTransition(() => workspace.open('shell', dir))"
          @run="(dir) => withViewTransition(() => workspace.openRun(dir))"
          @renamed="onRenamed"
          @trashed="onTrashed"
        />
      </div>
    </aside>
  </div>
</template>

<style>
:root[data-ui='next'] .ccm-explorer-drawer {
  position: absolute;
  inset: 0;
  z-index: 15;
  overflow: hidden;
  pointer-events: none;
}
:root[data-ui='next'] .ccm-explorer-scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(color-mix(in srgb, var(--ccm-ink) 14%, transparent), color-mix(in srgb, var(--ccm-ink) 14%, transparent)), var(--ccm-drawer-scrim);
  pointer-events: auto;
  animation: ccm-explorer-fade var(--ccm-dur-base, 180ms) ease-out;
}
:root[data-ui='next'] .ccm-explorer-scrim.is-dragging {
  pointer-events: none;
}
:root[data-ui='next'] .ccm-explorer-card {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  display: flex;
  max-width: 100%;
  flex-direction: column;
  border-radius: var(--radius-card);
  pointer-events: auto;
  animation: ccm-explorer-in var(--ccm-dur-base, 180ms) ease-out;
}
:root[data-ui='next'] .ccm-next-canvas:has([data-next-tabs]) .ccm-explorer-card:not(.is-pinned) {
  top: 48px;
}
:root[data-ui='next'] .ccm-explorer-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 8px 0 12px;
}
:root[data-ui='next'] .ccm-explorer-title {
  min-width: 0;
  flex: 1;
  font-size: 12px;
  font-weight: 600;
  color: var(--ccm-ink);
}
:root[data-ui='next'] .ccm-explorer-btn {
  display: inline-flex;
  width: 24px;
  height: 24px;
  cursor: pointer;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-control);
  color: var(--ccm-ink-muted);
  outline: none;
}
:root[data-ui='next'] .ccm-explorer-btn:hover,
:root[data-ui='next'] .ccm-explorer-btn[aria-pressed='true'] {
  color: var(--ccm-ink);
  background: var(--ccm-control);
}
:root[data-ui='next'] .ccm-explorer-btn:focus-visible {
  outline: 2px solid var(--ccm-focus);
  outline-offset: 1px;
}
:root[data-ui='next'] .ccm-explorer-body {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  padding: 6px;
  outline: none;
}
:root[data-ui='next'][data-explorer-pinned='on'] .ccm-next-canvas,
:root[data-ui='next'][data-explorer-pushed='on'] .ccm-next-canvas {
  padding-right: calc(var(--ccm-explorer-w, 288px) + 8px);
}
:root[data-ui='next'][data-explorer-pinned='on'] .ccm-next-canvas {
  transition: padding-right var(--ccm-dur-base, 180ms) var(--ccm-ease-standard, ease-out);
}
:root[data-ui='next'] .ccm-explorer-card.is-pushed {
  animation: none;
  border-color: var(--ccm-line);
}
:root[data-ui='next'][data-explorer-resizing='on'] .ccm-next-canvas,
:root[data-ui='next'][data-motion='off'] .ccm-next-canvas {
  transition: none;
}
:root[data-ui='next'][data-explorer-resizing='on'] {
  cursor: col-resize;
  user-select: none;
}
:root[data-ui='next'] .ccm-explorer-card.is-pinned {
  background: var(--ccm-material-pop);
  -webkit-backdrop-filter: none;
  backdrop-filter: none;
  box-shadow: none;
  border-color: var(--ccm-line);
}
:root[data-ui='next'] .ccm-explorer-grip {
  position: absolute;
  top: 0;
  bottom: 0;
  left: -4px;
  z-index: 2;
  width: 9px;
  cursor: col-resize;
  touch-action: none;
  outline: none;
}
:root[data-ui='next'] .ccm-explorer-grip::after {
  position: absolute;
  top: 12px;
  bottom: 12px;
  left: 4px;
  width: 2px;
  border-radius: 1px;
  background: transparent;
  content: '';
  transition: background var(--ccm-dur-fast, 120ms) ease-out;
}
:root[data-ui='next'] .ccm-explorer-grip:hover::after,
:root[data-ui='next'] .ccm-explorer-grip:focus-visible::after,
:root[data-ui='next'] .ccm-explorer-card.is-resizing .ccm-explorer-grip::after {
  background: var(--ccm-accent);
}
:root[data-ui='next'] [data-explorer-tree] {
  overflow-x: auto;
}
:root[data-ui='next'] [data-explorer-tree-inner] {
  min-width: var(--ccm-tree-min-w, 0);
}
:root[data-ui='next'] [data-explorer-tree] [role='treeitem'][aria-level='1'] {
  position: sticky !important;
  top: auto !important;
  right: auto !important;
  left: 0 !important;
  z-index: 1;
  width: var(--ccm-tree-cw, 100%);
}
:root[data-ui='next'] [data-explorer-panel] {
  display: none;
}
:root[data-ui='next'][data-motion='off'] .ccm-explorer-card,
:root[data-ui='next'][data-motion='off'] .ccm-explorer-scrim {
  animation: none;
}
:root[data-ui='next'][data-motion='reduced'] .ccm-explorer-card {
  animation-name: ccm-explorer-fade;
}
@keyframes ccm-explorer-in {
  from {
    transform: translateX(24px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
@keyframes ccm-explorer-fade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
@media (prefers-reduced-motion: reduce) {
  :root[data-ui='next'] .ccm-explorer-card,
  :root[data-ui='next'] .ccm-explorer-scrim {
    animation: none;
  }
}
</style>
