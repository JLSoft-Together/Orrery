<script setup lang="ts">
import { Check, Clock, GitBranch, LoaderCircle, SquareTerminal, TriangleAlert } from 'lucide-vue-next'
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AgentTile from '../components/AgentTile.vue'
import ChatAsk, { type AskPick } from '../components/ChatAsk.vue'
import { answerChoices, createAnswerFlow } from '../lib/paneAnswer'
import { findPane, parseWorkspace, type WsPane } from '../lib/workspaceModel'
import { RemoteCommandError } from '../remote/types'
import PageBar from '../components/PageBar.vue'
import StatusChip from '../components/StatusChip.vue'
import { activityKey, isBadActivity, parseActivity, parseSessions, type Rail } from '../lib/monitorModel'
import { projectName } from '../lib/projectModel'
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
const project = computed(() => projectName(session.value?.cwd ?? null))
const pageSub = computed(() => [project.value, machineName.value].filter(Boolean).join(', '))
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

function indent(depth: number): string {
  return `${16 + (depth - 1) * 16}px`
}

const subIcon: Record<Rail, typeof Clock> = { working: LoaderCircle, asked: Clock, failed: TriangleAlert, done: Check, idle: Check }

const tabs = computed(() => parseWorkspace(view.value.workspace, parseSessions(view.value.snapshot)))
const askPane = computed(() => (paneKey.value ? (findPane(tabs.value, paneKey.value)?.pane ?? null) : null))
const answerable = computed(() => !!askPane.value && askPane.value.rail === 'asked' && (!!askPane.value.menu || answerChoices(askPane.value).length > 0))
const pendingLabel = ref<string | null>(null)
const answers = createAnswerFlow({
  now: () => Date.now(),
  dataAt: () => (view.value.stale ? 0 : (view.value.updatedAt ?? 0)),
  refresh: async () => {
    await Promise.all([hub.command(props.machine, 'ws.get'), hub.refresh(props.machine)])
  },
  read: (key: string): WsPane | null => findPane(tabs.value, key)?.pane ?? null,
  send: async (target, keys) => {
    if (keys.startsWith('menu:')) await hub.command(props.machine, 'ws.menu', { paneKey: target.key, sig: target.menu?.sig ?? '', index: Number(keys.slice(5)) })
    else await hub.command(props.machine, 'ws.keys', { paneKey: target.key, keys, since: target.since ?? 0 })
  },
})

async function pick(choice: AskPick): Promise<void> {
  if (!askPane.value) return
  pendingLabel.value = choice.label
  const result = await answers.press(askPane.value, choice.keys)
  pendingLabel.value = null
  if (result.outcome === 'failed') console.warn(`[session] answer ${result.error instanceof RemoteCommandError ? result.error.code : 'internal'}`)
}

onBeforeUnmount(() => answers.dispose())

function openWorkspace(): void {
  navigate({ name: 'workspace', machine: props.machine, pane: paneKey.value })
}
</script>

