<script setup lang="ts">
import { Ellipsis, LayoutGrid, Plus, Rows3, Send, SquareTerminal } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import LinkChip from '../components/LinkChip.vue'
import NewPaneSheet from '../components/NewPaneSheet.vue'
import PageBar from '../components/PageBar.vue'
import PaneActionsSheet from '../components/PaneActionsSheet.vue'
import PaneCard from '../components/PaneCard.vue'
import PaneChips from '../components/PaneChips.vue'
import PaneGrid from '../components/PaneGrid.vue'
import SendSheet from '../components/SendSheet.vue'
import UndoBar from '../components/UndoBar.vue'
import { undoOffer } from '../lib/closeUndo'
import { errorCode, reopenPane } from '../lib/paneActions'
import { parseSessions } from '../lib/monitorModel'
import { closedCount, parseWorkspace } from '../lib/workspaceModel'
import { RemoteCommandError } from '../remote/types'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'

const props = defineProps<{ machine: string; selected?: string }>()
const { t } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const tabKey = ref<string | null>(null)
const paneKey = ref<string | null>(null)
const sendOpen = ref(false)
const newOpen = ref(false)
const actionsOpen = ref(false)
const undoOpen = ref(false)
const busy = ref(false)
const fail = ref<string | null>(null)
let startX = 0
let startY = 0

const view = computed(() => hub.viewOf(props.machine))
const link = computed(() => hub.linkOf(props.machine))
const lost = computed(() => link.value.status !== 'ready')
const known = computed(() => hub.machines.find((m) => m.id === props.machine))
const name = computed(() => known.value?.alias ?? known.value?.name ?? props.machine)
const tabs = computed(() => parseWorkspace(view.value.workspace, parseSessions(view.value.snapshot)))
const tab = computed(() => tabs.value.find((x) => x.key === tabKey.value) ?? tabs.value[0] ?? null)
const panes = computed(() => tab.value?.panes ?? [])
const pane = computed(() => panes.value.find((p) => p.key === paneKey.value) ?? panes.value[0] ?? null)
const index = computed(() => (pane.value ? panes.value.indexOf(pane.value) : -1))
const closed = computed(() => closedCount(view.value.workspace))
const grid = computed(() => prefs.local.workspaceMode === 'grid')
const viewOnly = computed(() => hub.isViewOnly(props.machine))

watch(
  () => undoOffer.value.seq,
  () => {
    if (undoOffer.value.machine === props.machine) undoOpen.value = true
  },
)

watch(
  tabs,
  (list) => {
    if (tabKey.value && !list.some((x) => x.key === tabKey.value)) tabKey.value = null
  },
)

function open(key: string): void {
  navigate({ name: 'workspace', machine: props.machine, pane: key })
}

function step(by: number): void {
  const next = panes.value[index.value + by]
  if (next) paneKey.value = next.key
}

function touchStart(event: TouchEvent): void {
  startX = event.touches[0]?.clientX ?? 0
  startY = event.touches[0]?.clientY ?? 0
}

function touchEnd(event: TouchEvent): void {
  const touch = event.changedTouches[0]
  if (!touch) return
  const dx = touch.clientX - startX
  const dy = touch.clientY - startY
  if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy) * 1.5) step(dx < 0 ? 1 : -1)
}

function toggleMode(): void {
  prefs.setWorkspaceMode(grid.value ? 'focus' : 'grid')
}

function failText(error: unknown): string {
  return t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
}

function openActions(key: string): void {
  paneKey.value = key
  actionsOpen.value = true
}

async function undo(): Promise<void> {
  undoOpen.value = false
  try {
    await reopenPane((type, fields) => hub.command(props.machine, type, fields))
  } catch (error) {
    fail.value = t(`err.${errorCode(error)}`)
  }
}

