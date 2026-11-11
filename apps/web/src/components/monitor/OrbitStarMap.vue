<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown, ChevronUp, Crosshair, Maximize2, ZoomIn, ZoomOut } from 'lucide-vue-next'
import type { AgentStatus, Provider } from '@ccm/shared'
import { duration, now } from '../../lib/format'
import { navigate, ringOf, type NavKey } from '../../lib/orbitRings'
import { isTyping } from '../../lib/shortcuts'
import { groupOf, type SessionGroup, type SessionGroupKey } from '../../lib/sessionGroups'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { agentVar, fitView, idleMinutes, layoutSlots, peerPath, planetLayout, slotColumns, statusRank, sunRadius, toolFile, zoomAt, type Fit, type PlanetLayout } from '../../lib/starMap'
import { statusMeta } from '../../lib/status'
import { waitDisplay } from '../../lib/wait'
import { useAgentsStore } from '../../stores/agents'
import { usePeersStore } from '../../stores/peers'
import { useSettingsStore } from '../../stores/settings'
import { useTerminalsStore } from '../../stores/terminals'
import StatusIcon from '../StatusIcon.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const props = defineProps<{ groups: readonly SessionGroup[]; selectedId: string | null }>()
const emit = defineEmits<{ select: [id: string]; open: [id: string]; menu: [e: MouseEvent, id: string]; keymenu: [e: KeyboardEvent, id: string] }>()

const { t } = useI18n()
const terminals = useTerminalsStore()
const agents = useAgentsStore()
const peers = usePeersStore()
const settings = useSettingsStore()

const stage = ref<HTMLElement | null>(null)
const size = ref({ w: 0, h: 0 })
const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(([entry]) => (size.value = { w: entry?.contentRect.width ?? 0, h: entry?.contentRect.height ?? 0 }))
watch(stage, (el, old) => {
  if (old) observer?.unobserve(old)
  if (el) observer?.observe(el)
})

const inView = ref(true)
const tabVisible = ref(typeof document === 'undefined' ? true : !document.hidden)
const animating = computed(() => inView.value && tabVisible.value)
const visObserver = typeof IntersectionObserver === 'undefined' ? null : new IntersectionObserver(([entry]) => (inView.value = entry?.isIntersecting ?? true))
const onVisibility = (): void => {
  tabVisible.value = !document.hidden
}
watch(stage, (el, old) => {
  if (old) visObserver?.unobserve(old)
  if (el) visObserver?.observe(el)
})
onMounted(() => document.addEventListener('visibilitychange', onVisibility))
onBeforeUnmount(() => {
  observer?.disconnect()
  visObserver?.disconnect()
  document.removeEventListener('visibilitychange', onVisibility)
  drag = null
})

interface Sun {
  id: string
  project: string
  group: SessionGroupKey
  rank: number
  provider: Provider
  status: string
  radius: number
  planets: PlanetLayout
  subCount: number
  subStats: { working: number; waiting: number; completed: number }
  x: number
  y: number
}

const projectOfTerminal = (cwd: string | undefined): string => {
  const dir = projectOf(cwd)
  return dir ? folderName(dir) : t('monitorV88.noProject')
}

const base = computed(() =>
  props.groups.flatMap((g) => g.ids).flatMap((id) => {
    const term = terminals.byId[id]
    if (!term) return []
    const subs = (agents.byTerminal[id] ?? []).filter((a) => a.role === 'subagent').map((a) => ({ id: a.id, status: a.status as AgentStatus }))
    const group = groupOf(term.status)
    const running = subs.filter((s) => s.status === 'working').length
    const radius = sunRadius(group, running, term.status === 'working')
    return [{ id, project: projectOfTerminal(term.cwd), group, provider: (term.provider ?? 'claude') as Provider, status: term.status as string, radius, subs }]
  }),
)

const slotMemory = new Map<number, ReadonlyMap<string, number>>()
const arranged = computed(() => layoutSlots(base.value.map((s) => ({ id: s.id, project: s.project })), size.value.w, slotMemory.get(slotColumns(size.value.w))))
watch(
  arranged,
  (r) => {
    slotMemory.set(r.cols, r.slots)
  },
  { immediate: true },
)

