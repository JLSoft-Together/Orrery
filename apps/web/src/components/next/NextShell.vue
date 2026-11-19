<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, provide, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  Activity,
  Bell,
  ChartColumnBig,
  CircleArrowDown,
  Cpu,
  Ellipsis,
  GraduationCap,
  HardDrive,
  History,
  Inbox,
  Keyboard,
  Maximize2,
  Minimize2,
  PanelLeft,
  Puzzle,
  Search,
  Settings2,
  Smartphone,
  SquareTerminal,
  SunDim,
  Undo2,
} from 'lucide-vue-next'
import { usePageActive } from '../../composables/usePageActive'
import { hasExtensions, hasTerminals, titleBar } from '../../lib/desktop'
import { hasDevices } from '../../lib/devices'
import { hasDiskMap } from '../../lib/diskMap'
import { hasTelegram } from '../../lib/telegram'
import { formatKeys } from '../../lib/shortcuts'
import { isMac } from '../../lib/platform'
import { leaves } from '../../lib/splitLayout'
import { canCollapseRail, effectiveRailMode, isRailToggleKey, readRailCollapsed, writeRailCollapsed } from '../../lib/next/railCollapse'
import { clampRailWidth, nextRailMode, RAIL_MAX, RAIL_MIN, railModeFor, railWidth, readRailWidth, writeRailWidth, type RailMode } from '../../lib/next/railMode'
import { NEXT_SHELL_KEY } from '../../lib/next/shellContext'
import { useExtrasStore } from '../../stores/extras'
import { useInboxStore } from '../../stores/inbox'
import { useResourcesStore } from '../../stores/resources'
import { useSettingsStore } from '../../stores/settings'
import { useUiStore, type View } from '../../stores/ui'
import { useUiGenStore } from '../../stores/uiGen'
import { useUpdateStore } from '../../stores/update'
import { useWorkspaceStore } from '../../stores/workspace'
import HeaderStatusPopover from '../HeaderStatusPopover.vue'
import SettingsMenu from '../SettingsMenu.vue'
import SideResizer from '../workspace/SideResizer.vue'
import TelegramToggle from '../TelegramToggle.vue'
import LiveRail from './rail/LiveRail.vue'
import RailCollapseButton from './rail/RailCollapseButton.vue'
import NextQuotaBar from './NextQuotaBar.vue'
import GlassMenu, { type GlassMenuItem } from './ui/GlassMenu.vue'
import GlassTooltip from './ui/GlassTooltip.vue'
import SegmentedControl, { type NextSegment } from './ui/SegmentedControl.vue'
import WindowControls from '../ui/WindowControls.vue'

const InboxPanel = defineAsyncComponent(() => import('../inbox/InboxPanel.vue'))

const { t } = useI18n()
const ui = useUiStore()
const gen = useUiGenStore()
const settings = useSettingsStore()
const workspace = useWorkspaceStore()
const inbox = useInboxStore()
const extras = useExtrasStore()
const resources = useResourcesStore()
const update = useUpdateStore()

const viewport = ref(typeof window === 'undefined' ? 1440 : window.innerWidth)
const baseMode = ref<RailMode>(railModeFor(viewport.value))
const collapsed = ref(readRailCollapsed())
const mode = computed<RailMode>(() => effectiveRailMode(baseMode.value, collapsed.value))
const collapsible = computed(() => canCollapseRail(baseMode.value))
const custom = ref<number | null>(readRailWidth())
const overlayOpen = ref(false)
const dragging = ref(false)

const width = computed(() => railWidth(mode.value, viewport.value, custom.value))
const railOpen = computed(() => mode.value !== 'overlay' || overlayOpen.value)
const railHidden = computed(() => ui.compact)
const iconMode = computed(() => mode.value === 'icon')

