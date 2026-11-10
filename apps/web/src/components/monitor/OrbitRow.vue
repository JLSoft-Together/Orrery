<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Provider, TerminalSession } from '@ccm/shared'
import { duration, now } from '../../lib/format'
import { groupOf } from '../../lib/sessionGroups'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { statusMeta } from '../../lib/status'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { useUiStore } from '../../stores/ui'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'
import OrbitMini from './OrbitMini.vue'

const props = defineProps<{ terminal: TerminalSession }>()
const emit = defineEmits<{ menu: [e: MouseEvent]; key: [e: KeyboardEvent] }>()

const { t } = useI18n()
const agents = useAgentsStore()
const ui = useUiStore()

const provider = computed<Provider>(() => props.terminal.provider ?? 'claude')
const agentLabel = computed(() => t(`monitorV88.agentLabel.${provider.value}`))
const group = computed(() => groupOf(props.terminal.status))
const meta = computed(() => statusMeta(props.terminal.status))
const main = computed(() => agents.mainOf(props.terminal.id))
const project = computed(() => {
  const dir = projectOf(props.terminal.cwd)
  return dir ? folderName(dir) : t('monitorV88.noProject')
})
const subs = computed(() => (agents.byTerminal[props.terminal.id] ?? []).filter((a) => a.role === 'subagent' && (a.status === 'working' || a.status === 'waiting')).length)
const needs = computed(() => group.value === 'needs')
const focused = computed(() => ui.focusedTerminalId === props.terminal.id)

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

function onKey(e: KeyboardEvent): void {
  if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
    e.preventDefault()
    ui.focusTerminal(props.terminal.id)
    return
  }
  emit('key', e)
}
</script>

<template>
  <div
    tabindex="0"
    role="button"
    class="flex min-w-0 cursor-pointer items-center gap-2 rounded-card border px-2 py-1.5 outline-none transition-colors hover:bg-raised focus-visible:ring-2 focus-visible:ring-focus"
    :class="[needs ? 'border-warning bg-warning-soft' : 'border-transparent', focused ? 'ring-2 ring-focus' : '', group === 'ended' ? 'text-ink-muted' : '']"
    :aria-label="`${agentLabel}: ${project} · ${line}`"
    :data-orbit-row="terminal.id"
    @click="ui.focusTerminal(terminal.id)"
    @contextmenu="emit('menu', $event)"
    @keydown="onKey"
  >
    <OrbitMini :group="group" :subs="subs" />
    <div class="flex min-w-0 flex-1 flex-col gap-0.5">
      <span class="flex min-w-0 items-center gap-1.5 text-xs font-semibold">
        <span class="inline-flex shrink-0" role="img" :aria-label="agentLabel" :title="agentLabel"><PresetIcon :preset="provider" :size="14" /></span>
        <span class="min-w-0 truncate" :title="project">{{ project }}</span>
      </span>
      <span class="flex min-w-0 items-center gap-1 text-xs tabular-nums" :class="needs ? 'font-medium text-warning' : meta.text" :title="line" data-orbit-line>
        <StatusIcon :status="terminal.status" :size="12" /><span class="truncate">{{ line }}</span>
      </span>
    </div>
  </div>
</template>