const suns = computed<Sun[]>(() => {
  const at = new Map(arranged.value.placements.map((p) => [p.id, p]))
  return base.value
    .map((s): Sun => {
      const p = at.get(s.id)
      return {
        id: s.id,
        project: s.project,
        group: s.group,
        rank: statusRank(s.group),
        provider: s.provider,
        status: s.status,
        radius: s.radius,
        planets: planetLayout(s.subs, s.radius),
        subCount: s.subs.length,
        subStats: {
          working: s.subs.filter((x) => x.status === 'working').length,
          waiting: s.subs.filter((x) => x.status === 'waiting').length,
          completed: s.subs.filter((x) => x.status === 'completed').length,
        },
        x: p?.x ?? 0,
        y: p?.y ?? 0,
      }
    })
    .sort((a, b) => a.rank - b.rank || a.id.localeCompare(b.id))
})

const byId = computed(() => new Map(suns.value.map((s) => [s.id, s])))

const links = computed(() =>
  peers.warm.flatMap((l) => {
    const a = byId.value.get(l.a)
    const b = byId.value.get(l.b)
    if (!a || !b) return []
    const count = l.kind === 'message' ? l.ab + l.ba : 0
    return [{ id: l.id, count, ...peerPath(a, b) }]
  }),
)

const view = ref<Fit>({ scale: 1, x: 0, y: 0 })
const moved = ref(false)
function refit(): void {
  moved.value = false
  view.value = fitView({ width: arranged.value.width, height: arranged.value.height }, { width: size.value.w, height: size.value.h })
}
watch(
  () => [arranged.value.width, arranged.value.height, size.value.w, size.value.h],
  () => {
    if (!moved.value) view.value = fitView({ width: arranged.value.width, height: arranged.value.height }, { width: size.value.w, height: size.value.h })
  },
  { immediate: true },
)

const worldStyle = computed(() => ({
  width: `${arranged.value.width}px`,
  height: `${arranged.value.height}px`,
  transform: `translate(${view.value.x}px, ${view.value.y}px) scale(${view.value.scale})`,
}))

function onWheel(e: WheelEvent): void {
  if ((e.target as HTMLElement | null)?.closest('aside, button')) return
  e.preventDefault()
  const box = stage.value?.getBoundingClientRect()
  if (!box) return
  moved.value = true
  view.value = zoomAt(view.value, e.deltaY < 0 ? 1.12 : 1 / 1.12, e.clientX - box.left, e.clientY - box.top)
}
function zoomBy(factor: number): void {
  moved.value = true
  view.value = zoomAt(view.value, factor, size.value.w / 2, size.value.h / 2)
}

let drag: { x: number; y: number; vx: number; vy: number; id: number } | null = null
function onPointerDown(e: PointerEvent): void {
  if (e.button !== 0 || (e.target as HTMLElement).closest('button, aside')) return
  drag = { x: e.clientX, y: e.clientY, vx: view.value.x, vy: view.value.y, id: e.pointerId }
  stage.value?.setPointerCapture(e.pointerId)
}
function onPointerMove(e: PointerEvent): void {
  if (!drag || drag.id !== e.pointerId) return
  moved.value = true
  view.value = { ...view.value, x: drag.vx + e.clientX - drag.x, y: drag.vy + e.clientY - drag.y }
}
function onPointerUp(e: PointerEvent): void {
  if (drag?.id === e.pointerId) drag = null
}
function endDrag(): void {
  drag = null
}