function onResize(): void {
  viewport.value = window.innerWidth
  baseMode.value = nextRailMode(baseMode.value, viewport.value)
}
function toggleCollapse(): void {
  if (!collapsible.value) return
  collapsed.value = !collapsed.value
  writeRailCollapsed(collapsed.value)
}
function closeRail(): void {
  overlayOpen.value = false
}
function openRail(): void {
  if (mode.value === 'overlay') overlayOpen.value = true
}
function toggleRail(): void {
  if (mode.value === 'overlay') overlayOpen.value = !overlayOpen.value
}
watch(mode, (m) => {
  if (m !== 'overlay') overlayOpen.value = false
})
watch(
  () => ui.view,
  () => closeRail(),
)

function onKey(e: KeyboardEvent): void {
  if (isRailToggleKey(e, isMac) && collapsible.value && !railHidden.value) {
    e.preventDefault()
    e.stopPropagation()
    toggleCollapse()
    return
  }
  if (e.key === 'Escape' && overlayOpen.value && mode.value === 'overlay') {
    e.preventDefault()
    closeRail()
  }
}

provide(NEXT_SHELL_KEY, { railMode: computed(() => mode.value), railWidth: width, railOpen, openRail, closeRail, toggleRail })

function resizeRail(px: number): void {
  custom.value = clampRailWidth(px)
}
function commitRail(): void {
  writeRailWidth(custom.value)
}
function resetRail(): void {
  custom.value = null
  writeRailWidth(null)
}

const pageActive = usePageActive()
function applyPause(): void {
  document.documentElement.toggleAttribute('data-paused', !pageActive.value)
}
watch(pageActive, applyPause, { immediate: true })

onMounted(() => {
  window.addEventListener('resize', onResize)
  window.addEventListener('keydown', onKey, true)
  onResize()
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize)
  window.removeEventListener('keydown', onKey, true)
  document.documentElement.removeAttribute('data-paused')
})

const options = computed<NextSegment<View>[]>(() => [
  ...(hasTerminals ? [{ value: 'workspace' as View, label: t('nav.workspace'), icon: SquareTerminal }] : []),
  { value: 'monitor' as View, label: t('nav.monitor'), icon: Activity },
])
const textOptions = computed<NextSegment<View>[]>(() => options.value.map(({ value, label }) => ({ value, label })))
const view = computed<View>({
  get: () => ui.view,
  set: (value) => ui.setView(value),
})

const inboxBtn = ref<HTMLButtonElement | null>(null)
const inboxAt = ref<DOMRect | null>(null)
const inboxCount = computed(() => inbox.items.length)
watch(
  () => inbox.open,
  (o) => (inboxAt.value = o ? (inboxBtn.value?.getBoundingClientRect() ?? null) : null),
)

const searchKeys = computed(() => formatKeys(['Mod', 'K'], isMac).join(isMac ? '' : '+'))
const updateVersion = computed(() => update.state?.release?.version ?? '')
const splitTab = computed(() => ui.view === 'workspace' && !!workspace.activeTab && leaves(workspace.activeTab.root).length > 1)

const moreBtn = ref<HTMLButtonElement | null>(null)
const moreOpen = ref(false)
const moreAt = ref({ x: 0, y: 0 })
let moreClosedAt = 0
const MENU_W = 240

const moreItems = computed<GlassMenuItem[]>(() => {
  const views: GlassMenuItem[] = ui.compact
    ? []
    : [
        ...(hasExtensions ? [{ id: 'view:extensions', label: t('nav.extensions'), icon: Puzzle }] : []),
        ...(hasDevices ? [{ id: 'view:devices', label: t('nav.devices'), icon: Smartphone }] : []),
        ...(hasDiskMap ? [{ id: 'view:disk', label: t('diskMap.nav'), icon: HardDrive }] : []),
      ]
  const list: GlassMenuItem[] = [...views, { id: 'resources', label: t('resources.open'), icon: Cpu }]
  list.push({ id: 'sep1', label: '', separator: true })
  list.push({ id: 'compact', label: ui.compact ? t('compact.exit') : t('compact.enter'), icon: ui.compact ? Maximize2 : Minimize2 })
  if (splitTab.value) list.push({ id: 'dim', label: t(settings.dimInactive ? 'workspace.dim.toggleOn' : 'workspace.dim.toggle'), icon: SunDim })
  list.push({ id: 'notify', label: settings.notifyEnabled ? t('settings.notifyOff') : t('settings.notifyOn'), icon: Bell, disabled: settings.notifyPermission === 'unsupported' })
  list.push({ id: 'sep2', label: '', separator: true })
  list.push({ id: 'shortcuts', label: t('wsHelp.shortcuts'), icon: Keyboard })
  if (ui.view === 'workspace') list.push({ id: 'tour', label: t('wsHelp.tour'), icon: GraduationCap })
  list.push({ id: 'sep3', label: '', separator: true })
  list.push({ id: 'classic', label: t('mist.switch.backClassic'), icon: Undo2 })
  return list
})

