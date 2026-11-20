<script setup lang="ts">
import { Check, Clock, GitBranch, Inbox, Info, LoaderCircle, MoonStar, RefreshCw, TriangleAlert, WifiOff } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import InboxSheet from '../components/InboxSheet.vue'
import LinkChip from '../components/LinkChip.vue'
import PageBar from '../components/PageBar.vue'
import SessionRow from '../components/SessionRow.vue'
import { inboxRows, type InboxRow } from '../lib/inboxModel'
import { filterCounts, groupByState, matchesFilter, parseSessions, sortSessions, type MonitorFilter, type Rail } from '../lib/monitorModel'
import { groupByProject } from '../lib/projectModel'
import { useNow } from '../lib/useNow'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'

const props = defineProps<{ machine: string; selected?: string }>()
const { t, locale } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const now = useNow()
const filter = ref<MonitorFilter>('all')
const filters: MonitorFilter[] = ['all', 'running', 'waiting', 'idle']
const filterIcon: Record<MonitorFilter, typeof Clock | null> = { all: null, running: LoaderCircle, waiting: Clock, idle: MoonStar }

const known = computed(() => hub.machines.find((m) => m.id === props.machine))
const name = computed(() => known.value?.alias ?? known.value?.name ?? props.machine)
const link = computed(() => hub.linkOf(props.machine))
const view = computed(() => hub.viewOf(props.machine))
const lost = computed(() => link.value.status !== 'ready')
const sessions = computed(() => sortSessions(parseSessions(view.value.snapshot)))
const counts = computed(() => filterCounts(sessions.value))
const shown = computed(() => sessions.value.filter((s) => matchesFilter(filter.value, s)))
const projects = computed(() => groupByProject(shown.value))
const groups = computed(() => groupByState(sessions.value))
const grouped = computed(() => prefs.local.monitorMode === 'groups')
const updated = computed(() =>
  view.value.updatedAt ? new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit' }).format(view.value.updatedAt) : null,
)
const sub = computed(() => {
  if (view.value.stale && updated.value) return t('monitor.staleSub')
  return updated.value ? t('monitor.updated', { time: updated.value }) : undefined
})
const inbox = computed(() => inboxRows(view.value.workspace))
const inboxOpen = ref(false)
const reasonKey = computed(() => (link.value.reason !== 'none' ? `reason.${link.value.reason}` : null))
const groupMeta: Record<Rail, { icon: typeof Clock; tone: string }> = {
  failed: { icon: TriangleAlert, tone: 'error' },
  asked: { icon: Clock, tone: 'waiting' },
  working: { icon: LoaderCircle, tone: 'working' },
  done: { icon: Check, tone: 'done' },
  idle: { icon: MoonStar, tone: 'idle' },
}

function open(id: string): void {
  navigate({ name: 'session', machine: props.machine, terminal: id })
}

function openInbox(row: InboxRow): void {
  inboxOpen.value = false
  navigate(row.terminalId ? { name: 'session', machine: props.machine, terminal: row.terminalId } : { name: 'workspace', machine: props.machine, pane: row.paneKey })
}

function megabytes(bytes: number): string {
  return `${new Intl.NumberFormat(locale.value, { maximumFractionDigits: 1 }).format(bytes / (1 << 20))} MB`
}

function setMode(mode: 'cards' | 'groups'): void {
  if ((mode === 'groups') !== grouped.value) prefs.setMonitorMode(mode)
}
</script>

