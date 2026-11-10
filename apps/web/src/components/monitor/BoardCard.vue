<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Network } from 'lucide-vue-next'
import type { Provider, TerminalSession } from '@ccm/shared'
import { duration, now } from '../../lib/format'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { statusMeta } from '../../lib/status'
import { displayTitle } from '../../lib/title'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { useUiStore } from '../../stores/ui'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const props = defineProps<{ terminal: TerminalSession }>()
const emit = defineEmits<{ menu: [e: MouseEvent]; key: [e: KeyboardEvent] }>()

const { t } = useI18n()
const agents = useAgentsStore()
const ui = useUiStore()

const provider = computed<Provider>(() => props.terminal.provider ?? 'claude')
const agentLabel = computed(() => t(`monitorV88.agentLabel.${provider.value}`))
const meta = computed(() => statusMeta(props.terminal.status))
const main = computed(() => agents.mainOf(props.terminal.id))
const title = computed(() => displayTitle(props.terminal))
const project = computed(() => {
  const dir = projectOf(props.terminal.cwd)
  return dir ? folderName(dir) : t('monitorV88.noProject')
})
const needs = computed(() => props.terminal.status === 'waiting')
const ended = computed(() => props.terminal.status === 'stale')
const subs = computed(() => (agents.byTerminal[props.terminal.id] ?? []).filter((a) => a.role === 'subagent' && (a.status === 'working' || a.status === 'waiting')).length)
const focused = computed(() => ui.focusedTerminalId === props.terminal.id)

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
  if (term.status === 'working' && tool) {
    const since = main.value?.currentToolStartedAt
    const time = since ? duration(since, now.value) : null
    return time ? t('monitorV88.toolRunning', { tool, time }) : tool
  }
  return t(meta.value.labelKey)
})
</script>

<template>
  <article
    tabindex="0"
    class="group relative flex min-w-0 flex-col gap-1.5 rounded-card border bg-surface px-3 py-2.5 shadow-card outline-none focus-visible:ring-2 focus-visible:ring-focus"
    :class="[needs ? 'border-warning' : 'border-transparent', focused ? 'ring-2 ring-focus' : '', ended ? 'text-ink-muted' : '']"
    :aria-label="`${agentLabel}: ${title}`"
    :data-board-card="terminal.id"
    @click="ui.focusTerminal(terminal.id)"
    @contextmenu="emit('menu', $event)"
    @keydown="onKey"
  >
    <div class="flex min-w-0 items-center gap-2">
      <span class="inline-flex shrink-0" role="img" :aria-label="agentLabel" :title="agentLabel"><PresetIcon :preset="provider" :size="16" /></span>
      <span class="min-w-0 flex-1 truncate text-sm font-semibold" :title="project">{{ project }}</span>
      <StatusIcon :status="terminal.status" :size="14" />
    </div>
    <p class="truncate text-xs text-ink-muted" :title="title" data-board-title>{{ title }}</p>
    <p
      class="flex min-w-0 items-center gap-1.5 truncate rounded-inner px-1.5 py-0.5 text-xs tabular-nums"
      :class="needs ? 'bg-warning-soft font-medium text-warning' : meta.text"
      :title="line"
      data-board-line
    >
      <span class="truncate">{{ line }}</span>
    </p>
    <div v-if="subs > 0 || needs" class="flex min-w-0 items-center gap-2">
      <span v-if="subs > 0" class="inline-flex min-w-0 items-center gap-1 text-xs text-ink-muted" :title="t('monitorV88.subagents', { n: subs })" data-board-subs>
        <Network :size="13" aria-hidden="true" /><span class="tabular-nums">{{ subs }}</span
        ><span class="sr-only">{{ t('monitorV88.subagents', { n: subs }) }}</span>
      </span>
      <OpenPaneButton v-if="needs" :terminal-id="terminal.id" reply compact class="ms-auto" />
    </div>
  </article>
</template>