function toggleMore(): void {
  if (moreOpen.value) return void (moreOpen.value = false)
  if (performance.now() - moreClosedAt < 250) return
  const r = moreBtn.value?.getBoundingClientRect()
  if (!r) return
  const rows = moreItems.value.reduce((h, it) => h + (it.separator ? 9 : 32), 8)
  const y = Math.max(8, Math.min(r.top - rows - 6, window.innerHeight - rows - 8))
  moreAt.value = { x: iconMode.value ? r.right + 8 : r.right - MENU_W, y }
  moreOpen.value = true
}
function closeMore(): void {
  moreOpen.value = false
  moreClosedAt = performance.now()
}
function pickMore(id: string): void {
  if (id.startsWith('view:')) return ui.setView(id.slice(5) as View)
  if (id === 'resources') return resources.show()
  if (id === 'compact') return ui.toggleCompact()
  if (id === 'dim') return void (settings.dimInactive = !settings.dimInactive)
  if (id === 'notify') return void settings.toggleNotifications()
  if (id === 'shortcuts') return void (ui.shortcutsOpen = true)
  if (id === 'tour') return void (ui.tourOpen = 'workspace')
  if (id === 'classic') gen.setGeneration('classic')
}

function onIconKey(e: KeyboardEvent): void {
  const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
  if (!step) return
  e.preventDefault()
  const list = options.value
  const at = list.findIndex((o) => o.value === view.value)
  const next = list[(at + step + list.length) % list.length]
  if (!next) return
  view.value = next.value
  const group = e.currentTarget as HTMLElement
  void nextTick(() => group.querySelector<HTMLElement>(`[data-view-switch="${next.value}"]`)?.focus())
}

const iconBtn =
  'relative inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors duration-(--ccm-dur-fast) outline-none hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus'
</script>

