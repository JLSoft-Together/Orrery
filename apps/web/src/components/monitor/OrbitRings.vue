<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentStatus, Provider } from '@ccm/shared'
import { useOrbitNudges } from '../../composables/useOrbitNudges'
import { duration, now } from '../../lib/format'
import { reducedMotion } from '../../lib/motion'
import { computeRings, NODE_RADIUS, navigate, type NavKey, type RingSession } from '../../lib/orbitRings'
import { isTyping } from '../../lib/shortcuts'
import { groupOf, type SessionGroup } from '../../lib/sessionGroups'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { statusMeta } from '../../lib/status'
import { displayTitle } from '../../lib/title'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { useTerminalsStore } from '../../stores/terminals'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'

const props = defineProps<{ groups: readonly SessionGroup[]; selectedId: string | null }>()
const emit = defineEmits<{ select: [id: string]; open: [id: string]; menu: [e: MouseEvent, id: string]; keymenu: [e: KeyboardEvent, id: string] }>()

const { t } = useI18n()
const terminals = useTerminalsStore()
const agents = useAgentsStore()
const nudges = useOrbitNudges()

const stage = ref<HTMLElement | null>(null)
const size = ref({ w: 0, h: 0 })
const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(([entry]) => (size.value = { w: entry?.contentRect.width ?? 0, h: entry?.contentRect.height ?? 0 }))
watch(stage, (el, old) => {
  if (old) observer?.unobserve(old)
  if (el) observer?.observe(el)
})
onBeforeUnmount(() => observer?.disconnect())

const projectOfTerminal = (cwd: string | undefined): string => {
  const dir = projectOf(cwd)
  return dir ? folderName(dir) : t('monitorV88.noProject')
}

const sessions = computed<RingSession[]>(() =>
  props.groups.flatMap((g) => g.ids).flatMap((id) => {
    const term = terminals.byId[id]
    if (!term) return []
    const subs = (agents.byTerminal[id] ?? []).filter((a) => a.role === 'subagent').map((a) => ({ id: a.id, status: a.status as AgentStatus }))
    return [{ id, project: projectOfTerminal(term.cwd), status: term.status, subs }]
  }),
)

const layout = computed(() => computeRings({ sessions: sessions.value, width: size.value.w, height: size.value.h, nudges: nudges.value, reduced: reducedMotion() }))
const RING_KEYS = ['ringNeeds', 'ringRunning', 'ringQuiet'] as const
const ringStroke = ['var(--color-warning)', 'var(--color-accent)', 'var(--color-ink-faint)'] as const

const tabbable = computed(() => props.selectedId ?? layout.value.nodes[0]?.id ?? null)

function line(id: string): string {
  const term = terminals.byId[id]
  if (!term) return ''
  const meta = statusMeta(term.status)
  const main = agents.mainOf(id)
  if (groupOf(term.status) === 'needs') {
    const w = waitDisplay(term.waitingFor, main?.currentTool, t)
    const waited = term.statusSince ? duration(term.statusSince, now.value) : null
    return [w?.text ?? t(meta.labelKey), waited].filter(Boolean).join(' · ')
  }
  if (term.status === 'working' && main?.currentTool) {
    const time = main.currentToolStartedAt ? duration(main.currentToolStartedAt, now.value) : null
    return time ? t('monitorV88.toolRunning', { tool: main.currentTool, time }) : main.currentTool
  }
  return t(meta.labelKey)
}

function labelOf(id: string): string {
  const term = terminals.byId[id]
  if (!term) return ''
  const provider: Provider = term.provider ?? 'claude'
  return `${t(`monitorV88.agentLabel.${provider}`)}: ${projectOfTerminal(term.cwd)} · ${line(id)}`
}

function nodeClass(id: string, ring: number): string[] {
  const term = terminals.byId[id]
  const g = term ? groupOf(term.status) : 'ended'
  return [
    g === 'needs' ? 'border-warning ccm-orbit-needs' : g === 'running' ? 'border-accent' : g === 'idle' ? 'border-line-strong' : 'border-line',
    ring === 2 && props.selectedId !== id ? 'opacity-60 hover:opacity-100 focus-visible:opacity-100' : '',
    props.selectedId === id ? 'ccm-orbit-selected' : '',
  ]
}

function subClass(status: AgentStatus): string {
  if (status === 'working') return 'bg-accent'
  if (status === 'waiting') return 'border-2 border-accent bg-surface'
  if (status === 'completed') return 'bg-success'
  return 'bg-fill-strong'
}

const NAV_KEYS = new Set<string>(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'])
function onKey(e: KeyboardEvent, id: string): void {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey || e.isComposing || isTyping(e.target)) {
    if (e.shiftKey && e.key === 'F10') emit('keymenu', e, id)
    return
  }
  if (NAV_KEYS.has(e.key)) {
    e.preventDefault()
    const next = navigate(layout.value.nodes, id, e.key as NavKey)
    if (next) {
      emit('select', next)
      void nextTick(() => stage.value?.querySelector<HTMLElement>(`[data-orbit-node="${CSS.escape(next)}"]`)?.focus())
    }
  } else if (e.key === 'Enter') {
    e.preventDefault()
    emit('open', id)
  } else if (e.key === 'ContextMenu') emit('keymenu', e, id)
}
</script>

