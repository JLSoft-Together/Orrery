<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleDot, Ellipsis, Focus, GitBranch, Hand, ListChecks, Network, X } from 'lucide-vue-next'
import { useFocusMode } from '../composables/useFocusMode'
import FooterSysChip from '../../footerSys/FooterSysChip.vue'
import ShareModeChip from '../../shareMode/ShareModeChip.vue'
import { isMac } from '../../../core/lib/platform'
import { usePaneInfo } from '../composables/usePaneInfo'
import { changedCount, contextPair, devPorts, footerCounts, portsLabel, sessionCounts } from '../lib/footerStatus'
import { planFooterChips, type FooterChipId } from '../lib/footerChips'
import { flashSettingRow } from '@features/settings'
import { useSetupChecklistStore } from '../../settings/lib/setupChecklist'
import { compactNumber } from '../../../core/lib/format'
import { isTerm, workCwd, useWorkspaceStore } from '../stores/workspace'
import { useAgentsStore } from '../../../core/stores/agents'
import { useResourcesStore } from '@features/resources'
import { useSettingsStore } from '../../../core/stores/settings'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useUiStore } from '../../../core/stores/ui'
import { useUsageStore } from '@features/usage'

const { t } = useI18n()
const workspace = useWorkspaceStore()
const info = usePaneInfo()
const agents = useAgentsStore()
const resources = useResourcesStore()
const settings = useSettingsStore()
const focusMode = useFocusMode()
const focusKeys = isMac ? '⌘.' : 'Ctrl+.'
const terminals = useTerminalsStore()
const ui = useUiStore()
const onBoard = computed(() => ui.view === 'monitor')
const usage = useUsageStore()
const setup = useSetupChecklistStore()
setup.ensureLoaded()
const tokensToday = computed(() => (onBoard.value && usage.received && usage.todayTokens > 0 ? compactNumber(usage.todayTokens) : null))

const termPanes = computed(() => workspace.panes.filter(isTerm))
const counts = computed(() =>
  onBoard.value ? sessionCounts(terminals.list) : footerCounts(termPanes.value.map((p) => ({ key: p.key, state: info.railStateOf(p) }))),
)

const ports = computed(() => {
  const servers = resources.report?.servers
  if (!servers?.length) return []
  const ids = new Set<string>()
  const dirs: string[] = []
  if (onBoard.value) {
    for (const s of terminals.list) {
      ids.add(s.id)
      if (s.cwd) dirs.push(s.cwd)
    }
    return devPorts(servers, ids, dirs)
  }
  for (const p of termPanes.value) {
    if (p.terminalId) ids.add(p.terminalId)
    const dir = workCwd(p)
    if (dir) dirs.push(dir)
  }
  return devPorts(servers, ids, dirs)
})

const focused = computed(() => (onBoard.value ? null : workspace.active))
const session = computed(() => info.sessionOf(focused.value ?? undefined))
const branch = computed(() => (focused.value && isTerm(focused.value) ? (focused.value.branch ?? '') : ''))
const changed = computed(() => (branch.value ? changedCount(session.value?.diff) : null))
const context = computed(() => {
  const id = session.value?.id
  const tokens = id ? agents.mainOf(id)?.contextTokens : undefined
  return contextPair(tokens, settings.contextWarnTokens, (n) => compactNumber(n))
})

const contextTokens = computed(() => {
  const id = session.value?.id
  return id ? (agents.mainOf(id)?.contextTokens ?? null) : null
})

const plan = computed(() =>
  planFooterChips({
    running: counts.value.running,
    waiting: counts.value.waiting,
    ports: ports.value.length,
    hasBranch: !!branch.value,
    changed: changed.value,
    contextTokens: context.value ? contextTokens.value : null,
    contextWarn: settings.contextWarnTokens,
    tokensToday: !!tokensToday.value,
    canFocus: !onBoard.value,
    focusOn: focusMode.on.value,
    setupPending: setup.pending.length,
    setupHidden: setup.hidden,
  }),
)
const has = (id: FooterChipId): boolean => plan.value.visible.includes(id)

const branchText = computed(() => {
  if (!branch.value) return ''
  return changed.value === null ? t('wsFooter.branchOnly', { branch: branch.value }) : t('wsFooter.branchChanges', { branch: branch.value, n: changed.value })
})
const branchAria = computed(() =>
  changed.value === null ? t('wsFooter.branchAriaNoCount', { branch: branch.value }) : t('wsFooter.branchAria', { branch: branch.value, n: changed.value }),
)
const contextText = computed(() => {
  const c = context.value
  if (!c) return ''
  return c.limit ? t('wsFooter.context', { used: c.used, limit: c.limit }) : t('wsFooter.contextOnly', { used: c.used })
})
const contextAria = computed(() => {
  const c = context.value
  if (!c) return ''
  return c.limit ? t('wsFooter.contextAria', { used: c.used, limit: c.limit }) : t('wsFooter.contextAriaOnly', { used: c.used })
})