<template>
  <div class="ccm-next-shell" :data-rail-mode="mode" :data-rail-open="railOpen || undefined" :data-rail-hidden="railHidden || undefined" :data-rail-dragging="dragging || undefined" :data-title-bar="titleBar ?? undefined" :style="{ '--ccm-rail-w': `${width}px` }" data-next-shell>
    <div v-if="titleBar" class="ccm-next-drag" aria-hidden="true" data-next-drag data-tauri-drag-region />
    <WindowControls v-if="titleBar === 'overlay'" class="ccm-next-winctl" />
    <div v-if="mode === 'overlay' && overlayOpen && !railHidden" class="fixed inset-0 z-30 bg-(--ccm-scrim,rgb(0_0_0/0.32))" aria-hidden="true" data-rail-scrim @click="closeRail" />

    <div v-if="!railHidden" class="ccm-next-rail-cell" :class="mode === 'overlay' ? 'ccm-next-rail-overlay' : ''" :aria-hidden="!railOpen || undefined" :inert="!railOpen || undefined">
      <aside id="ccm-next-rail" class="glass-side flex h-full min-h-0 flex-col overflow-hidden rounded-[18px]" :aria-label="t('mist.shell.rail')" data-next-rail>
        <div class="flex shrink-0 gap-2 px-2.5" :class="iconMode ? 'flex-col items-center py-2.5' : 'h-11 items-center'">
          <span class="bg-accent-grad inline-grid size-[26px] shrink-0 place-items-center rounded-inner text-[13px] font-bold" aria-hidden="true" data-next-logo>O</span>
          <SegmentedControl v-if="!iconMode" v-model="view" :options="textOptions" :label="t('mist.switch.title')" size="sm" fill class="min-w-0 flex-1" data-tour="tabs" />
          <RailCollapseButton v-if="!iconMode && collapsible" :collapsed="false" @toggle="toggleCollapse" />
          <div v-else class="flex flex-col gap-1" role="radiogroup" :aria-label="t('mist.switch.title')" data-tour="tabs" @keydown="onIconKey">
            <GlassTooltip v-for="o in options" :key="o.value" :text="o.label" side="bottom">
              <button
                type="button"
                role="radio"
                :aria-checked="view === o.value"
                :aria-label="o.label"
                :tabindex="view === o.value ? 0 : -1"
                :class="[iconBtn, view === o.value ? 'bg-accent-soft text-accent-ink' : '']"
                :data-view-switch="o.value"
                @click="view = o.value"
              >
                <component :is="o.icon" :size="16" aria-hidden="true" />
              </button>
            </GlassTooltip>
          </div>
          <RailCollapseButton v-if="iconMode && collapsible" :collapsed="true" @toggle="toggleCollapse" />
        </div>

        <button
          v-if="!iconMode"
          type="button"
          class="mx-2.5 mb-1.5 flex h-[30px] shrink-0 cursor-pointer items-center gap-1.5 rounded-[10px] border border-line bg-(--ccm-control) px-2.5 text-left text-xs text-ink-faint outline-none transition-colors duration-(--ccm-dur-fast) hover:text-ink-muted focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus"
          :aria-label="t('palette.open')"
          :aria-keyshortcuts="isMac ? 'Meta+K' : 'Control+K'"
          data-tour="palette"
          data-next-search
          @click="ui.paletteOpen = true"
        >
          <Search :size="14" class="shrink-0" aria-hidden="true" />
          <span class="min-w-0 flex-1 truncate">{{ t('mist.shell.search') }}</span>
          <kbd class="shrink-0 rounded-[5px] border border-line-strong px-1 font-mono text-[10px] font-semibold text-ink-muted">{{ searchKeys }}</kbd>
        </button>
        <GlassTooltip v-else :text="t('palette.open')" side="bottom">
          <button type="button" :class="[iconBtn, 'mx-auto mb-1']" :aria-label="t('palette.open')" data-tour="palette" data-next-search @click="ui.paletteOpen = true">
            <Search :size="16" aria-hidden="true" />
          </button>
        </GlassTooltip>

        <LiveRail :collapsed="iconMode" />

        <div class="flex shrink-0 flex-col gap-1.5 border-t border-line px-2 py-2" :class="iconMode ? 'items-center' : ''" data-next-rail-foot>
          <div v-if="(update.available || hasTelegram) && !iconMode" class="flex flex-wrap items-center gap-1">
            <button
              v-if="update.available"
              type="button"
              class="inline-flex h-6 cursor-pointer items-center gap-1 rounded-full bg-fill px-2 text-2xs font-semibold text-accent-ink"
              :title="t('update.pillTitle', { version: updateVersion })"
              :aria-label="t('update.pillTitle', { version: updateVersion })"
              aria-haspopup="dialog"
              data-update-pill
              @click="update.dialogOpen = true"
            >
              <CircleArrowDown :size="14" aria-hidden="true" />
              <span class="tabular-nums">{{ t('update.pill', { version: updateVersion }) }}</span>
            </button>
            <TelegramToggle v-if="hasTelegram" />
          </div>
          <TelegramToggle v-if="iconMode && hasTelegram" icon-only />
          <button
            v-if="iconMode && update.available"
            type="button"
            :class="iconBtn"
            :title="t('update.pillTitle', { version: updateVersion })"
            :aria-label="t('update.pillTitle', { version: updateVersion })"
            data-update-pill
            @click="update.dialogOpen = true"
          >
            <CircleArrowDown :size="16" class="text-accent-ink" aria-hidden="true" />
          </button>

          <div class="flex items-center gap-0.5" :class="iconMode ? 'flex-col' : 'justify-between'">
            <div class="ccm-next-status"><HeaderStatusPopover /></div>
            <button
              ref="inboxBtn"
              type="button"
              :class="[iconBtn, inbox.open ? 'bg-fill text-ink' : '']"
              :aria-label="t('inbox.button', { n: inboxCount }, inboxCount)"
              :title="t('inbox.button', { n: inboxCount }, inboxCount)"
              aria-haspopup="dialog"
              :aria-expanded="inbox.open"
              data-inbox-button
              @click="inbox.open = !inbox.open"
            >
              <Inbox :size="16" aria-hidden="true" />
              <span
                v-if="inboxCount"
                class="absolute top-0.5 right-0.5 grid h-3.5 min-w-3.5 place-items-center rounded-full bg-accent px-0.5 text-[10px] leading-none font-bold text-on-accent tabular-nums"
                aria-hidden="true"
                data-inbox-badge
                >{{ inboxCount > 99 ? '99+' : inboxCount }}</span
              >
            </button>
            <button type="button" :class="[iconBtn, ui.view === 'usage' ? 'bg-accent-soft text-accent-ink' : '']" :aria-label="t('nav.usage')" :title="t('nav.usage')" :aria-current="ui.view === 'usage' ? 'page' : undefined" data-view-switch="usage" @click="ui.setView('usage')">
              <ChartColumnBig :size="16" aria-hidden="true" />
            </button>
            <button type="button" :class="[iconBtn, ui.view === 'history' ? 'bg-accent-soft text-accent-ink' : '']" :aria-label="t('nav.history')" :title="t('nav.history')" :aria-current="ui.view === 'history' ? 'page' : undefined" data-view-switch="history" @click="ui.setView('history')">
              <History :size="16" aria-hidden="true" />
            </button>
            <button type="button" :class="[iconBtn, ui.settingsOpen ? 'bg-fill text-ink' : '']" data-tour="settings" :aria-label="t('settings.title')" :title="t('settings.title')" :aria-expanded="ui.settingsOpen" aria-controls="settings-panel" data-next-settings @click="ui.settingsOpen = !ui.settingsOpen">
              <Settings2 :size="16" aria-hidden="true" />
              <span v-if="update.available" class="absolute top-1 right-1 size-2 rounded-full bg-accent ring-2 ring-canvas" aria-hidden="true" data-update-dot />
            </button>
            <button
              ref="moreBtn"
              type="button"
              :class="[iconBtn, moreOpen ? 'bg-fill text-ink' : '']"
              data-tour="compact"
              data-more-views
              :aria-label="t('nav.more')"
              :title="t('nav.more')"
              aria-haspopup="menu"
              :aria-expanded="moreOpen"
              @click="toggleMore"
            >
              <Ellipsis :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>
      </aside>
      <SideResizer v-if="mode === 'full'" v-model:dragging="dragging" :width="width" :min="RAIL_MIN" :max="RAIL_MAX" :label="t('mist.shell.rail')" @resize="resizeRail" @commit="commitRail" @reset="resetRail" />
    </div>

    <div class="ccm-next-canvas" data-next-canvas>
      <button
        v-if="mode === 'overlay' && !railHidden && !overlayOpen"
        type="button"
        class="glass fixed top-3 left-3 z-20 inline-flex size-9 cursor-pointer items-center justify-center rounded-control text-ink-muted outline-none transition-colors hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus"
        :aria-label="t('mist.shell.openRail')"
        :title="t('mist.shell.openRail')"
        aria-expanded="false"
        aria-controls="ccm-next-rail"
        data-rail-toggle
        @click="openRail"
      >
        <PanelLeft :size="16" aria-hidden="true" />
      </button>
      <slot />
      <slot name="drawer" />
    </div>

    <NextQuotaBar v-if="!railHidden" />

    <div hidden aria-hidden="true"><SettingsMenu /></div>
    <InboxPanel v-if="inbox.open && inboxAt" :anchor="inboxAt" :trigger="inboxBtn" @close="inbox.open = false" />
    <GlassMenu :open="moreOpen" :items="moreItems" :label="t('nav.more')" :width="MENU_W" :x="moreAt.x" :y="moreAt.y" @select="pickMore" @close="closeMore" />
  </div>
