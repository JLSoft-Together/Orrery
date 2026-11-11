<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Eye, FileDiff, GitBranch, Pencil } from 'lucide-vue-next'
import type { Provider, TerminalSession } from '@ccm/shared'
import { useBoardLastTurn } from '../../composables/useBoardLastTurn'
import { fileAction } from '../../lib/boardV95'
import { hasTerminals } from '../../lib/desktop'
import { compactNumber, duration, fullNumber, now } from '../../lib/format'
import { liveSubagents, meterState, type MeterTone } from '../../lib/paneV93'
import { monogram, projectTone } from '../../lib/rail/projectRow'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { statusMeta } from '../../lib/status'
import { displayTitle } from '../../lib/title'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { useSettingsStore } from '../../stores/settings'
import { useToolSpansStore } from '../../stores/toolSpans'
import { useUiStore } from '../../stores/ui'
import { useWorkspaceStore } from '../../stores/workspace'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'
import SubagentOrbit from '../workspace/SubagentOrbit.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const props = defineProps<{ terminal: TerminalSession }>()
const emit = defineEmits<{ menu: [e: MouseEvent]; key: [e: KeyboardEvent] }>()

const { t, locale } = useI18n()
const agents = useAgentsStore()
const ui = useUiStore()
const settings = useSettingsStore()
const spans = useToolSpansStore()
const workspace = useWorkspaceStore()

const provider = computed<Provider>(() => props.terminal.provider ?? 'claude')
const agentLabel = computed(() => t(`monitorV88.agentLabel.${provider.value}`))
const meta = computed(() => statusMeta(props.terminal.status))
const main = computed(() => agents.mainOf(props.terminal.id))
const title = computed(() => displayTitle(props.terminal))
const projectKey = computed(() => projectOf(props.terminal.cwd) ?? '')
const project = computed(() => (projectKey.value ? folderName(projectKey.value) : t('monitorV88.noProject')))
const subline = computed(() => t('boardV95.subline', { project: project.value, agent: agentLabel.value }))
const needs = computed(() => props.terminal.status === 'waiting')
const working = computed(() => props.terminal.status === 'working')
const ended = computed(() => props.terminal.status === 'stale')
const quiet = computed(() => !needs.value && !working.value)
const focused = computed(() => ui.focusedTerminalId === props.terminal.id)
const subAgents = computed(() => agents.byTerminal[props.terminal.id])
const subCount = computed(() => (ended.value ? 0 : liveSubagents(subAgents.value).length))
const tone = computed(() => projectTone(projectKey.value || props.terminal.id))
const letter = computed(() => monogram(project.value))

const file = computed(() => {
  if (!working.value) return null
  void spans.version
  return fileAction(spans.spansFor(props.terminal.id), main.value?.id)
})
const fileText = computed(() => (file.value ? t(file.value.access === 'edit' ? 'boardV95.editing' : 'boardV95.reading', { file: file.value.file }) : ''))

const elapsed = computed(() => (working.value ? duration(props.terminal.statusSince, now.value) : null))
const branch = computed(() => (ended.value ? undefined : props.terminal.gitBranch || undefined))

const meter = computed(() => (ended.value ? null : meterState(main.value?.contextTokens || undefined, settings.contextWarnTokens)))
const METER_FILL: Record<MeterTone, string> = { ok: 'bg-ink-faint', warn: 'bg-warning', over: 'bg-st-error' }
const contextText = computed(() => (meter.value ? t('boardV95.context', { now: compactNumber(meter.value.now, locale.value), max: compactNumber(meter.value.max, locale.value) }) : ''))

const canDiff = computed(() => hasTerminals && !quiet.value && !!props.terminal.cwd && (props.terminal.diff?.files ?? 0) > 0)
const canOpen = computed(() => hasTerminals && !ended.value && !quiet.value)

const lastTurn = useBoardLastTurn(() => props.terminal)
const lastTurnStat = computed(() => {
  const s = lastTurn.value
  if (!s || !quiet.value || ended.value) return null
  return { text: t('boardV95.lastTurn', { n: s.files }, s.files), added: s.added, removed: s.removed }
})

function openDiff(): void {
  const cwd = props.terminal.cwd
  if (!cwd) return
  workspace.openChanges(cwd)
  ui.setView('workspace')
}

function onKey(e: KeyboardEvent): void {
  if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
    e.preventDefault()
    ui.focusTerminal(props.terminal.id)
    return
  }
  emit('key', e)
}

const line = computed(() => {
  const term = props.terminal
  if (needs.value) {
    const w = waitDisplay(term.waitingFor, main.value?.currentTool, t)
    const waited = term.statusSince ? duration(term.statusSince, now.value) : null
    return [w?.text ?? t(meta.value.labelKey), waited].filter(Boolean).join(' · ')
  }
  const tool = main.value?.currentTool
  if (working.value && tool) {
    const since = main.value?.currentToolStartedAt
    const time = since ? duration(since, now.value) : null
    return time ? t('monitorV88.toolRunning', { tool, time }) : tool
  }
  return t(meta.value.labelKey)
})
</script>

