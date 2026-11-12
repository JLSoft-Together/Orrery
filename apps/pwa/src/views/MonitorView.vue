<script setup lang="ts">
import { Check, Clock, LayoutList, LoaderCircle, MoonStar, Rows3, TriangleAlert, WifiOff } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import LinkChip from '../components/LinkChip.vue'
import PageBar from '../components/PageBar.vue'
import SessionCard from '../components/SessionCard.vue'
import SessionRow from '../components/SessionRow.vue'
import { useNow } from '../lib/useNow'
import { filterCounts, groupByState, matchesFilter, parseSessions, sortSessions, type MonitorFilter, type Rail } from '../lib/monitorModel'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'
import { ref } from 'vue'

const props = defineProps<{ machine: string; selected?: string }>()
const { t, locale } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const now = useNow()
const filter = ref<MonitorFilter>('all')
const filters: MonitorFilter[] = ['all', 'running', 'waiting', 'idle']

const known = computed(() => hub.machines.find((m) => m.id === props.machine))
const name = computed(() => known.value?.alias ?? known.value?.name ?? props.machine)
const link = computed(() => hub.linkOf(props.machine))
const view = computed(() => hub.viewOf(props.machine))
const lost = computed(() => link.value.status !== 'ready')
const sessions = computed(() => sortSessions(parseSessions(view.value.snapshot)))
const counts = computed(() => filterCounts(sessions.value))
const shown = computed(() => sessions.value.filter((s) => matchesFilter(filter.value, s)))
const groups = computed(() => groupByState(sessions.value))
const grouped = computed(() => prefs.local.monitorMode === 'groups')
const updated = computed(() =>
  view.value.updatedAt ? new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit' }).format(view.value.updatedAt) : null,
)
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

function toggleMode(): void {
  prefs.setMonitorMode(grouped.value ? 'cards' : 'groups')
}
</script>

<template>
  <section class="page" data-testid="monitor-view" :data-mode="grouped ? 'groups' : 'cards'" :data-machine="machine">
    <PageBar :title="name">
      <template #right>
        <LinkChip :link="link" :rtt="hub.rtt[machine]" />
        <button type="button" class="icon-btn" :aria-label="t(grouped ? 'monitor.modeCards' : 'monitor.modeGroups')" data-testid="mode-toggle" @click="toggleMode">
          <component :is="grouped ? LayoutList : Rows3" :size="22" :stroke-width="2" aria-hidden="true" />
        </button>
      </template>
    </PageBar>

    <div v-if="lost && link.status !== 'idle'" class="banner" role="status" data-testid="monitor-offline">
      <WifiOff :size="22" :stroke-width="2" aria-hidden="true" />
      <div class="banner-text">
        <strong>{{ t('monitor.lostTitle', { name }) }}</strong>
        <span>{{ reasonKey ? t(reasonKey) : t('monitor.lostBody') }}<template v-if="updated"> · {{ t('monitor.dataAt', { time: updated }) }}</template></span>
      </div>
      <button type="button" class="banner-btn" data-testid="monitor-retry" @click="hub.refresh(machine)">{{ t('monitor.retry') }}</button>
    </div>

    <p v-if="!known" class="empty" data-testid="monitor-unknown">{{ t('monitor.unknown') }}</p>
    <p v-else-if="sessions.length === 0" class="empty" data-testid="monitor-empty">{{ t(view.snapshot ? 'monitor.empty' : 'machines.noData') }}</p>

    <template v-else-if="!grouped">
      <div class="filters" role="tablist" data-testid="monitor-filters">
        <button v-for="f in filters" :key="f" type="button" role="tab" class="filter" :class="{ on: filter === f }" :aria-selected="filter === f" :data-testid="`filter-${f}`" @click="filter = f">
          {{ t(`filter.${f}`) }} {{ counts[f] }}
        </button>
      </div>
      <div class="list" data-testid="monitor-list">
        <SessionCard v-for="s in shown" :key="s.id" :session="s" :now="now" :lost="lost" :class="{ picked: s.id === selected }" @open="open(s.id)" />
        <p v-if="shown.length === 0" class="empty">{{ t('monitor.emptyFilter') }}</p>
      </div>
    </template>

    <div v-else class="groups" data-testid="monitor-groups">
      <section v-for="g in groups" :key="g.rail" class="group" :data-rail="g.rail">
        <h2 class="group-head" :class="`tone-${groupMeta[g.rail].tone}`">
          <component :is="groupMeta[g.rail].icon" :size="16" :stroke-width="2.2" aria-hidden="true" />
          <span class="label">{{ t(`status.${g.rail}`) }} · {{ g.sessions.length }}</span>
        </h2>
        <div class="card group-card">
          <SessionRow v-for="s in g.sessions" :key="s.id" :session="s" :now="now" @open="open(s.id)" />
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 16px;
}

.icon-btn {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
}

.banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: var(--radius-card);
  border: 1px solid color-mix(in srgb, var(--ccm-error) 38%, transparent);
  background: color-mix(in srgb, var(--ccm-error) 8%, var(--ccm-surface));
  color: var(--ccm-error);
}

.banner-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.banner-text strong {
  color: var(--ccm-ink);
  font-size: 0.9375rem;
}

.banner-btn {
  min-height: 44px;
  padding: 0 8px;
  border: 0;
  background: transparent;
  color: var(--ccm-accent-ink);
  font-weight: 700;
}

.filters {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  margin: 0 -16px;
  padding: 0 16px;
  scrollbar-width: none;
}

.filters::-webkit-scrollbar {
  display: none;
}

.filter {
  flex: none;
  min-height: 44px;
  padding: 0 16px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-weight: 600;
  transition:
    background-color 180ms var(--ease-out),
    border-color 180ms var(--ease-out);
}

.filter.on {
  background: var(--ccm-selection);
  border-color: var(--ccm-accent);
  color: var(--ccm-accent-ink);
}

.list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.picked {
  border-color: var(--ccm-accent);
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
  margin: 0 0 8px;
}

.group-head .label {
  color: inherit;
}

.tone-error {
  color: var(--ccm-error);
}

.tone-waiting {
  color: var(--ccm-waiting);
}

.tone-working {
  color: var(--ccm-working);
}

.tone-done {
  color: var(--ccm-done);
}

.tone-idle {
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
</style>