function focusPane(key: string | null): void {
  if (!key) return
  if (!onBoard.value) {
    workspace.activate(key)
    return
  }
  ui.monitorMode = 'board'
  requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-board-card="${CSS.escape(key)}"]`)?.focus())
}

function openChanges(): void {
  const dir = focused.value ? workCwd(focused.value) : undefined
  if (dir) workspace.openChanges(dir)
}

function openSetup(): void {
  ui.settingsPage = 'general'
  ui.settingsOpen = true
  flashSettingRow('setup')
}

const moreOpen = ref(false)
const moreBtn = ref<HTMLElement | null>(null)
const morePanel = ref<HTMLElement | null>(null)
const moreStyle = ref<Record<string, string>>({})

interface MoreRow {
  id: FooterChipId
  icon: Component | null
  text: string
  aria: string
  run: () => void
}

const moreRows = computed<MoreRow[]>(() => {
  const rows: MoreRow[] = []
  for (const id of plan.value.more) {
    if (id === 'ports') rows.push({ id, icon: Network, text: t('wsFooter.ports', { list: portsLabel(ports.value) }), aria: t('wsFooter.portsAria', { list: portsLabel(ports.value) }), run: () => resources.show('ports') })
    else if (id === 'branch') rows.push({ id, icon: GitBranch, text: branchText.value, aria: branchAria.value, run: openChanges })
    else if (id === 'context') rows.push({ id, icon: null, text: contextText.value, aria: contextAria.value, run: () => focusPane(focused.value?.key ?? null) })
    else if (id === 'tokens' && tokensToday.value) rows.push({ id, icon: null, text: t('wsFooter.tokensToday', { n: tokensToday.value }), aria: t('wsFooter.tokensTodayAria', { n: tokensToday.value }), run: () => ui.setView('usage') })
    else if (id === 'focus') rows.push({ id, icon: Focus, text: t('tabsV93.focus'), aria: t('tabsV93.focusAria'), run: focusMode.toggle })
  }
  return rows
})

function placeMore(): void {
  const r = moreBtn.value?.getBoundingClientRect()
  if (!r) return
  moreStyle.value = { right: `${Math.max(8, window.innerWidth - r.right)}px`, bottom: `${Math.max(8, window.innerHeight - r.top + 6)}px` }
}

function closeMore(focusBack = false): void {
  if (!moreOpen.value) return
  moreOpen.value = false
  if (focusBack) moreBtn.value?.focus()
}

function toggleMore(): void {
  if (moreOpen.value) return closeMore()
  placeMore()
  moreOpen.value = true
}

function runMore(row: MoreRow): void {
  closeMore()
  row.run()
}

function onDocPointer(e: PointerEvent): void {
  const el = e.target as Node
  if (morePanel.value?.contains(el) || moreBtn.value?.contains(el)) return
  closeMore()
}

function onDocKey(e: KeyboardEvent): void {
  if (e.key !== 'Escape') return
  e.preventDefault()
  closeMore(true)
}

watch(moreOpen, async (open) => {
  if (open) {
    document.addEventListener('pointerdown', onDocPointer, true)
    document.addEventListener('keydown', onDocKey, true)
    window.addEventListener('resize', placeMore)
    await nextTick()
    morePanel.value?.querySelector<HTMLElement>('button')?.focus()
  } else {
    document.removeEventListener('pointerdown', onDocPointer, true)
    document.removeEventListener('keydown', onDocKey, true)
    window.removeEventListener('resize', placeMore)
  }
})
watch(
  () => moreRows.value.length,
  (n) => n === 0 && closeMore(),
)
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocPointer, true)
  document.removeEventListener('keydown', onDocKey, true)
  window.removeEventListener('resize', placeMore)
})

const btn =
  'ccm-press inline-flex h-full min-w-0 cursor-pointer items-center gap-1 rounded-inner px-1.5 transition-colors duration-(--ccm-dur-fast) hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent'
</script>

<template>
  <footer class="ccm-footer flex shrink-0 items-stretch justify-between gap-3 overflow-hidden px-3" role="contentinfo" :aria-label="t('wsFooter.label')" data-ws-footer>
    <div class="flex min-w-0 items-stretch gap-1">
      <button
        v-if="has('running')"
        type="button"
        :class="btn"
        class="text-st-working"
        :aria-label="t(onBoard ? 'wsFooter.runningAriaBoard' : 'wsFooter.runningAria', { n: counts.running })"
        data-ws-footer-running
        @click="focusPane(counts.firstRunning)"
      >
        <CircleDot :size="12" class="shrink-0" aria-hidden="true" />
        <span class="tabular-nums">{{ t('wsFooter.running', { n: counts.running }) }}</span>
      </button>
      <button
        v-if="has('waiting')"
        type="button"
        :class="btn"
        class="text-st-waiting"
        :aria-label="t(onBoard ? 'wsFooter.waitingAriaBoard' : 'wsFooter.waitingAria', { n: counts.waiting })"
        data-ws-footer-waiting
        @click="focusPane(counts.firstWaiting)"
      >
        <Hand :size="12" class="shrink-0" aria-hidden="true" />
        <span class="tabular-nums">{{ t('wsFooter.waiting', { n: counts.waiting }) }}</span>
      </button>
      <span v-if="has('setup')" class="flex items-stretch" data-ws-footer-setup>
        <button type="button" :class="btn" :aria-label="t('setup.chipAria', { n: setup.pending.length })" data-ws-footer-setup-open @click="openSetup">
          <ListChecks :size="12" class="shrink-0" aria-hidden="true" />
          <span class="truncate tabular-nums">{{ t('setup.chip', { n: setup.pending.length }) }}</span>
        </button>
        <button type="button" :class="btn" :aria-label="t('setup.chipHide')" :title="t('setup.chipHide')" data-ws-footer-setup-hide @click="setup.hideChip">
          <X :size="11" class="shrink-0" aria-hidden="true" />
        </button>
      </span>
      <button
        v-if="has('ports')"
        type="button"
        :class="btn"
        :aria-label="t('wsFooter.portsAria', { list: portsLabel(ports) })"
        data-ws-footer-ports
        @click="resources.show('ports')"
      >
        <Network :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate tabular-nums">{{ t('wsFooter.ports', { list: portsLabel(ports) }) }}</span>
      </button>
    </div>
    <div class="flex min-w-0 items-stretch gap-1">
      <button v-if="has('branch')" type="button" :class="btn" :aria-label="branchAria" data-ws-footer-branch @click="openChanges">
        <GitBranch :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ branchText }}</span>
      </button>
      <button v-if="has('context')" type="button" :class="btn" :aria-label="contextAria" data-ws-footer-context @click="focusPane(focused?.key ?? null)">
        <span class="truncate tabular-nums">{{ contextText }}</span>
      </button>
      <button
        v-if="has('tokens')"
        type="button"
        :class="btn"
        :aria-label="t('wsFooter.tokensTodayAria', { n: tokensToday })"
        data-ws-footer-tokens
        @click="ui.setView('usage')"
      >
        <span class="truncate tabular-nums">{{ t('wsFooter.tokensToday', { n: tokensToday }) }}</span>
      </button>
      <button
        v-if="has('focus')"
        type="button"
        :class="[btn, focusMode.on.value ? 'text-accent-ink' : '']"
        :aria-pressed="focusMode.on.value"
        :aria-label="t('tabsV93.focusAria')"
        :title="t('tabsV93.focusTitle', { keys: focusKeys })"
        data-ws-footer-focus
        @click="focusMode.toggle"
      >
        <Focus :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ t('tabsV93.focus') }}</span>
      </button>
      <button
        v-if="moreRows.length"
        ref="moreBtn"
        type="button"
        :class="btn"
        :aria-label="t('wsFooter.moreAria', { n: moreRows.length })"
        :aria-expanded="moreOpen"
        aria-haspopup="true"
        :title="t('wsFooter.more')"
        data-ws-footer-more
        @click="toggleMore"
      >
        <Ellipsis :size="14" class="shrink-0" aria-hidden="true" />
      </button>
      <ShareModeChip />
      <FooterSysChip />
    </div>
    <Teleport to="body">
      <div
        v-if="moreOpen"
        ref="morePanel"
        class="ccm-sheet fixed z-50 flex min-w-[200px] max-w-[320px] flex-col gap-0.5 p-1"
        :style="moreStyle"
        role="menu"
        :aria-label="t('wsFooter.more')"
        data-ws-footer-popover
      >
        <button
          v-for="r in moreRows"
          :key="r.id"
          type="button"
          role="menuitem"
          class="ccm-press flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 rounded-inner px-2 text-left text-xs text-ink hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent"
          :aria-label="r.aria"
          :data-ws-more-row="r.id"
          @click="runMore(r)"
        >
          <component :is="r.icon" v-if="r.icon" :size="12" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <span class="truncate tabular-nums">{{ r.text }}</span>
        </button>
      </div>
    </Teleport>
  </footer>
</template>