<template>
  <section class="page" data-testid="monitor-view" :data-mode="grouped ? 'groups' : 'cards'" :data-machine="machine">
    <PageBar :title="name" :sub="sub">
      <template #right>
        <button type="button" class="inbox-btn" :aria-label="t('inbox.open')" data-testid="inbox-open" @click="inboxOpen = true">
          <Inbox :size="20" :stroke-width="2.2" aria-hidden="true" />
          <span v-if="inbox.length > 0" class="inbox-n" data-testid="inbox-count">{{ inbox.length }}</span>
        </button>
        <LinkChip :link="link" :rtt="hub.rtt[machine]" />
      </template>
      <template #below>
        <div class="seg" role="tablist" :aria-label="t('monitor.modeLabel')" data-testid="mode-seg">
          <button type="button" role="tab" class="seg-btn" :class="{ on: !grouped }" :aria-selected="!grouped" data-testid="mode-cards" @click="setMode('cards')">{{ t('monitor.modeProject') }}</button>
          <button type="button" role="tab" class="seg-btn" :class="{ on: grouped }" :aria-selected="grouped" data-testid="mode-groups" @click="setMode('groups')">{{ t('monitor.modeStatus') }}</button>
        </div>
      </template>
    </PageBar>

    <div v-if="lost && link.status !== 'idle'" class="banner offline" role="status" data-testid="monitor-offline">
      <WifiOff :size="22" :stroke-width="2" aria-hidden="true" />
      <div class="banner-text">
        <strong>{{ t('monitor.lostTitle', { name }) }}</strong>
        <span>{{ reasonKey ? t(reasonKey) : t('monitor.lostBody') }}<template v-if="updated">, {{ t('monitor.dataAt', { time: updated }) }}</template></span>
      </div>
      <button type="button" class="banner-btn" data-testid="monitor-retry" @click="hub.refresh(machine)">{{ t('monitor.retry') }}</button>
    </div>

    <div v-if="view.limit" class="banner limit" role="alert" data-testid="monitor-limit">
      <TriangleAlert :size="22" :stroke-width="2" aria-hidden="true" />
      <div class="banner-text">
        <strong>{{ t('monitor.limitTitle') }}</strong>
        <span>{{ t('monitor.limitBody', { size: megabytes(view.limit.bytes), max: megabytes(view.limit.max) }) }}</span>
        <button type="button" class="retry" data-testid="monitor-limit-retry" @click="hub.refresh(machine)"><RefreshCw :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('monitor.retry') }}</button>
      </div>
    </div>
    <p v-else-if="view.trimmed" class="note" role="status" data-testid="monitor-trimmed">
      <Info :size="16" :stroke-width="2" aria-hidden="true" />
      <span>{{ t('monitor.trimmed', { agents: view.trimmed.agents, activity: view.trimmed.activity, text: view.trimmed.text ? t('monitor.trimmedText') : '' }) }}</span>
    </p>

    <p v-if="!known" class="empty" data-testid="monitor-unknown">{{ t('monitor.unknown') }}</p>
    <p v-else-if="sessions.length === 0" class="empty" data-testid="monitor-empty">{{ t(view.snapshot ? 'monitor.empty' : 'machines.noData') }}</p>

    <template v-else-if="!grouped">
      <div class="filters" role="tablist" data-testid="monitor-filters">
        <button v-for="f in filters" :key="f" type="button" role="tab" class="filter" :class="{ on: filter === f }" :aria-selected="filter === f" :data-testid="`filter-${f}`" @click="filter = f">
          <component :is="filterIcon[f]" v-if="filterIcon[f]" :size="18" :stroke-width="2.2" aria-hidden="true" />
          {{ t(`filter.${f}`) }} <span class="n">{{ counts[f] }}</span>
        </button>
      </div>
      <TransitionGroup name="plist" tag="div" class="list" data-testid="monitor-list" :class="{ dim: lost }">
        <section v-for="p in projects" :key="p.key" class="card project" :data-testid="`project-${p.key || 'none'}`">
          <header class="p-head">
            <span class="tone-mono" :data-tone="p.tone" aria-hidden="true">{{ p.letter }}</span>
            <span class="p-name">{{ p.name ?? t('monitor.noProject') }}</span>
            <span v-if="p.branch" class="p-branch"><GitBranch :size="16" :stroke-width="2" aria-hidden="true" /><span class="p-branch-text">{{ p.branch }}</span></span>
            <span class="p-count">{{ p.sessions.length }}</span>
          </header>
          <TransitionGroup name="srow" tag="div" class="rows">
            <SessionRow v-for="s in p.sessions" :key="s.id" :session="s" :now="now" :lost="lost" :class="{ picked: s.id === selected }" @open="open(s.id)" />
          </TransitionGroup>
        </section>
        <p v-if="shown.length === 0" key="empty" class="empty">{{ t('monitor.emptyFilter') }}</p>
      </TransitionGroup>
    </template>

    <div v-else class="groups" data-testid="monitor-groups">
      <section v-for="g in groups" :key="g.rail" class="group" :data-rail="g.rail">
        <h2 class="group-head" :class="`gtone-${groupMeta[g.rail].tone}`">
          <component :is="groupMeta[g.rail].icon" :size="16" :stroke-width="2.2" aria-hidden="true" />
          <span class="gl">{{ t(`status.${g.rail}`) }} {{ g.sessions.length }}</span>
        </h2>
        <div class="card group-card">
          <SessionRow v-for="s in g.sessions" :key="s.id" prefix="row" :session="s" :now="now" :lost="lost" :class="{ picked: s.id === selected }" @open="open(s.id)" />
        </div>
      </section>
    </div>
    <InboxSheet v-if="inboxOpen" :rows="inbox" :now="now" @pick="openInbox" @close="inboxOpen = false" />
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 16px;
}

.inbox-btn {
  position: relative;
  display: inline-grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 1px solid var(--ccm-line-strong);
  border-radius: var(--radius-pill);
  background: var(--ccm-glass);
  color: var(--ccm-ink);
}