const PAN_STEP = 80
const PAN_KEYS: Record<string, [number, number]> = { ArrowLeft: [1, 0], ArrowRight: [-1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }
function onStageKey(e: KeyboardEvent): void {
  if (e.target !== e.currentTarget || e.ctrlKey || e.metaKey || e.altKey || e.isComposing) return
  const d = PAN_KEYS[e.key]
  if (!d) return
  e.preventDefault()
  moved.value = true
  view.value = { ...view.value, x: view.value.x + d[0] * PAN_STEP, y: view.value.y + d[1] * PAN_STEP }
}

function centreOn(id: string): void {
  const s = byId.value.get(id)
  if (!s) return
  moved.value = true
  view.value = { ...view.value, x: size.value.w / 2 - s.x * view.value.scale, y: size.value.h / 2 - s.y * view.value.scale }
}
function ensureVisible(id: string): void {
  const s = byId.value.get(id)
  if (!s) return
  const px = s.x * view.value.scale + view.value.x
  const py = s.y * view.value.scale + view.value.y
  const m = 48
  if (px < m || py < m || px > size.value.w - m || py > size.value.h - m) centreOn(id)
}
function focusSun(id: string): void {
  void nextTick(() => stage.value?.querySelector<HTMLElement>(`[data-orbit-node="${CSS.escape(id)}"]`)?.focus({ preventScroll: true }))
}
function goSelected(): void {
  const id = props.selectedId
  if (!id || !byId.value.has(id)) return
  centreOn(id)
  focusSun(id)
}
const tabStop = computed(() => (props.selectedId && byId.value.has(props.selectedId) ? props.selectedId : (suns.value[0]?.id ?? null)))

function activity(s: Sun): string {
  const term = terminals.byId[s.id]
  if (!term) return ''
  const main = agents.mainOf(s.id)
  if (s.group === 'needs') return waitDisplay(term.waitingFor, main?.currentTool, t)?.text ?? t(statusMeta(term.status).labelKey)
  if (term.status === 'working' && main?.currentTool) {
    const file = toolFile(main.currentTool, main.currentToolSummary)
    return file ? t('orbitV95.activityFile', { tool: main.currentTool, file }) : t('orbitV95.activity', { tool: main.currentTool })
  }
  if (s.group === 'idle') {
    const min = idleMinutes(term.statusSince, now.value)
    return min === null || min < 1 ? t('orbitV95.idleNow') : t('orbitV95.idleFor', { n: min })
  }
  return t(statusMeta(term.status).labelKey)
}

function statusText(s: Sun): string {
  const term = terminals.byId[s.id]
  if (!term) return ''
  const waited = s.group === 'needs' && term.statusSince ? duration(term.statusSince, now.value) : null
  const text = s.group === 'needs' ? t('orbitV95.waitingLabel') : t(statusMeta(term.status).labelKey)
  return waited ? `${text} · ${waited}` : text
}

function subSummary(s: Sun): string {
  if (!s.subCount) return ''
  const parts = [
    s.subStats.working ? t('orbitV95.subsRunning', { n: s.subStats.working }) : '',
    s.subStats.waiting ? t('orbitV95.subsWaiting', { n: s.subStats.waiting }) : '',
    s.subStats.completed ? t('orbitV95.subsDone', { n: s.subStats.completed }) : '',
  ].filter(Boolean)
  return parts.length ? t('orbitV95.subSummary', { subs: subsText(s.subCount), parts: parts.join(', ') }) : subsText(s.subCount)
}

function labelOf(s: Sun): string {
  const head = t('orbitV95.sunLabel', { project: s.project, agent: t(`monitorV88.agentLabel.${s.provider}`), status: statusText(s) })
  const subs = subSummary(s)
  return subs ? `${head}, ${subs}` : head
}

function sunColor(s: Sun): string {
  return s.group === 'needs' ? 'var(--color-st-waiting)' : agentVar(s.provider)
}

function sunStyle(s: Sun): Record<string, string> {
  const c = sunColor(s)
  const selected = props.selectedId === s.id
  return {
    '--sun': c,
    width: `${s.radius * 2}px`,
    height: `${s.radius * 2}px`,
    left: `${-s.radius}px`,
    top: `${-s.radius}px`,
    background: c,
    boxShadow: `0 0 0 ${selected ? 4 : 6}px color-mix(in srgb, ${c} 24%, transparent), 0 0 0 ${selected ? 12 : 14}px color-mix(in srgb, ${c} 11%, transparent)`,
  }
}

function dim(s: Sun): string {
  return s.group === 'idle' ? 'opacity-60' : s.group === 'ended' ? 'opacity-40' : ''
}

function planetClass(status: AgentStatus): string {
  if (status === 'working') return 'bg-accent'
  if (status === 'waiting') return 'border-2 border-accent bg-surface'
  if (status === 'completed') return 'rounded-[2px] bg-success'
  return 'bg-fill-strong'
}

function planetText(status: AgentStatus): string {
  return t('orbitV95.planetLabel', { status: t(statusMeta(status).labelKey) })
}

function subsText(n: number): string {
  return n === 1 ? t('orbitV95.subsOne') : t('orbitV95.subs', { n })
}

const nodes = computed(() => suns.value.map((s) => ({ id: s.id, x: s.x, y: s.y })))
const NAV_KEYS = new Set<string>(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'])
function onKey(e: KeyboardEvent, id: string): void {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey || e.isComposing || isTyping(e.target)) {
    if (e.shiftKey && e.key === 'F10') emit('keymenu', e, id)
    return
  }
  if (NAV_KEYS.has(e.key)) {
    e.preventDefault()
    const next = navigate(nodes.value, id, e.key as NavKey)
    if (next && next !== id) {
      emit('select', next)
      ensureVisible(next)
      focusSun(next)
    }
  } else if (e.key === 'Enter') {
    e.preventDefault()
    emit('open', id)
  } else if (e.key === 'ContextMenu') emit('keymenu', e, id)
}

const legendOpen = ref(true)
const legendTop = computed(() => (settings.officeEnabled ? 'top-12' : 'top-3'))
</script>

<template>
  <div
    ref="stage"
    class="ccm-sky relative min-h-0 min-w-0 flex-1 touch-none overflow-hidden outline-none select-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset"
    :class="animating ? '' : 'star-paused'"
    role="group"
    tabindex="0"
    :aria-label="t('orbitV95.region')"
    :aria-description="t('orbitV95.panHint')"
    data-orbit-rings
    data-star-map
    @wheel="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="endDrag"
    @lostpointercapture="endDrag"
    @keydown="onStageKey"
  >
    <div class="absolute top-0 left-0 origin-top-left" :style="worldStyle">
      <svg class="pointer-events-none absolute inset-0 overflow-visible" :width="arranged.width" :height="arranged.height" aria-hidden="true">
        <path v-for="l in links" :key="l.id" :d="l.d" fill="none" stroke="var(--color-accent)" stroke-opacity="0.45" stroke-width="1.5" stroke-dasharray="5 6" stroke-linecap="round" data-star-link />
      </svg>
      <template v-for="l in links" :key="`p-${l.id}`">
        <span
          v-if="l.count > 0"
          class="pointer-events-none absolute -translate-x-1/2 -translate-y-1/2 rounded-full border border-line bg-surface px-1.5 py-px text-2xs font-medium whitespace-nowrap text-ink-muted tabular-nums"
          :style="{ left: `${l.mx}px`, top: `${l.my}px` }"
          data-star-pill
        >
          {{ l.count === 1 ? t('orbitV95.messagesOne') : t('orbitV95.messages', { n: l.count }) }}
        </span>
      </template>

      <div v-for="s in suns" :key="s.id" class="absolute size-0 transition-[left,top] duration-(--ccm-dur-base) motion-reduce:transition-none" :class="dim(s)" :style="{ left: `${s.x}px`, top: `${s.y}px` }" :data-star-system="s.id">
        <div
          v-for="r in s.planets.rings"
          :key="r.ring"
          class="star-ring pointer-events-none absolute rounded-full border border-dashed border-line-strong"
          :class="s.group === 'running' || s.group === 'needs' ? 'star-spin' : ''"
          :style="{ width: `${r.radius * 2}px`, height: `${r.radius * 2}px`, left: `${-r.radius}px`, top: `${-r.radius}px`, animationDuration: `${r.seconds}s` }"
          aria-hidden="true"
        >
          <span
            v-for="p in s.planets.spots.filter((x) => x.ring === r.ring)"
            :key="p.id"
            class="absolute block size-2.5 rounded-full"
            :class="planetClass(p.status)"
            :style="{ left: `calc(50% + ${Math.cos((p.angle * Math.PI) / 180) * r.radius}px - 5px)`, top: `calc(50% + ${Math.sin((p.angle * Math.PI) / 180) * r.radius}px - 5px)` }"
            :title="planetText(p.status)"
            :data-orbit-sub="p.status"
          />
        </div>
        <span
          v-if="s.planets.more"
          class="pointer-events-none absolute text-2xs tabular-nums text-ink-muted"
          :style="{ left: `${s.radius + 36}px`, top: `${-s.radius - 30}px` }"
          aria-hidden="true"
        >
          {{ t('monitorV88.orbit.more', { n: s.planets.more }) }}
        </span>

        <button
          type="button"
          class="ccm-press absolute flex cursor-pointer items-center justify-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
          :class="[selectedId === s.id ? 'ring-2 ring-ink' : '', s.group === 'needs' ? 'star-needs' : '']"
          :style="sunStyle(s)"
          :aria-label="labelOf(s)"
          :aria-pressed="selectedId === s.id"
          :tabindex="tabStop === s.id ? 0 : -1"
          :title="labelOf(s)"
          :data-orbit-node="s.id"
          :data-ring="ringOf(terminals.byId[s.id]?.status ?? 'idle')"
          @click="emit('select', s.id)"
          @dblclick="emit('open', s.id)"
          @keydown="onKey($event, s.id)"
          @contextmenu="emit('menu', $event, s.id)"
        >
          <span class="flex size-4.5 items-center justify-center rounded-full bg-surface"><StatusIcon :status="terminals.byId[s.id]?.status ?? 'unknown'" :size="12" /></span>
        </button>

        <div class="absolute left-1/2 flex w-44 -translate-x-1/2 flex-col items-center gap-0.5 text-center" :style="{ top: `${s.radius + (s.planets.rings.length ? s.planets.rings.length * 16 + 12 : 12)}px` }">
          <span class="max-w-full truncate text-xs font-semibold text-ink" data-orbit-sector>{{ s.project }}</span>
          <span v-if="s.group === 'needs'" class="max-w-full text-2xs font-semibold text-st-waiting">{{ statusText(s) }}</span>
          <span class="max-w-full truncate text-2xs text-ink-muted" data-star-activity>{{ activity(s) }}</span>
          <span v-if="s.subCount" class="text-2xs text-ink-faint tabular-nums">{{ subsText(s.subCount) }}</span>
          <OpenPaneButton v-if="s.group === 'needs'" :terminal-id="s.id" compact class="pointer-events-auto mt-1" />
        </div>
      </div>
    </div>

    <p v-if="!suns.length" class="pointer-events-none absolute inset-x-0 top-1/2 text-center text-sm text-ink-muted" data-orbit-empty>{{ t('monitorV88.orbit.empty') }}</p>

    <div class="absolute bottom-3 left-3 flex flex-col gap-1">
      <button type="button" class="ccm-btn-plain ccm-press ccm-material" :aria-label="t('orbitV95.zoomIn')" :title="t('orbitV95.zoomIn')" @click="zoomBy(1.25)"><ZoomIn :size="16" aria-hidden="true" /></button>
      <button type="button" class="ccm-btn-plain ccm-press ccm-material" :aria-label="t('orbitV95.zoomOut')" :title="t('orbitV95.zoomOut')" @click="zoomBy(0.8)"><ZoomOut :size="16" aria-hidden="true" /></button>
      <button type="button" class="ccm-btn-plain ccm-press ccm-material" :aria-label="t('orbitV95.fit')" :title="t('orbitV95.fit')" @click="refit"><Maximize2 :size="16" aria-hidden="true" /></button>
      <button type="button" class="ccm-btn-plain ccm-press ccm-material" :disabled="!selectedId || !byId.has(selectedId)" :aria-label="t('orbitV95.goSelected')" :title="t('orbitV95.goSelected')" data-star-go-selected @click="goSelected"><Crosshair :size="16" aria-hidden="true" /></button>
    </div>

    <aside class="absolute right-3 w-64 max-w-[calc(100%-1.5rem)] rounded-card border border-line bg-surface p-2.5 text-xs text-ink shadow-pop" :class="legendTop" :aria-label="t('orbitV95.legendTitle')" data-orbit-legend>
      <button type="button" class="flex w-full items-center justify-between gap-2 text-left font-semibold outline-none focus-visible:ring-2 focus-visible:ring-focus" :aria-expanded="legendOpen" :title="t('orbitV95.legendToggle')" @click="legendOpen = !legendOpen">
        {{ t('orbitV95.legendTitle') }}
        <component :is="legendOpen ? ChevronUp : ChevronDown" :size="14" aria-hidden="true" />
      </button>
      <ul v-if="legendOpen" class="mt-2 flex flex-col gap-1.5 text-ink-muted">
        <li class="flex items-center gap-2"><span class="size-3.5 shrink-0 rounded-full bg-agent-claude" aria-hidden="true" />{{ t('orbitV95.legendSun') }}</li>
        <li class="flex items-center gap-2"><span class="size-2.5 shrink-0 rounded-full bg-accent" aria-hidden="true" />{{ t('orbitV95.legendPlanet') }}</li>
        <li class="flex items-center gap-2"><svg class="h-3 w-3.5 shrink-0" viewBox="0 0 14 12" aria-hidden="true"><path d="M1 10 Q7 -2 13 10" fill="none" stroke="var(--color-accent)" stroke-width="1.5" stroke-dasharray="3 3" /></svg>{{ t('orbitV95.legendLink') }}</li>
        <li class="flex items-center gap-2"><span class="size-3.5 shrink-0 rounded-full bg-st-waiting" aria-hidden="true" />{{ t('orbitV95.legendWaiting') }}</li>
        <li class="flex items-center gap-2"><span class="size-3.5 shrink-0 rounded-full bg-agent-claude opacity-50" aria-hidden="true" />{{ t('orbitV95.legendIdle') }}</li>
      </ul>
    </aside>
  </div>
</template>

<style scoped>
.star-paused .star-spin {
  animation-play-state: paused;
}
.star-spin {
  animation-name: star-orbit;
  animation-timing-function: linear;
  animation-iteration-count: infinite;
}
@keyframes star-orbit {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .star-spin {
    animation: none;
  }
}
</style>