async function send(text: string): Promise<void> {
  if (!pane.value) return
  busy.value = true
  fail.value = null
  try {
    await hub.command(props.machine, 'ws.send', { paneKey: pane.value.key, text })
    sendOpen.value = false
  } catch (error) {
    fail.value = failText(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="page" data-testid="workspace-view" :data-machine="machine" :data-mode="grid ? 'grid' : 'focus'">
    <PageBar :title="t('tabs.workspace')" backless>
      <template #right>
        <LinkChip :link="link" :rtt="hub.rtt[machine]" />
        <button type="button" class="icon" :aria-label="t(grid ? 'workspace.modeFocus' : 'workspace.modeGrid')" data-testid="ws-mode" @click="toggleMode">
          <Rows3 v-if="grid" :size="22" :stroke-width="2" aria-hidden="true" />
          <LayoutGrid v-else :size="22" :stroke-width="2" aria-hidden="true" />
        </button>
      </template>
    </PageBar>
    <p class="machine mono">{{ name }}</p>
    <p v-if="viewOnly" class="note" role="status" data-testid="view-only">{{ t('term.viewOnly') }}</p>

    <div v-if="tabs.length === 0" class="empty" data-testid="ws-empty">
      <SquareTerminal :size="32" :stroke-width="1.8" aria-hidden="true" />
      <h2>{{ view.workspace ? t('workspace.emptyTitle') : t('workspace.loading') }}</h2>
      <p v-if="view.workspace">{{ t('workspace.emptyBody') }}</p>
    </div>
    <template v-else>
      <div class="tabs" role="tablist" data-testid="ws-tabs">
        <button v-for="x in tabs" :key="x.key" type="button" role="tab" class="tab" :class="{ on: x.key === tab?.key }" :aria-selected="x.key === tab?.key" :data-testid="`ws-tab-${x.key}`" @click="((tabKey = x.key), (paneKey = null))"><Transition name="swap" mode="out-in"><span :key="x.name">{{ x.name }}</span></Transition></button>
      </div>

      <template v-if="!grid">
        <PaneChips :panes="panes" :current="pane?.key" @pick="paneKey = $event" />
        <div v-if="pane" class="focus" data-testid="ws-focus" :data-pane="pane.key" @touchstart.passive="touchStart" @touchend.passive="touchEnd">
          <PaneCard :pane="pane" :lost="lost" large :selected="selected === pane.key" @open="open(pane.key)" />
          <div class="actions">
            <button type="button" class="btn btn-primary full" :disabled="!pane.canAttach" data-testid="ws-open" @click="open(pane.key)"><SquareTerminal :size="20" :stroke-width="2" aria-hidden="true" />{{ t('workspace.openFull') }}</button>
            <button type="button" class="btn sendbtn" :disabled="lost || viewOnly" :aria-label="t('send.title')" data-testid="ws-send" @click="((sendOpen = true), (fail = null))"><Send :size="20" :stroke-width="2" aria-hidden="true" /></button>
            <button type="button" class="btn sendbtn" :disabled="lost || viewOnly" :aria-label="t('paneActions.title')" data-testid="ws-more" @click="openActions(pane.key)"><Ellipsis :size="20" :stroke-width="2" aria-hidden="true" /></button>
          </div>
          <p class="hint">{{ t('workspace.swipeHint', { n: panes.length }) }}</p>
        </div>
      </template>

      <PaneGrid v-else :panes="panes" :lost="lost" :view-only="viewOnly" :selected="selected" @open="open" @actions="openActions" />

      <button v-if="tab?.cwd && !viewOnly" type="button" class="fab btn btn-primary" :disabled="lost" data-testid="ws-new" @click="newOpen = true"><Plus :size="20" :stroke-width="2.2" aria-hidden="true" />{{ t('workspace.newPane') }}</button>
    </template>

    <PaneActionsSheet v-if="actionsOpen && tab && pane" :machine="machine" :tab="tab" :tabs="tabs" :pane="pane" :closed="closed" @close="actionsOpen = false" @closed-pane="undoOpen = true" />
    <Transition name="undo"><UndoBar v-if="undoOpen" :text="t('paneActions.closedToast')" :action="t('paneActions.undo')" @undo="undo" @dismiss="undoOpen = false" /></Transition>
    <SendSheet v-if="sendOpen && pane" :title="t('send.titleFor', { name: pane.title })" :busy="busy" :error="fail" @send="send" @close="sendOpen = false" />
    <NewPaneSheet v-if="newOpen && tab?.cwd" :machine="machine" :cwd="tab.cwd" @close="newOpen = false" @opened="(key) => key && (paneKey = key)" />
  </section>
</template>

<style scoped>
.page {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 88px;
}

.icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
}

.machine {
  margin: -8px 0 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-faint);
}

.note {
  margin: 0;
  padding: 8px 12px;
  border-radius: var(--radius-control);
  background: color-mix(in srgb, var(--ccm-waiting) 10%, var(--ccm-surface));
  color: var(--ccm-waiting);
  font-weight: 600;
}

.tabs {
  display: flex;
  gap: 4px;
  overflow-x: auto;
  scrollbar-width: none;
  border-bottom: 1px solid var(--ccm-line);
}

.tab {
  flex: none;
  min-height: 44px;
  padding: 0 14px;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--ccm-ink-muted);
  font-weight: 600;
}

.swap-enter-active,
.swap-leave-active {
  transition: opacity 160ms var(--ease-out), transform 160ms var(--ease-out);
}

.swap-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.swap-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.tab.on {
  color: var(--ccm-ink);
  border-bottom-color: var(--ccm-accent);
}

.focus {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.actions {
  display: flex;
  gap: 8px;
}

.full {
  flex: 1;
}

.sendbtn {
  width: 52px;
  padding: 0;
}

.hint {
  margin: 0;
  text-align: center;
  font-size: 0.8125rem;
  color: var(--ccm-ink-faint);
}

.undo-enter-active,
.undo-leave-active {
  transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out);
}

.undo-enter-from,
.undo-leave-to {
  opacity: 0;
  transform: translateY(12px);
}

.fab {
  position: fixed;
  right: 16px;
  bottom: calc(76px + env(safe-area-inset-bottom));
  min-height: 52px;
  padding: 0 20px;
  border-radius: 26px;
  z-index: 5;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 64px 16px;
  text-align: center;
  color: var(--ccm-ink-muted);
}

.empty h2 {
  margin: 8px 0 0;
  font-size: 1.125rem;
  color: var(--ccm-ink);
}

.empty p {
  margin: 0;
  max-width: 30ch;
}

@media (min-width: 768px) {
  .fab {
    right: calc(100% - 380px + 16px);
  }
}
</style>