<template>
  <section class="page" data-testid="session-view" :data-terminal="terminal">
    <PageBar :title="session?.title ?? t('session.title')" :sub="pageSub" :backless="embedded" />

    <p v-if="!session" class="empty" data-testid="session-missing">{{ t('session.missing') }}</p>
    <template v-else>
      <div class="card hero">
        <div class="hero-head">
          <AgentTile :provider="provider" :size="48" />
          <div class="who">
            <p v-if="session.branch" class="sub">
              <GitBranch :size="14" :stroke-width="2" aria-hidden="true" />
              <span class="sub-text mono">{{ session.branch }}</span>
            </p>
          </div>
          <StatusChip :rail="session.rail" :lost="lost" />
        </div>
        <dl class="facts">
          <div class="fact"><dt>{{ t('session.agent') }}</dt><dd class="agent" :class="`p-${provider}`">{{ t(`provider.${provider}`) }}</dd></div>
          <div v-if="started" class="fact"><dt>{{ t('session.startedAt') }}</dt><dd>{{ started }}</dd></div>
        </dl>
        <p v-if="session.tool" class="tool mono" data-testid="session-tool-detail">▸ {{ session.toolSummary ? `${session.tool}, ${session.toolSummary}` : session.tool }}</p>
      </div>

      <Transition name="ask">
        <div v-if="session.rail === 'asked'" class="ask" role="status" data-testid="session-ask">
          <p class="ask-text">
            <Clock :size="22" :stroke-width="2.2" aria-hidden="true" />
            <span><strong>{{ t('session.waitingTitle') }}</strong><template v-if="session.waitingFor">: {{ session.waitingFor }}</template></span>
          </p>
          <ChatAsk v-if="answerable && askPane" class="ask-card" :pane="askPane" :cmd="session.toolSummary" :enabled="!lost && !hub.isViewOnly(machine)" :pending="answers.pending.value ? pendingLabel : null" @pick="pick" @undo="answers.undo()" @terminal="openWorkspace" />
          <button v-else type="button" class="btn btn-primary" data-testid="session-answer" @click="openWorkspace"><SquareTerminal :size="20" :stroke-width="2" aria-hidden="true" />{{ t('session.answer') }}</button>
        </div>
      </Transition>

      <template v-if="session.subagents.length">
        <h2 class="sec">{{ t('session.subagents', { n: session.subagents.length }) }}</h2>
        <TransitionGroup name="srow" tag="div" class="card list" data-testid="session-subagents">
          <div v-for="sub in session.subagents" :key="sub.id" class="sub-row" :data-depth="sub.depth" :style="{ paddingLeft: indent(sub.depth) }">
            <component :is="subIcon[sub.rail]" :size="20" :stroke-width="2.2" class="sub-icon" :class="`subtone-${sub.rail}`" aria-hidden="true" />
            <span class="sub-name">{{ sub.name ?? t('session.subagent') }}</span>
            <StatusChip :rail="sub.rail" />
          </div>
        </TransitionGroup>
      </template>

      <h2 class="sec">{{ t('session.activity') }}</h2>
      <p v-if="activity.length === 0" class="empty" data-testid="session-activity-empty">{{ t('session.activityEmpty') }}</p>
      <TransitionGroup v-else name="srow" tag="ul" class="card list activity" data-testid="session-activity">
        <li v-for="item in activity" :key="item.id" class="act" :class="{ bad: isBadActivity(item.kind) }">
          <span class="mono clock">{{ clock(item.at) }}</span>
          <span class="dot" aria-hidden="true" />
          <span class="act-text">
            <span class="act-label">{{ label(item.kind) }}</span>
            <span v-if="item.detail" class="act-detail mono">{{ item.detail }}</span>
          </span>
        </li>
      </TransitionGroup>

      <div v-if="!(session.rail === 'asked' && !answerable)" class="actions">
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
  gap: 14px;
  padding: 16px;
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
  font-size: 1.1875rem;
  font-weight: 800;
  line-height: 1.25;
  overflow-wrap: anywhere;
}

.sub {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  margin: 4px 0 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.sub svg {
  flex: none;
}

.sub-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub .tone-mono {
  width: 22px;
  height: 22px;
  font-size: 0.75rem;
}

.facts {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0;
}

.fact {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.fact dt {
  color: var(--ccm-ink-muted);
}

.fact dd {
  margin: 0;
  font-weight: 700;
  text-align: right;
  overflow-wrap: anywhere;
}

.agent {
  color: var(--ccm-agent-claude);
}

.agent.p-codex {
  color: var(--ccm-agent-codex);
}

.agent.p-shell {
  color: var(--ccm-agent-shell);
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.tool {
  margin: 0;
  padding: 8px 10px;
  border-radius: var(--radius-inner);
  background: var(--ccm-raised);
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
}

.ask-card {
  align-self: stretch;
}

.ask {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  border-radius: var(--radius-card);
  border: 1px solid color-mix(in srgb, var(--ccm-waiting) 42%, transparent);
  background: color-mix(in srgb, var(--ccm-waiting) 12%, var(--ccm-glass));
}

.ask-text {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin: 0;
  color: var(--ccm-ink);
  font-size: 1rem;
  overflow-wrap: anywhere;
}

.ask-text svg {
  flex: none;
  margin-top: 1px;
  color: var(--ccm-waiting);
}

.sec {
  margin: 8px 0 0 4px;
  font-size: 1.0625rem;
  font-weight: 800;
}

.list {
  position: relative;
  margin: 0;
  padding: 0;
  list-style: none;
  overflow: hidden;
}

.sub-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 8px 16px;
  border-top: 1px solid var(--ccm-line);
}

.sub-row:first-child {
  border-top: 0;
}

.sub-name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.subtone-working {
  color: var(--ccm-working);
}

.subtone-asked {
  color: var(--ccm-waiting);
}

.subtone-failed {
  color: var(--ccm-error);
}

.subtone-done,
.subtone-idle {
  color: var(--ccm-done);
}

.act {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  border-top: 1px solid var(--ccm-line);
}

.act:first-child {
  border-top: 0;
}

.clock {
  flex: none;
  width: 4.5rem;
  margin-top: 2px;
  color: var(--ccm-ink-faint);
}

.dot {
  width: 8px;
  height: 8px;
  margin-top: 7px;
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
  font-size: 0.9375rem;
  font-weight: 700;
}

.act.bad .act-label {
  color: var(--ccm-error);
}

.act-detail {
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
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

.srow-enter-active,
.srow-leave-active,
.ask-enter-active,
.ask-leave-active {
  transition:
    opacity var(--ccm-dur-slow) var(--ccm-ease-enter),
    transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.srow-move {
  transition: transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.srow-enter-from,
.srow-leave-to,
.ask-enter-from,
.ask-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

.srow-leave-active {
  position: absolute;
  left: 0;
  right: 0;
}
</style>
