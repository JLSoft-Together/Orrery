<script setup lang="ts">
import { Activity, ChevronDown, Ellipsis, Gauge, LayoutGrid, Laptop, Plus, Search, SlidersHorizontal, SquareTerminal } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AgentGlyph from '../components/AgentGlyph.vue'
import BottomSheet from '../components/BottomSheet.vue'
import NewSessionSheet from '../components/NewSessionSheet.vue'
import QuotaBars from '../components/QuotaBars.vue'
import RailText from '../components/RailText.vue'
import { chatRows, filterRows, firstCwd, shortAgo, sortRows, type ChatRow } from '../lib/chatInbox'
import { chatSupported } from '../lib/chatModel'
import { parseSessions } from '../lib/monitorModel'
import { quotaOf } from '../lib/quotaModel'
import { useNow } from '../lib/useNow'
import { parseWorkspace } from '../lib/workspaceModel'
import { RemoteCommandError } from '../remote/types'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'

const props = defineProps<{ machine: string; selected?: string }>()
const { t } = useI18n()
const hub = useHubStore()
const now = useNow(60_000)
const searching = ref(false)
const query = ref('')
const more = ref(false)
const quotaOpen = ref(false)
const creating = ref(false)
const shellBusy = ref(false)
const fail = ref<string | null>(null)

const known = computed(() => hub.machines.find((m) => m.id === props.machine))
const name = computed(() => known.value?.alias ?? known.value?.name ?? props.machine)
const view = computed(() => hub.viewOf(props.machine))
const link = computed(() => hub.linkOf(props.machine))
const online = computed(() => link.value.status === 'ready')
const viewOnly = computed(() => hub.isViewOnly(props.machine))
const supported = computed(() => chatSupported(view.value.hello))
const tabs = computed(() => parseWorkspace(view.value.workspace, parseSessions(view.value.snapshot)))
const rows = computed(() => chatRows(tabs.value, view.value.snapshot, view.value.chatLast))
const shown = computed(() => sortRows(filterRows(rows.value, query.value)))
const quota = computed(() => quotaOf(view.value.snapshot, now.value))
const hot = computed(() => quota.value.filter((q) => q.pct >= 80 && !q.stale))
const places = computed(() => {
  const seen = new Set<string>()
  const out: string[] = []
  const recent = firstCwd(tabs.value, rows.value)
  for (const cwd of [recent, ...[...rows.value].sort((a, b) => (b.at ?? 0) - (a.at ?? 0)).map((r) => r.cwd), ...tabs.value.map((x) => x.cwd)]) {
    if (!cwd || seen.has(cwd)) continue
    seen.add(cwd)
    out.push(cwd)
  }
  return out
})
const canOpen = computed(() => online.value && !viewOnly.value && places.value.length > 0)
const waitingState = computed(() => (link.value.status === 'reconnecting' || link.value.status === 'discovering' || link.value.status === 'handshaking' ? 'reconnecting' : 'offline'))

function ago(row: ChatRow): string {
  return row.at ? shortAgo(Math.max(0, now.value - row.at), t) : ''
}

function bottom(row: ChatRow): string {
  if (row.rail === 'working' && row.tool) return row.tool
  if (row.last) return row.last
  if (row.rail === 'asked' && row.waitText && !['permission', 'question', 'other'].includes(row.waitText)) return row.waitText
  return row.tool ?? ''
}

function openRow(row: ChatRow): void {
  if (row.canChat && supported.value) navigate({ name: 'chat', machine: props.machine, pane: row.key })
  else navigate({ name: 'workspace', machine: props.machine, pane: row.key })
}

function toggleSearch(): void {
  searching.value = !searching.value
  if (!searching.value) query.value = ''
}

function go(target: 'workspace' | 'monitor' | 'settings' | 'home'): void {
  more.value = false
  if (target === 'settings') navigate({ name: 'settings' })
  else if (target === 'home') navigate({ name: 'home' })
  else navigate({ name: target, machine: props.machine })
}

async function newShell(): Promise<void> {
  const cwd = places.value[0]
  if (!cwd || shellBusy.value) return
  shellBusy.value = true
  fail.value = null
  try {
    const reply = await hub.command(props.machine, 'ws.open', { cwd, preset: 'shell' })
    if (typeof reply.paneKey === 'string') navigate({ name: 'workspace', machine: props.machine, pane: reply.paneKey })
  } catch (error) {
    fail.value = t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
  } finally {
    shellBusy.value = false
  }
}

function opened(paneKey: string | null, preset: 'claude' | 'codex' | 'shell'): void {
  creating.value = false
  if (!paneKey) return
  if (preset !== 'shell' && supported.value) navigate({ name: 'chat', machine: props.machine, pane: paneKey })
  else navigate({ name: 'workspace', machine: props.machine, pane: paneKey })
}
</script>