<template>
  <article
    role="group"
    tabindex="0"
    class="ccm-board-card group relative flex min-w-0 flex-col gap-2 rounded-card border bg-surface py-2.5 ps-3.5 pe-3 shadow-card outline-none focus-visible:ring-2 focus-visible:ring-focus"
    :class="[`ccm-agent-${provider}`, needs ? 'border-warning' : 'border-transparent', focused ? 'ring-2 ring-focus' : '', ended ? 'text-ink-muted' : '', quiet ? 'ccm-board-quiet' : '']"
    :aria-label="`${agentLabel}: ${title}`"
    :data-board-card="terminal.id"
    @click="ui.focusTerminal(terminal.id)"
    @contextmenu="emit('menu', $event)"
    @keydown="onKey"
  >
    <span class="pointer-events-none absolute inset-y-2 start-0 w-0.5 rounded-full bg-(--ccm-pane-accent)" :class="ended ? 'opacity-40' : ''" aria-hidden="true" data-board-bar />
    <div class="flex min-w-0 items-center gap-2.5">
      <span class="ccm-board-mono" :class="ended ? 'ccm-board-mono-off' : ''" :data-tone="tone" role="img" :aria-label="project" data-board-mono>{{ letter }}</span>
      <span class="flex min-w-0 flex-1 flex-col">
        <span class="truncate text-sm font-semibold" :title="title" data-board-title>{{ title }}</span>
        <span class="inline-flex min-w-0 items-center gap-1 text-xs text-ink-muted" :title="subline" data-board-subline>
          <PresetIcon :preset="provider" :size="12" class="shrink-0" aria-hidden="true" />
          <span class="truncate">{{ subline }}</span>
        </span>
      </span>
      <span v-if="subCount > 0" class="shrink-0" data-board-subs @click.stop @keydown.stop>
        <SubagentOrbit :agents="subAgents" />
      </span>
    </div>
    <p
      class="flex min-w-0 items-center gap-1.5 rounded-inner px-1.5 py-0.5 text-xs tabular-nums"
      :class="needs ? 'ccm-chip ccm-chip-warning font-medium text-warning' : meta.text"
      :title="fileText || line"
      data-board-line
    >
      <component :is="file.access === 'edit' ? Pencil : Eye" v-if="file" :size="13" class="shrink-0" aria-hidden="true" />
      <StatusIcon v-else :status="terminal.status" :size="13" />
      <span class="truncate">{{ fileText || line }}</span>
    </p>
    <div v-if="elapsed || branch" class="flex min-w-0 items-center gap-3 font-mono text-xs text-ink-muted">
      <span v-if="elapsed" class="shrink-0 tabular-nums" :title="t('boardV95.elapsed', { time: elapsed })" data-board-elapsed>{{ elapsed }}</span>
      <span v-if="branch" class="inline-flex min-w-0 items-center gap-1" :title="t('boardV95.branch', { name: branch })" data-board-branch>
        <GitBranch :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ branch }}</span>
      </span>
    </div>
    <div v-if="meter" class="flex min-w-0 flex-col gap-1" :data-board-context="meter.tone">
      <p class="text-xs tabular-nums" :class="meter.tone === 'ok' ? 'text-ink-muted' : meter.tone === 'warn' ? 'font-medium text-warning' : 'font-medium text-st-error'">{{ contextText }}</p>
      <div
        class="h-1 w-full overflow-hidden rounded-full bg-fill"
        role="meter"
        :aria-label="t('boardV95.contextAria')"
        aria-valuemin="0"
        :aria-valuemax="meter.max"
        :aria-valuenow="Math.min(meter.now, meter.max)"
        :aria-valuetext="contextText"
      >
        <div class="h-full w-full origin-left rounded-full" :class="METER_FILL[meter.tone]" :style="{ transform: `scaleX(${meter.fill})` }" />
      </div>
    </div>
    <p v-if="lastTurnStat" class="flex min-w-0 flex-wrap items-center gap-x-2 text-xs text-ink-muted" data-board-last-turn>
      <span>{{ lastTurnStat.text }}</span>
      <span v-if="lastTurnStat.added" class="font-mono font-medium text-diff-add tabular-nums">+{{ fullNumber(lastTurnStat.added, locale) }}</span>
      <span v-if="lastTurnStat.removed" class="font-mono font-medium text-diff-del tabular-nums">−{{ fullNumber(lastTurnStat.removed, locale) }}</span>
    </p>
    <div v-if="canOpen || canDiff" class="flex min-w-0 flex-wrap items-center gap-2" data-board-actions>
      <OpenPaneButton v-if="canOpen" :terminal-id="terminal.id" primary compact label-key="boardV95.openWorkspace" />
      <button v-if="canDiff" type="button" class="ccm-btn h-6 px-2 text-xs" :title="t('boardV95.viewDiffTitle')" data-board-diff @click.stop="openDiff" @keydown.stop>
        <FileDiff :size="14" aria-hidden="true" />{{ t('boardV95.viewDiff') }}
      </button>
    </div>
  </article>
</template>

<style scoped>
.ccm-board-mono {
  --mono: var(--ccm-accent);
  display: inline-flex;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-inner);
  font-size: 13px;
  font-weight: 700;
  line-height: 1;
  color: var(--ccm-mono-ink);
  background: color-mix(in srgb, var(--mono) 62%, var(--ccm-mono-mix));
}
.ccm-board-mono[data-tone='0'] { --mono: var(--ccm-agent-claude); }
.ccm-board-mono[data-tone='1'] { --mono: var(--ccm-agent-codex); }
.ccm-board-mono[data-tone='2'] { --mono: var(--ccm-agent-browser); }
.ccm-board-mono[data-tone='3'] { --mono: var(--ccm-agent-file); }
.ccm-board-mono[data-tone='4'] { --mono: var(--ccm-success); }
.ccm-board-mono[data-tone='5'] { --mono: var(--ccm-peer); }
.ccm-board-mono[data-tone='6'] { --mono: var(--ccm-warning); }
.ccm-board-mono[data-tone='7'] { --mono: var(--ccm-accent); }
.ccm-board-mono-off {
  background: transparent;
  color: var(--ccm-ink-muted);
  box-shadow: inset 0 0 0 1px var(--ccm-line-strong);
}
.ccm-board-quiet {
  box-shadow: none;
}
</style>