</template>

<style>
:root[data-ui='next'] .ccm-next-shell {
  display: grid;
  grid-template-columns: var(--ccm-rail-w) minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr) 24px;
  gap: var(--ccm-gap-shell, 12px);
  row-gap: 8px;
  transition: grid-template-columns var(--ccm-dur-base) var(--ccm-ease-enter);
  height: 100dvh;
  padding: 12px 12px 6px;
  box-sizing: border-box;
  overflow: hidden;
}
:root[data-ui='next'] .ccm-next-shell[data-rail-dragging],
:root[data-ui='next'][data-motion='off'] .ccm-next-shell {
  transition: none;
}
:root[data-ui='next'] .ccm-next-footer {
  grid-column: 1 / -1;
}
:root[data-ui='next'] [data-explorer-toggle][aria-pressed='true'] {
  background: var(--ccm-accent-soft);
  color: var(--ccm-accent-ink);
}
:root[data-ui='next'] .ccm-next-shell[data-rail-hidden] {
  grid-template-columns: minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  padding: 0;
}
:root[data-ui='next'] .ccm-next-shell[data-rail-mode='overlay'] {
  grid-template-columns: minmax(0, 1fr);
}
:root[data-ui='next'] .ccm-next-shell[data-rail-mode='overlay']:not([data-rail-hidden]) .ccm-next-canvas {
  padding-left: calc(36px + var(--ccm-gap-shell, 12px));
}
:root[data-ui='next'] .ccm-next-shell[data-title-bar] {
  padding-top: var(--ccm-next-titlebar, 36px);
}
:root[data-ui='next'] .ccm-next-shell[data-title-bar] .ccm-next-rail-overlay {
  top: var(--ccm-next-titlebar, 36px);
}
:root[data-ui='next'] .ccm-next-shell[data-title-bar] [data-rail-toggle] {
  top: var(--ccm-next-titlebar, 36px);
  -webkit-app-region: no-drag;
  app-region: no-drag;
}
:root[data-ui='next'] .ccm-next-drag {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: var(--ccm-next-titlebar, 36px);
  -webkit-app-region: drag;
  app-region: drag;
}
:root[data-ui='next'] .ccm-next-winctl {
  position: fixed;
  top: calc((var(--ccm-next-titlebar, 36px) - 28px) / 2);
  right: 12px;
  z-index: 60;
}
:root[data-ui='next'] .ccm-next-rail-cell {
  position: relative;
  min-height: 0;
  min-width: 0;
}
:root[data-ui='next'] .ccm-next-rail-overlay {
  position: fixed;
  top: 12px;
  bottom: 38px;
  left: 12px;
  width: var(--ccm-rail-w);
  z-index: 40;
  transform: translateX(calc(-100% - 24px));
  transition: transform var(--ccm-dur-base) var(--ccm-ease-enter);
}
:root[data-ui='next'] .ccm-next-shell[data-rail-open] .ccm-next-rail-overlay {
  transform: translateX(0);
}
:root[data-ui='next'] .ccm-next-canvas {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
}
:root[data-ui='next'] .ccm-next-shell[data-rail-dragging] {
  user-select: none;
}
:root[data-ui='next'] .ccm-next-shell[data-rail-dragging] .ccm-next-canvas {
  pointer-events: none;
}
:root[data-ui='next'] .ccm-next-status [data-status-button] > span:not([data-status-dot]):not([data-status-risk]):not([data-offline-label]) {
  display: none;
}
:root[data-ui='next'] [data-tour='ws-sidebar'] {
  display: none;
}
:root[data-ui='next'][data-motion='off'] .ccm-next-rail-overlay {
  transition: none;
}
</style>
