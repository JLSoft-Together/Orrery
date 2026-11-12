<script setup lang="ts">
import { Check, Clock, GitBranch, Laptop, LoaderCircle, SquareTerminal, TriangleAlert } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import AgentTile from '../components/AgentTile.vue'
import PageBar from '../components/PageBar.vue'
import StatusChip from '../components/StatusChip.vue'
import { activityKey, isBadActivity, parseActivity, parseSessions, type Rail } from '../lib/monitorModel'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'

const props = defineProps<{ machine: string; terminal: string; embedded?: boolean }>()
const { t, locale } = useI18n()
const hub = useHubStore()

const view = computed(() => hub.viewOf(props.machine))
const machineName = computed(() => {
  const m = hub.machines.find((x) => x.id === props.machine)
  return m?.alias ?? m?.name ?? props.machine
})
const session = computed(() => parseSessions(view.value.snapshot).find((s) => s.id === props.terminal) ?? null)
const lost = computed(() => hub.linkOf(props.machine).status !== 'ready')
const activity = computed(() => parseActivity(view.value.snapshot?.activity, props.terminal))
const project = computed(() => {
  const cwd = session.value?.cwd
  return cwd ? (cwd.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? cwd) : null
})
const provider = computed(() => session.value?.provider ?? 'claude')
const started = computed(() => (session.value?.since ? clock(session.value.since) : null))
const paneKey = computed(() => {
  const tabs = view.value.workspace?.tabs
  if (!Array.isArray(tabs)) return undefined
  for (const tab of tabs) {
    const panes = tab && typeof tab === 'object' ? (tab as Record<string, unknown>).panes : null
    if (!Array.isArray(panes)) continue
    for (const pane of panes) {
      const p = pane as Record<string, unknown>
      if ((p.terminalId === props.terminal || p.sessionId === props.terminal) && typeof p.key === 'string') return p.key
    }
  }
  return undefined
})

function clock(ms: number): string {
  return new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }).format(ms)
}

function label(kind: string): string {
  const key = activityKey(kind)
  return key ? t(`act.${key}`) : kind
}

const subIcon: Record<Rail, typeof Clock> = { working: LoaderCircle, asked: Clock, failed: TriangleAlert, done: Check, idle: Check }

function openWorkspace(): void {
  navigate({ name: 'workspace', machine: props.machine, pane: paneKey.value })
}
</script>

<template>
  <section class="page" data-testid="session-view" :data-terminal="terminal">
    <PageBar :title="session?.title ?? t('session.title')" :backless="embedded" />

    <p v-if="!session" class="empty" data-testid="session-missing">{{ t('session.missing') }}</p>
    <template v-else>
      <div class="card hero">
        <div class="hero-head">
          <AgentTile :provider="provider" :size="40" />
          <div class="who">
            <h2 class="name">{{ t(`provider.${provider}`) }}</h2>
            <p v-if="project" class="sub">{{ project }}</p>
          </div>
          <StatusChip :rail="session.rail" :lost="lost" />
        </div>
        <div class="facts">
          <span v-if="session.branch" class="fact mono"><GitBranch :size="16" :stroke-width="2" aria-hidden="true" />{{ session.branch }}</span>
          <span v-if="started" class="fact"><Clock :size="16" :stroke-width="2" aria-hidden="true" />{{ t('session.since', { time: started }) }}</span>
          <span class="fact"><Laptop :size="16" :stroke-width="2" aria-hidden="true" />{{ machineName }}</span>
        </div>
        <p v-if="session.tool" class="tool mono" data-testid="session-tool-detail">▸ {{ session.toolSummary ? `${session.tool} · ${session.toolSummary}` : session.tool }}</p>
        <p v-else-if="session.waitingFor" class="tool mono">▸ {{ session.waitingFor }}</p>
      </div>

      <template v-if="session.subagents.length">
        <h2 class="label head">{{ t('session.subagents', { n: session.subagents.length }) }}</h2>
        <div class="card list" data-testid="session-subagents">
          <div v-for="sub in session.subagents" :key="sub.id" class="sub-row" :style="{ paddingLeft: `${14 + (sub.depth - 1) * 16}px` }">
            <component :is="subIcon[sub.rail]" :size="18" :stroke-width="2.2" class="sub-icon" :class="`tone-${sub.rail}`" aria-hidden="true" />
            <span class="sub-name">{{ sub.name ?? t('session.subagent') }}</span>
            <StatusChip :rail="sub.rail" />
          </div>
        </div>
      </template>

      <h2 class="label head">{{ t('session.activity') }}</h2>
      <p v-if="activity.length === 0" class="empty" data-testid="session-activity-empty">{{ t('session.activityEmpty') }}</p>
      <ul v-else class="card list activity" data-testid="session-activity">
        <li v-for="item in activity" :key="item.id" class="act" :class="{ bad: isBadActivity(item.kind) }">
          <span class="dot" aria-hidden="true" />
          <span class="act-text">
            <span class="act-label">{{ label(item.kind) }}</span>
            <span v-if="item.detail" class="act-detail mono">{{ item.detail }}</span>
          </span>
          <span class="mono clock">{{ clock(item.at) }}</span>
        </li>
      </ul>

      <div class="actions">
        <button type="button" class="btn btn-primary open" data-testid="open-terminal" @click="openWorkspace">
          <SquareTerminal :size="20" :stroke-width="2" aria-hidden="true" />{{ t('session.openTerminal') }}
        </button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 16px;
}

.hero {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
}

.hero-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.who {
  flex: 1;
  min-width: 0;
}

.name {
  margin: 0;
  font-size: 1.0625rem;
  font-weight: 700;
}

.sub {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.facts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  color: var(--ccm-ink-muted);
  font-size: 0.8125rem;
}

.fact {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.tool {
  margin: 0;
  padding: 6px 8px;
  border-radius: var(--radius-inner);
  background: var(--ccm-raised);
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
}

.head {
  margin: 8px 0 0;
}

.list {
  margin: 0;
  padding: 0;
  list-style: none;
  overflow: hidden;
}

.sub-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 48px;
  padding: 8px 14px;
  border-top: 1px solid var(--ccm-line);
}

.sub-row:first-child {
  border-top: 0;
}

.sub-name {
  flex: 1;
  min-width: 0;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tone-working {
  color: var(--ccm-working);
}

.tone-asked {
  color: var(--ccm-waiting);
}

.tone-failed {
  color: var(--ccm-error);
}

.tone-done,
.tone-idle {
  color: var(--ccm-done);
}

.act {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 14px;
  border-top: 1px solid var(--ccm-line);
}

.act:first-child {
  border-top: 0;
}

.dot {
  width: 8px;
  height: 8px;
  margin-top: 6px;
  border-radius: 50%;
  background: var(--ccm-agent-claude);
  flex: none;
}

.act.bad .dot {
  background: var(--ccm-error);
}

.act-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.act-label {
  font-size: 0.875rem;
  font-weight: 600;
}

.act.bad .act-label {
  color: var(--ccm-error);
}

.act-detail {
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
}

.clock {
  color: var(--ccm-ink-faint);
  flex: none;
}

.actions {
  display: flex;
  gap: 8px;
  padding-top: 8px;
}

.open {
  flex: 1;
}

.empty {
  margin: 16px 0;
  text-align: center;
  color: var(--ccm-ink-muted);
}
</style>
