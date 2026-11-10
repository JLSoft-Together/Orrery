<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronLeft, ChevronRight, CornerUpLeft } from 'lucide-vue-next'
import type { Agent, AgentStatus, Provider, ToolOutcome } from '@ccm/shared'
import { useOrbitNudges } from '../../composables/useOrbitNudges'
import { clockTime, duration, now } from '../../lib/format'
import { reducedMotion } from '../../lib/motion'
import { arcPath, computeFocus, feedRows, PLANET_RADIUS, stepSystem } from '../../lib/orbitFocus'
import { groupOf } from '../../lib/sessionGroups'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { statusMeta } from '../../lib/status'
import { displayTitle } from '../../lib/title'
import { formatMs } from '../../lib/toolTimeline'
import { useAgentsStore } from '../../stores/agents'
import { useConnectionStore } from '../../stores/connection'
import { useTerminalsStore } from '../../stores/terminals'
import { useToolSpansStore } from '../../stores/toolSpans'
import { useUiStore } from '../../stores/ui'
import StatusIcon from '../StatusIcon.vue'
import PresetIcon from '../workspace/PresetIcon.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const props = defineProps<{ terminalId: string; ids: readonly string[] }>()
const emit = defineEmits<{ select: [id: string]; back: [] }>()

const { t, locale } = useI18n()
const terminals = useTerminalsStore()
const agents = useAgentsStore()
const toolSpans = useToolSpansStore()
const connection = useConnectionStore()
const ui = useUiStore()
const nudges = useOrbitNudges()

const LOOKBACK_MS = 60 * 60_000

const root = ref<HTMLElement | null>(null)
const stage = ref<HTMLElement | null>(null)
const size = ref({ w: 0, h: 0 })
const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(([entry]) => (size.value = { w: entry?.contentRect.width ?? 0, h: entry?.contentRect.height ?? 0 }))
watch(stage, (el, old) => {
  if (old) observer?.unobserve(old)
  if (el) observer?.observe(el)
})
onBeforeUnmount(() => observer?.disconnect())

const terminal = computed(() => terminals.byId[props.terminalId])
const provider = computed<Provider>(() => terminal.value?.provider ?? 'claude')
const main = computed(() => agents.mainOf(props.terminalId))
const subs = computed(() => (agents.byTerminal[props.terminalId] ?? []).filter((a) => a.role === 'subagent'))
const group = computed(() => (terminal.value ? groupOf(terminal.value.status) : 'ended'))
const meta = computed(() => statusMeta(terminal.value?.status))
const elapsed = computed(() => (terminal.value?.statusSince ? duration(terminal.value.statusSince, now.value) : null))

const spans = computed(() => {
  void toolSpans.version
  return toolSpans.spansFor(props.terminalId)
})

const layout = computed(() =>
  computeFocus({
    terminalId: props.terminalId,
    mainId: main.value?.id ?? null,
    subs: subs.value.map((a) => ({ id: a.id, status: a.status as AgentStatus })),
    spans: spans.value,
    nowMs: now.value,
    width: size.value.w,
    height: size.value.h,
    nudges: nudges.value,
    reduced: reducedMotion(),
  }),
)

const liveAgents = computed(() => new Set((agents.byTerminal[props.terminalId] ?? []).filter((a) => a.status === 'working' || a.status === 'waiting').map((a) => a.id)))
const feed = computed(() => feedRows(spans.value, liveAgents.value))
const agentOf = (id: string): Agent | undefined => agents.byId[id]

function agentName(a: Agent | undefined): string {
  if (!a) return t('tools.endedSubagent')
  if (a.role === 'main') return t('monitorV88.orbit.sun')
  return a.type ?? a.name ?? t('agent.subagent')
}
function toolText(a: Agent | undefined): string {
  if (!a) return ''
  if (a.status === 'working' || a.status === 'waiting') return a.currentToolSummary || a.currentTool || t(statusMeta(a.status).labelKey)
  return a.completedAt ? t('monitorV88.orbit.subDone') : t(statusMeta(a.status).labelKey)
}
const clip = (s: string, n: number): string => (s.length > n ? `${s.slice(0, n - 1)}…` : s)