<template>
  <section class="page" data-testid="chat-home" :data-machine="machine" :data-chat="supported ? '1' : '0'">
    <header class="bar">
      <button type="button" class="machine" :aria-label="t('chat.machines')" data-testid="back" @click="navigate({ name: 'home' })">
        <span class="m-dot" :class="{ off: !online }" aria-hidden="true" />
        <span class="m-name">{{ name }}</span>
        <ChevronDown :size="18" :stroke-width="2" aria-hidden="true" />
      </button>
      <div class="actions">
        <button type="button" class="icon-btn" :aria-label="t('chat.search')" :aria-expanded="searching" data-testid="chat-search" @click="toggleSearch">
          <Search :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" :aria-label="t('chat.newShell')" :disabled="!canOpen || shellBusy" data-testid="new-shell" @click="newShell">
          <SquareTerminal :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" :aria-label="t('chat.more')" data-testid="chat-more" @click="more = true">
          <Ellipsis :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" :aria-label="t('chat.newSession')" :disabled="!canOpen" data-testid="chat-new" @click="creating = true">
          <Plus :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
      </div>
    </header>
    <p v-if="!online" class="reconnect" role="status" data-testid="chat-home-offline">
      <span v-if="waitingState === 'reconnecting'" class="spinner" aria-hidden="true" />{{ t(`chat.${waitingState}`) }}
    </p>

    <div class="scroll">
      <h1 class="title">{{ t('chat.inbox') }}</h1>
      <button v-for="q in hot" :key="`${q.agent}-${q.window}`" type="button" class="quota-line" :data-testid="`quota-hot-${q.agent}-${q.window}`" @click="quotaOpen = true">
        <Gauge :size="18" :stroke-width="2" aria-hidden="true" />
        <span>{{ t('chat.quotaHot', { agent: t(`quota.agent.${q.agent}`), window: t(q.window === '5h' ? 'quota.window5h' : 'quota.windowWeek') }) }} <b>{{ q.pct }}%</b></span>
        <span class="q-bar" aria-hidden="true"><i :style="{ width: `${q.pct}%` }" /></span>
      </button>
      <label v-if="searching" class="search">
        <Search :size="18" :stroke-width="2" aria-hidden="true" />
        <input v-model="query" type="search" :placeholder="t('chat.search')" :aria-label="t('chat.search')" autocomplete="off" data-testid="chat-search-input" />
      </label>
      <p v-if="fail" class="fail" role="alert">{{ fail }}</p>

      <template v-if="rows.length === 0">
        <div class="empty" data-testid="chat-home-empty">
          <p>{{ t('chat.empty') }}</p>
          <button v-if="canOpen" type="button" class="btn btn-primary" @click="creating = true"><Plus :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.newSession') }}</button>
        </div>
      </template>
      <p v-else-if="shown.waiting.length + shown.rest.length === 0" class="empty">{{ t('chat.noMatch') }}</p>

      <template v-if="shown.waiting.length">
        <h2 class="group-label wait" data-testid="chat-group-waiting">{{ t('chat.waiting', { n: shown.waiting.length }) }}</h2>
        <div class="card-list wait">
          <button v-for="row in shown.waiting" :key="row.key" type="button" class="row" :aria-current="selected === row.key ? 'true' : undefined" :data-testid="`chat-row-${row.key}`" :data-rail="row.rail" @click="openRow(row)">
            <AgentGlyph :provider="row.provider" avatar />
            <span class="row-main">
              <span class="row-top"><span class="row-name">{{ row.title }}</span><span class="row-meta"><span v-if="row.project" class="proj">{{ row.project }}</span><span>{{ ago(row) }}</span></span></span>
              <span class="row-bot"><RailText :rail="row.rail" /><span class="row-last" data-testid="chat-row-last">{{ bottom(row) }}</span></span>
            </span>
          </button>
        </div>
      </template>
      <template v-if="shown.rest.length">
        <h2 v-if="shown.waiting.length" class="group-label">{{ t('chat.recent') }}</h2>
        <div class="card-list" :class="{ first: !shown.waiting.length }">
          <button v-for="row in shown.rest" :key="row.key" type="button" class="row" :aria-current="selected === row.key ? 'true' : undefined" :data-testid="`chat-row-${row.key}`" :data-rail="row.rail" @click="openRow(row)">
            <AgentGlyph :provider="row.provider" avatar />
            <span class="row-main">
              <span class="row-top"><span class="row-name">{{ row.title }}</span><span class="row-meta"><span v-if="row.project" class="proj">{{ row.project }}</span><span>{{ ago(row) }}</span></span></span>
              <span class="row-bot"><RailText v-if="row.rail !== 'idle'" :rail="row.rail" :text="row.rail !== 'working'" /><span class="row-last" data-testid="chat-row-last">{{ bottom(row) }}</span></span>
            </span>
          </button>
        </div>
      </template>
    </div>

    <BottomSheet v-if="more" :title="name" testid="chat-more-sheet" @close="more = false">
      <div class="s-list">
        <button type="button" class="s-row" data-testid="more-workspace" @click="go('workspace')"><LayoutGrid :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.workspace') }}</span></button>
        <button type="button" class="s-row" data-testid="more-monitor" @click="go('monitor')"><Activity :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.monitor') }}</span></button>
        <button type="button" class="s-row" data-testid="more-quota" @click="((more = false), (quotaOpen = true))"><Gauge :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.quota') }}</span></button>
        <button type="button" class="s-row" data-testid="more-settings" @click="go('settings')"><SlidersHorizontal :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.settings') }}</span></button>
        <button type="button" class="s-row" data-testid="more-machines" @click="go('home')"><Laptop :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.machines') }}</span></button>
      </div>
    </BottomSheet>
    <BottomSheet v-if="quotaOpen" :title="t('chat.menu.quota')" testid="chat-quota-sheet" @close="quotaOpen = false">
      <QuotaBars v-if="quota.length" :rows="quota" :now="now" />
      <p v-else class="none">{{ t('chat.noQuota') }}</p>
    </BottomSheet>
    <NewSessionSheet v-if="creating" :machine="machine" :places="places" @close="creating = false" @opened="opened" />
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  background: var(--ccm-canvas);
}

