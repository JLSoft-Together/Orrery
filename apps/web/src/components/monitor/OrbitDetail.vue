<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Orbit, X } from 'lucide-vue-next'
import type { Provider } from '@ccm/shared'
import { clockTime, duration, now } from '../../lib/format'
import { feedRows } from '../../lib/orbitFocus'
import { groupOf } from '../../lib/sessionGroups'
import { statusMeta } from '../../lib/status'
import { displayTitle } from '../../lib/title'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { useConnectionStore } from '../../stores/connection'
import { useTerminalsStore } from '../../stores/terminals'
import { useToolSpansStore } from '../../stores/toolSpans'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const props = defineProps<{ terminalId: string; closable?: boolean }>()
const emit = defineEmits<{ focus: []; close: [] }>()

const { t, locale } = useI18n()
const terminals = useTerminalsStore()
const agents = useAgentsStore()
const toolSpans = useToolSpansStore()
const connection = useConnectionStore()

const RECENT = 5
const LOOKBACK_MS = 30 * 60_000

const terminal = computed(() => terminals.byId[props.terminalId])
const provider = computed<Provider>(() => terminal.value?.provider ?? 'claude')
const agentLabel = computed(() => t(`monitorV88.agentLabel.${provider.value}`))
const main = computed(() => agents.mainOf(props.terminalId))
const group = computed(() => (terminal.value ? groupOf(terminal.value.status) : 'ended'))
const meta = computed(() => statusMeta(terminal.value?.status))
const needs = computed(() => group.value === 'needs')
const wait = computed(() => (terminal.value && needs.value ? waitDisplay(terminal.value.waitingFor, main.value?.currentTool, t) : null))
const elapsed = computed(() => {
  const since = terminal.value?.statusSince
  return since ? duration(since, now.value) : null
})
const command = computed(() => (needs.value ? (main.value?.currentToolSummary ?? '') : ''))

const subs = computed(() => (agents.byTerminal[props.terminalId] ?? []).filter((a) => a.role === 'subagent'))

function subLine(a: (typeof subs.value)[number]): string {
  if (a.status === 'working' && a.currentTool) {
    const time = a.currentToolStartedAt ? duration(a.currentToolStartedAt, now.value) : null
    return time ? `${a.currentTool} · ${time}` : a.currentTool
  }
  if (a.status === 'waiting') return t('monitorV88.orbit.subIdle')
  if (a.status === 'working') return t(statusMeta(a.status).labelKey)
  return t('monitorV88.orbit.subDone')
}

const recent = computed(() => {
  void toolSpans.version
  const live = new Set((agents.byTerminal[props.terminalId] ?? []).filter((a) => a.status === 'working' || a.status === 'waiting').map((a) => a.id))
  return feedRows(toolSpans.spansFor(props.terminalId), live, RECENT)
})

function load(): void {
  connection.loadToolSpans(props.terminalId, new Date(Date.now() - LOOKBACK_MS).toISOString())
}
onMounted(load)
watch(() => props.terminalId, load)
</script>

<template>
  <section v-if="terminal" class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto text-xs" :aria-label="t('monitorV88.orbit.detailTitle')" data-orbit-detail>
    <header class="flex items-start gap-2">
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="flex min-w-0 items-center gap-1.5 text-sm font-semibold text-ink">
          <span class="inline-flex shrink-0" role="img" :aria-label="agentLabel" :title="agentLabel"><PresetIcon :preset="provider" :size="16" /></span>
          <span class="min-w-0 truncate" :title="displayTitle(terminal)" data-detail-title>{{ displayTitle(terminal) }}</span>
        </span>
        <span class="flex items-center gap-1.5 font-medium tabular-nums" :class="needs ? 'text-warning' : meta.text" data-detail-status>
          <StatusIcon :status="terminal.status" :size="14" />{{ t(meta.labelKey) }}<template v-if="elapsed"> · {{ elapsed }}</template>
        </span>
      </div>
      <button v-if="closable" type="button" class="ccm-btn-plain ccm-press shrink-0" :aria-label="t('monitorV88.orbit.closeDetail')" :title="t('monitorV88.orbit.closeDetail')" data-detail-close @click="emit('close')">
        <X :size="16" aria-hidden="true" />
      </button>
    </header>

    <div v-if="needs" class="flex flex-col gap-2 rounded-card border border-warning bg-warning-soft p-2.5" data-detail-needs>
      <strong class="text-ink">{{ wait?.text ?? t(meta.labelKey) }}</strong>
      <code v-if="command" class="break-all font-mono text-2xs text-ink" :title="command">{{ command }}</code>
      <OpenPaneButton :terminal-id="terminal.id" reply class="self-start" />
    </div>
    <div v-else class="flex flex-wrap items-center gap-2">
      <OpenPaneButton :terminal-id="terminal.id" />
    </div>

    <button type="button" class="ccm-btn self-start text-xs" aria-keyshortcuts="Enter" data-detail-focus @click="emit('focus')">
      <Orbit :size="14" aria-hidden="true" />{{ t('monitorV88.orbit.viewSystem') }}
    </button>

    <div class="flex flex-col gap-1">
      <h3 class="font-semibold text-ink">{{ t('monitorV88.orbit.subagents') }}</h3>
      <p v-if="!subs.length" class="text-ink-muted">{{ t('monitorV88.orbit.noSubs') }}</p>
      <ul v-else class="flex flex-col gap-1">
        <li v-for="a in subs" :key="a.id" class="flex min-w-0 items-center justify-between gap-2" data-detail-sub>
          <span class="flex min-w-0 items-center gap-1.5"><StatusIcon :status="a.status" :size="12" /><span class="truncate">{{ a.type ?? a.name ?? t('agent.subagent') }}</span></span>
          <span class="shrink-0 truncate text-ink-muted tabular-nums">{{ subLine(a) }}</span>
        </li>
      </ul>
    </div>

    <div class="flex flex-col gap-1">
      <h3 class="font-semibold text-ink">{{ t('monitorV88.orbit.recentTools') }}</h3>
      <p v-if="!recent.length" class="text-ink-muted">{{ t('monitorV88.orbit.noTools') }}</p>
      <ul v-else class="flex flex-col gap-0.5 font-mono text-2xs text-ink-muted">
        <li v-for="r in recent" :key="r.id" class="flex min-w-0 gap-2 tabular-nums" data-detail-tool>
          <span class="shrink-0">{{ clockTime(r.start, locale) }}</span><span class="truncate text-ink">{{ r.tool }}</span>
        </li>
      </ul>
    </div>
  </section>
</template>