<template>
  <div ref="stage" class="relative min-h-0 min-w-0 flex-1 overflow-hidden bg-canvas" role="group" :aria-label="t('monitorV88.orbit.region')" data-orbit-rings>
    <svg class="absolute inset-0 h-full w-full" aria-hidden="true">
      <line v-for="s in layout.sectors" :key="`l-${s.project}`" :x1="s.line.x1" :y1="s.line.y1" :x2="s.line.x2" :y2="s.line.y2" stroke="var(--color-line)" />
      <text v-for="s in layout.sectors" :key="`t-${s.project}`" :x="s.label.x" :y="s.label.y" :text-anchor="s.label.anchor" font-size="12" font-weight="600" fill="var(--color-ink)" data-orbit-sector>
        {{ s.project.length > 16 ? `${s.project.slice(0, 15)}…` : s.project }}
      </text>
      <template v-for="(r, i) in layout.radii" :key="`r-${i}`">
        <circle :cx="layout.cx" :cy="layout.cy" :r="r" fill="none" :stroke="ringStroke[i]" stroke-dasharray="4 5" :data-orbit-ring="i" />
        <text :x="layout.cx + 6" :y="layout.cy - r - 5" font-size="11" :fill="ringStroke[i]">{{ t(`monitorV88.orbit.${RING_KEYS[i]}`) }}</text>
      </template>
      <circle :cx="layout.cx" :cy="layout.cy" :r="layout.core" fill="var(--color-ink)" />
      <text :x="layout.cx" :y="layout.cy + 4" text-anchor="middle" font-size="12" font-weight="700" fill="var(--color-canvas)">{{ t('monitorV88.orbit.you') }}</text>
    </svg>

    <template v-for="n in layout.nodes" :key="n.id">
      <button
        v-if="n.dot"
        type="button"
        class="ccm-press absolute block size-3.5 cursor-pointer rounded-full border border-line-strong bg-fill-strong opacity-60 outline-none hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-focus"
        :class="selectedId === n.id ? 'ccm-orbit-selected opacity-100' : ''"
        :style="{ left: `${n.x - 7}px`, top: `${n.y - 7}px` }"
        :tabindex="tabbable === n.id ? 0 : -1"
        :aria-label="labelOf(n.id)"
        :aria-pressed="selectedId === n.id"
        :title="labelOf(n.id)"
        :data-orbit-node="n.id"
        data-orbit-dot
        @click="emit('select', n.id)"
        @dblclick="emit('open', n.id)"
        @keydown="onKey($event, n.id)"
        @contextmenu="emit('menu', $event, n.id)"
      />
      <template v-else>
        <span
          v-for="s in n.subs"
          :key="s.id"
          class="pointer-events-none absolute block size-2.5 rounded-full transition-[left,top] duration-(--ccm-dur-base)"
          :class="subClass(s.status)"
          :style="{ left: `${s.x - 5}px`, top: `${s.y - 5}px` }"
          :data-orbit-sub="s.status"
          aria-hidden="true"
        />
        <span
          v-if="n.moreSubs"
          class="pointer-events-none absolute text-2xs tabular-nums text-ink-muted"
          :style="{ left: `${n.x + NODE_RADIUS + 4}px`, top: `${n.y - NODE_RADIUS - 10}px` }"
          aria-hidden="true"
        >
          {{ t('monitorV88.orbit.more', { n: n.moreSubs }) }}
        </span>
        <button
          type="button"
          class="ccm-press absolute flex size-[34px] cursor-pointer items-center justify-center rounded-full border-[3px] bg-surface outline-none focus-visible:ring-2 focus-visible:ring-focus"
          :class="nodeClass(n.id, n.ring)"
          :style="{ left: `${n.x - NODE_RADIUS}px`, top: `${n.y - NODE_RADIUS}px` }"
          :tabindex="tabbable === n.id ? 0 : -1"
          :aria-label="labelOf(n.id)"
          :aria-pressed="selectedId === n.id"
          :title="labelOf(n.id)"
          :data-orbit-node="n.id"
          :data-ring="n.ring"
          @click="emit('select', n.id)"
          @dblclick="emit('open', n.id)"
          @keydown="onKey($event, n.id)"
          @contextmenu="emit('menu', $event, n.id)"
        >
          <PresetIcon :preset="terminals.byId[n.id]?.provider ?? 'claude'" :size="16" />
          <span class="absolute -right-1 -bottom-1 flex size-3.5 items-center justify-center rounded-full bg-surface"><StatusIcon :status="terminals.byId[n.id]?.status ?? 'unknown'" :size="11" /></span>
        </button>
      </template>
    </template>

    <p v-if="!layout.nodes.length" class="pointer-events-none absolute inset-x-0 top-[58%] text-center text-sm text-ink-muted" data-orbit-empty>{{ t('monitorV88.orbit.empty') }}</p>
    <p class="pointer-events-none absolute inset-x-3 bottom-2 text-xs text-ink-muted" data-orbit-legend>
      {{ t('monitorV88.orbit.legend') }}
    </p>
  </div>
</template>