.inbox-n {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--ccm-waiting);
  color: var(--ccm-material);
  font-size: 0.6875rem;
  font-weight: 800;
  line-height: 18px;
  text-align: center;
}

.seg {
  display: flex;
  padding: 3px;
  border-radius: var(--radius-control);
  background: var(--ccm-fill);
  border: 1px solid var(--ccm-glass-edge);
}

.seg-btn {
  flex: 1;
  min-height: 38px;
  border: 0;
  border-radius: calc(var(--radius-control) - 3px);
  background: transparent;
  font-weight: 700;
  color: var(--ccm-ink-muted);
  transition:
    background-color var(--ccm-dur-base) var(--ccm-ease-standard),
    color var(--ccm-dur-base) var(--ccm-ease-standard),
    box-shadow var(--ccm-dur-base) var(--ccm-ease-standard);
}

.seg-btn.on {
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  box-shadow: 0 1px 3px color-mix(in srgb, var(--ccm-ink) 16%, transparent);
}

.banner {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  border-radius: var(--radius-card);
  border: 1px solid color-mix(in srgb, var(--ccm-error) 38%, transparent);
  background: color-mix(in srgb, var(--ccm-error) 10%, var(--ccm-glass));
  color: var(--ccm-error);
  animation: banner-in var(--ccm-dur-slow) var(--ccm-ease-enter) both;
}

.banner.offline {
  align-items: center;
  border-color: color-mix(in srgb, var(--ccm-warning) 40%, transparent);
  background: color-mix(in srgb, var(--ccm-warning) 10%, var(--ccm-glass));
  color: var(--ccm-warning);
}

.banner-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  font-size: 0.875rem;
  color: var(--ccm-ink-muted);
}

.banner-text strong {
  color: var(--ccm-error);
  font-size: 1rem;
}

.offline .banner-text strong {
  color: var(--ccm-ink);
}

.banner-btn {
  min-height: 44px;
  padding: 0 8px;
  border: 0;
  background: transparent;
  color: var(--ccm-accent-ink);
  font-weight: 700;
}

.retry {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 44px;
  margin-top: 6px;
  padding: 0 18px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-weight: 700;
}

.note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.note svg {
  flex: none;
  margin-top: 2px;
}

.filters {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  margin: 0 -16px;
  padding: 2px 16px;
  scrollbar-width: none;
}

.filters::-webkit-scrollbar {
  display: none;
}

.filter {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 44px;
  padding: 0 16px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-glass);
  font-weight: 700;
  transition:
    background-color var(--ccm-dur-base) var(--ccm-ease-standard),
    border-color var(--ccm-dur-base) var(--ccm-ease-standard),
    color var(--ccm-dur-base) var(--ccm-ease-standard);
}

.filter .n {
  font-variant-numeric: tabular-nums;
}

.filter.on {
  background: var(--ccm-accent-soft);
  border-color: var(--ccm-accent);
  color: var(--ccm-accent-ink);
}

.list {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.list.dim {
  opacity: 0.82;
}

.project {
  overflow: hidden;
}

.p-head {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--ccm-line);
}

.tone-mono {
  width: 32px;
  height: 32px;
  font-size: 0.9375rem;
}

.p-name {
  flex: 1;
  min-width: 0;
  font-size: 1.0625rem;
  font-weight: 800;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.p-branch {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  max-width: 50%;
  color: var(--ccm-ink-muted);
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.p-branch svg {
  flex: none;
}

.p-branch-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.p-count {
  flex: none;
  min-width: 1.5rem;
  color: var(--ccm-ink-muted);
  font-weight: 700;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.rows {
  position: relative;
  display: flex;
  flex-direction: column;
}

.picked {
  background: var(--ccm-accent-soft);
}

.groups {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px 4px;
  font-size: 0.875rem;
  font-weight: 800;
}

.gtone-error {
  color: var(--ccm-error);
}

.gtone-waiting {
  color: var(--ccm-waiting);
}

.gtone-working {
  color: var(--ccm-working);
}

.gtone-done {
  color: var(--ccm-done);
}

.gtone-idle {
  color: var(--ccm-idle);
}

.group-card {
  overflow: hidden;
}

.empty {
  margin: 24px 0;
  text-align: center;
  color: var(--ccm-ink-muted);
}

.plist-enter-active,
.plist-leave-active,
.srow-enter-active,
.srow-leave-active {
  transition:
    opacity var(--ccm-dur-slow) var(--ccm-ease-enter),
    transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.plist-move,
.srow-move {
  transition: transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.plist-enter-from,
.plist-leave-to,
.srow-enter-from,
.srow-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

.plist-leave-active,
.srow-leave-active {
  position: absolute;
  left: 0;
  right: 0;
}

@keyframes banner-in {
  from {
    opacity: 0;
    transform: translateY(-6px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