const SUN_FILL: Record<string, string> = { needs: 'var(--color-warning)', running: 'var(--color-accent)', idle: 'var(--color-ink-faint)', ended: 'var(--color-line-strong)' }
const SUN_HALO: Record<string, string> = { needs: 'var(--color-warning-soft)', running: 'var(--color-accent-soft)', idle: 'var(--color-fill)', ended: 'var(--color-fill)' }
const OUTCOME_STROKE: Record<ToolOutcome | 'running', string> = {
  running: 'var(--color-accent)',
  ok: 'var(--color-accent)',
  error: 'var(--color-st-error)',
  interrupted: 'var(--color-ink-faint)',
}
const trailOpacity = (running: boolean): number => (running ? 0.9 : 0.5)

function planetFill(status: AgentStatus): { fill: string; stroke: string } {
  if (status === 'working') return { fill: 'var(--color-accent)', stroke: 'none' }
  if (status === 'waiting') return { fill: 'var(--color-surface)', stroke: 'var(--color-accent)' }
  if (status === 'completed') return { fill: 'var(--color-success)', stroke: 'none' }
  return { fill: 'var(--color-fill-strong)', stroke: 'none' }
}
function anchorOf(angle: number): 'start' | 'end' {
  return Math.cos(angle) >= 0 ? 'start' : 'end'
}

const projectName = computed(() => {
  const dir = projectOf(terminal.value?.cwd)
  return dir ? folderName(dir) : t('monitorV88.noProject')
})

function railLabel(id: string): string {
  const term = terminals.byId[id]
  return term ? displayTitle(term) : id
}
function railSubs(id: string): number {
  return (agents.byTerminal[id] ?? []).filter((a) => a.role === 'subagent' && (a.status === 'working' || a.status === 'waiting')).length
}
function railSun(id: string): string {
  const term = terminals.byId[id]
  return SUN_FILL[term ? groupOf(term.status) : 'ended']!
}

function step(delta: 1 | -1): void {
  emit('select', stepSystem(props.ids, props.terminalId, delta))
}
function onRailKey(e: KeyboardEvent): void {
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
  e.preventDefault()
  const next = stepSystem(props.ids, props.terminalId, e.key === 'ArrowDown' ? 1 : -1)
  emit('select', next)
  void nextTick(() => root.value?.querySelector<HTMLElement>(`[data-focus-rail="${CSS.escape(next)}"]`)?.focus())
}

function load(): void {
  connection.loadToolSpans(props.terminalId, new Date(Date.now() - LOOKBACK_MS).toISOString())
}
onMounted(() => {
  load()
  root.value?.focus({ preventScroll: true })
})
watch(() => props.terminalId, load)
watch(
  () => connection.state,
  (state, prev) => {
    if (state === 'connected' && prev !== 'connected') load()
  },
)

const wide = computed(() => !ui.narrow)
</script>