.bar {
  position: sticky;
  top: 0;
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: 56px;
  padding: 4px 8px;
  background: var(--ccm-canvas);
}

.machine {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  min-height: 44px;
  padding: 0 12px;
  border: 0;
  border-radius: 22px;
  background: transparent;
  color: var(--ccm-ink);
  font-size: 1rem;
  font-weight: 600;
}

.machine svg {
  color: var(--ccm-ink-muted);
}

.m-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.m-dot {
  width: 8px;
  height: 8px;
  flex: none;
  border-radius: 50%;
  background: var(--ccm-done);
}

.m-dot.off {
  background: transparent;
  box-shadow: inset 0 0 0 1.5px var(--ccm-warning);
}

.actions {
  display: flex;
  margin-left: auto;
}

.icon-btn {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--ccm-ink);
}

.icon-btn:disabled {
  color: var(--ccm-ink-faint);
}

.reconnect {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 0;
  min-height: 28px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--ccm-warning);
}

.spinner {
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.scroll {
  flex: 1;
  padding: 0 12px 24px;
}

.title {
  margin: 6px 8px 6px;
  font-size: 1.875rem;
  line-height: 1.2;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.quota-line {
  display: flex;
  align-items: center;
  gap: 8px;
  width: calc(100% - 16px);
  min-height: 44px;
  margin: 4px 8px;
  padding: 0 12px;
  border: 0;
  border-radius: 22px;
  background: var(--ccm-warning-soft);
  color: var(--ccm-ink);
  font-size: 0.875rem;
  text-align: left;
}

.quota-line svg,
.quota-line b {
  color: var(--ccm-warning);
}

.q-bar {
  flex: 1;
  max-width: 80px;
  height: 5px;
  margin-left: auto;
  overflow: hidden;
  border-radius: 3px;
  background: var(--ccm-line-strong);
}

.q-bar i {
  display: block;
  height: 100%;
  background: var(--ccm-warning);
}

.search {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 44px;
  margin: 8px;
  padding: 0 14px;
  border-radius: 22px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
  color: var(--ccm-ink-muted);
}

.search input {
  flex: 1;
  min-width: 0;
  border: 0;
  background: none;
  outline: none;
  color: var(--ccm-ink);
  font-size: 16px;
}

.fail {
  margin: 8px;
  color: var(--ccm-error);
  font-weight: 600;
}

.group-label {
  margin: 22px 16px 8px;
  font-size: 0.8125rem;
  font-weight: 700;
  color: var(--ccm-ink-muted);
}

.group-label.wait {
  color: var(--ccm-waiting);
}

.card-list {
  display: flex;
  flex-direction: column;
  padding: 4px;
  border-radius: 26px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.card-list.first {
  margin-top: 14px;
}

.card-list.wait {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ccm-waiting) 28%, transparent);
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 68px;
  padding: 10px 12px;
  border: 0;
  border-radius: 22px;
  background: transparent;
  color: var(--ccm-ink);
  text-align: left;
}

.row:active,
.row[aria-current='true'] {
  background: var(--ccm-hover);
}

.row-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.row-top,
.row-bot {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.row-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 1rem;
  font-weight: 600;
}

.row-meta {
  display: inline-flex;
  flex: none;
  gap: 6px;
  font-size: 0.78rem;
  color: var(--ccm-ink-muted);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.proj {
  max-width: 84px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row-bot {
  align-items: center;
  font-size: 0.875rem;
  color: var(--ccm-ink-muted);
}

.row-last {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  margin: 48px 16px;
  color: var(--ccm-ink-muted);
  text-align: center;
}

.empty p {
  margin: 0;
}

.s-list {
  display: flex;
  flex-direction: column;
}

.s-row {
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 52px;
  padding: 0 12px;
  border: 0;
  border-radius: 16px;
  background: transparent;
  color: var(--ccm-ink);
  font-size: 1rem;
  text-align: left;
}

.s-row svg {
  color: var(--ccm-ink-muted);
}

.s-row:active {
  background: var(--ccm-hover);
}

.none {
  margin: 0;
  color: var(--ccm-ink-muted);
}

@media (prefers-reduced-motion: reduce) {
  .spinner {
    animation: none;
  }
}
</style>