<template>
  <div ref="root" tabindex="-1" class="flex min-h-0 min-w-0 flex-1 outline-none" :class="wide ? 'flex-row' : 'flex-col'" role="group" :aria-label="t('monitorV88.orbit.focusRegion')" data-orbit-focus>
    <nav v-if="wide" class="flex w-56 shrink-0 flex-col gap-1 overflow-y-auto border-r border-line bg-canvas p-2" :aria-label="t('monitorV88.orbit.systems')" data-focus-rail @keydown="onRailKey">
      <h2 class="px-1 pb-1 text-xs font-semibold text-ink-muted">{{ t('monitorV88.orbit.systems') }}</h2>
      <button
        v-for="id in ids"
        :key="id"
        type="button"
        class="flex min-w-0 cursor-pointer items-center gap-2 rounded-card border px-1.5 py-1 text-left outline-none transition-colors hover:bg-raised focus-visible:ring-2 focus-visible:ring-focus"
        :class="[id === terminalId ? 'border-line-strong bg-raised' : 'border-transparent', groupOf(terminals.byId[id]?.status ?? 'stale') === 'needs' ? 'text-warning' : '']"
        :aria-current="id === terminalId ? 'true' : undefined"
        :data-focus-rail="id"
        @click="emit('select', id)"
      >
        <svg width="28" height="28" viewBox="0 0 28 28" class="shrink-0" aria-hidden="true">
          <circle cx="14" cy="14" r="12.5" fill="none" stroke="var(--color-line-strong)" stroke-dasharray="2 3" />
          <circle cx="14" cy="14" r="5" :fill="railSun(id)" />
          <circle v-if="railSubs(id) > 0" cx="23" cy="8" r="3" fill="var(--color-accent)" />
        </svg>
        <span class="flex min-w-0 flex-1 flex-col text-xs">
          <span class="flex min-w-0 items-center gap-1 font-semibold text-ink"><PresetIcon :preset="terminals.byId[id]?.provider ?? 'claude'" :size="12" /><span class="truncate">{{ railLabel(id) }}</span></span>
          <span class="flex min-w-0 items-center gap-1 text-ink-muted"><StatusIcon :status="terminals.byId[id]?.status ?? 'unknown'" :size="11" /><span class="truncate">{{ t(statusMeta(terminals.byId[id]?.status).labelKey) }}</span></span>
        </span>
      </button>
    </nav>

    <div class="flex min-h-0 min-w-0 flex-1 flex-col">
      <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-3 py-1.5 text-xs">
        <button type="button" class="ccm-btn text-xs" aria-keyshortcuts="Escape" data-focus-back @click="emit('back')">
          <CornerUpLeft :size="14" aria-hidden="true" />{{ t('monitorV88.orbit.back') }}
        </button>
        <button type="button" class="ccm-btn-plain ccm-press" :aria-label="t('monitorV88.orbit.prevSystem')" :title="t('monitorV88.orbit.prevSystem')" :disabled="ids.length < 2" data-focus-prev @click="step(-1)">
          <ChevronLeft :size="16" aria-hidden="true" />
        </button>
        <button type="button" class="ccm-btn-plain ccm-press" :aria-label="t('monitorV88.orbit.nextSystem')" :title="t('monitorV88.orbit.nextSystem')" :disabled="ids.length < 2" data-focus-next @click="step(1)">
          <ChevronRight :size="16" aria-hidden="true" />
        </button>
        <span v-if="terminal" class="flex min-w-0 items-center gap-1.5 font-semibold text-ink">
          <span class="inline-flex shrink-0" role="img" :aria-label="t(`monitorV88.agentLabel.${provider}`)"><PresetIcon :preset="provider" :size="14" /></span>
          <span class="truncate" :title="projectName" data-focus-title>{{ displayTitle(terminal) }}</span>
        </span>
        <span v-if="terminal" class="flex shrink-0 items-center gap-1 font-medium tabular-nums" :class="group === 'needs' ? 'text-warning' : meta.text" data-focus-status>
          <StatusIcon :status="terminal.status" :size="13" />{{ t(meta.labelKey) }}<template v-if="elapsed"> · {{ elapsed }}</template>
        </span>
        <OpenPaneButton v-if="terminal" :terminal-id="terminal.id" :reply="group === 'needs'" compact class="ml-auto" />
      </div>

      <div ref="stage" class="relative min-h-0 flex-1 overflow-hidden bg-canvas">
        <svg class="absolute inset-0 h-full w-full" aria-hidden="true">
          <circle v-for="(r, i) in layout.rings" :key="`r-${i}`" :cx="layout.cx" :cy="layout.cy" :r="r" fill="none" stroke="var(--color-line-strong)" stroke-dasharray="3 5" data-focus-ring />
          <circle :cx="layout.cx" :cy="layout.cy" :r="layout.mainR" fill="none" stroke="var(--color-line-strong)" stroke-dasharray="3 5" />
          <circle :cx="layout.cx" :cy="layout.cy" :r="layout.sunR * 1.5" :fill="SUN_HALO[group]" />
          <circle :cx="layout.cx" :cy="layout.cy" :r="layout.sunR" :fill="SUN_FILL[group]" data-focus-sun />
          <path
            v-for="tr in layout.trails"
            :key="tr.id"
            :d="arcPath(layout.cx, layout.cy, tr.r, tr.a0, tr.a1)"
            fill="none"
            :stroke="OUTCOME_STROKE[tr.running ? 'running' : (tr.outcome ?? 'ok')]"
            stroke-width="5"
            stroke-linecap="round"
            :opacity="trailOpacity(tr.running)"
            :data-focus-trail="tr.agentId"
          />
          <g v-for="p in layout.planets" :key="p.id" :data-focus-planet="p.id">
            <circle :cx="p.x" :cy="p.y" :r="PLANET_RADIUS" :fill="planetFill(p.status).fill" :stroke="planetFill(p.status).stroke" stroke-width="2" />
            <text :x="p.x + (anchorOf(p.angle) === 'start' ? 16 : -16)" :y="p.y - 2" :text-anchor="anchorOf(p.angle)" font-size="12" font-weight="600" fill="var(--color-ink)">{{ clip(agentName(agentOf(p.id)), 18) }}</text>
            <text :x="p.x + (anchorOf(p.angle) === 'start' ? 16 : -16)" :y="p.y + 13" :text-anchor="anchorOf(p.angle)" font-size="11" fill="var(--color-ink-muted)">{{ clip(toolText(agentOf(p.id)), 24) }}</text>
          </g>
          <text :x="layout.cx" :y="layout.cy + layout.sunR * 1.5 + 16" text-anchor="middle" font-size="12" font-weight="600" fill="var(--color-ink)" data-focus-sun-tool>{{ clip(toolText(main), 32) }}</text>
        </svg>
        <p class="pointer-events-none absolute inset-x-3 bottom-2 text-xs text-ink-muted">{{ t('monitorV88.orbit.trailHint') }}</p>
      </div>
    </div>

    <aside class="flex shrink-0 flex-col gap-1 overflow-y-auto bg-canvas p-3 text-xs" :class="wide ? 'w-72 border-l border-line' : 'max-h-[40%] border-t border-line'" :aria-label="t('monitorV88.orbit.feed')" data-focus-feed>
      <h2 class="font-semibold text-ink">{{ t('monitorV88.orbit.feed') }}</h2>
      <p v-if="!feed.length" class="text-ink-muted">{{ t('monitorV88.orbit.feedEmpty') }}</p>
      <ul v-else class="flex flex-col gap-1">
        <li v-for="r in feed" :key="r.id" class="flex min-w-0 items-baseline gap-2 tabular-nums" :class="r.state === 'running' ? 'text-accent-ink' : 'text-ink'" data-focus-row>
          <span class="shrink-0 font-mono text-2xs text-ink-muted">{{ clockTime(r.start, locale) }}</span>
          <span class="min-w-0 flex-1 truncate" :title="`${r.tool} · ${agentName(agentOf(r.agentId))}`">
            {{ r.tool }}<span v-if="agentOf(r.agentId)?.role !== 'main'" class="text-ink-muted"> · {{ agentName(agentOf(r.agentId)) }}</span>
          </span>
          <span class="shrink-0 text-ink-muted" :class="r.state === 'error' ? 'text-st-error' : ''">{{ r.ms !== null ? formatMs(r.ms) : t(`monitorV88.orbit.outcome.${r.state}`) }}<template v-if="r.state === 'error' || r.state === 'interrupted'"> · {{ t(`monitorV88.orbit.outcome.${r.state}`) }}</template></span>
        </li>
      </ul>
    </aside>
  </div>
</template>
