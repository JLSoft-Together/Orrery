<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, Hand, LayoutGrid, Orbit, SlidersHorizontal } from 'lucide-vue-next'
import type { ProjectGroup } from '../../lib/projectGroups'
import { countOf } from '../../lib/monitorRows'
import type { SessionGroup } from '../../lib/sessionGroups'
import { useUiStore, type MonitorMode } from '../../stores/ui'
import { BOARD_GROUP_BY, type BoardGroupBy } from '../../lib/boardV95'
import StatusIcon from '../StatusIcon.vue'

const props = defineProps<{ projects: readonly ProjectGroup[]; groups: readonly SessionGroup[] }>()
const { t } = useI18n()
const ui = useUiStore()

const MODES: { id: MonitorMode; icon: typeof LayoutGrid; n: number }[] = [
  { id: 'board', icon: LayoutGrid, n: 1 },
  { id: 'sky', icon: Orbit, n: 2 },
]
const groupOptions = computed(() => BOARD_GROUP_BY.map((value) => ({ value, label: t(`boardV95.groupBy.${value}`) })))
const open = ref(false)
const menu = ref<HTMLElement | null>(null)
const canFilter = computed(() => ui.monitorGroupBy === 'status' && props.projects.length > 1)
const projectName = (g: ProjectGroup): string => g.name || t('sessions.projects.noProject')
const filtered = computed(() => (canFilter.value && ui.monitorProject ? props.projects.find((g) => g.key === ui.monitorProject) : undefined))
const viewLabel = computed(() => {
  const base = t(`boardV95.groupBy.${ui.monitorGroupBy}`)
  return filtered.value ? `${base}, ${projectName(filtered.value)}` : base
})
function pickGroup(value: BoardGroupBy): void {
  ui.monitorGroupBy = value
}
function pickProject(key: string | null): void {
  ui.monitorProject = key
  open.value = false
}
function onDoc(e: Event): void {
  if (menu.value && !menu.value.contains(e.target as Node)) open.value = false
}
function onMenuKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.stopPropagation()
    open.value = false
    menu.value?.querySelector<HTMLElement>('[data-view-toggle]')?.focus()
  }
}
watch(open, (v) => {
  if (v) document.addEventListener('pointerdown', onDoc)
  else document.removeEventListener('pointerdown', onDoc)
})
onBeforeUnmount(() => document.removeEventListener('pointerdown', onDoc))
const needs = computed(() => countOf(props.groups, 'needs'))
const running = computed(() => countOf(props.groups, 'running'))
const tabs = ref<HTMLElement | null>(null)

function onKey(e: KeyboardEvent): void {
  if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
  e.preventDefault()
  const next = ui.monitorMode === 'board' ? 'sky' : 'board'
  ui.monitorMode = next
  requestAnimationFrame(() => tabs.value?.querySelector<HTMLElement>(`[data-monitor-mode="${next}"]`)?.focus())
}
</script>

<template>
  <div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1.5 px-3 pt-2" data-monitor-bar>
    <h2 class="inline-flex min-h-7 items-center gap-1.5 text-sm font-semibold text-ink" data-monitor-summary :data-needs="needs" :data-running="running">
      <StatusIcon status="working" :size="14" />
      <span class="tabular-nums">{{ t('boardV95.summary.running', { n: running }, running) }},{{ ' ' }}<span :class="needs ? 'text-warning' : 'text-ink-muted'"><Hand v-if="needs" :size="14" aria-hidden="true" class="mr-1 inline align-[-2px]" />{{ needs ? t('boardV95.summary.waiting', { n: needs }) : t('boardV95.summary.noneWaiting') }}</span></span>
    </h2>
    <div ref="tabs" role="tablist" :aria-label="t('monitorV88.modes')" class="mist-seg inline-flex items-center gap-0.5 rounded-control bg-raised p-0.5" @keydown="onKey">
      <button
        v-for="m in MODES"
        :key="m.id"
        type="button"
        role="tab"
        :aria-selected="ui.monitorMode === m.id"
        :tabindex="ui.monitorMode === m.id ? 0 : -1"
        :id="`monitor-tab-${m.id}`"
        :aria-controls="`monitor-panel-${m.id}`"
        :data-monitor-mode="m.id"
        :aria-keyshortcuts="`Alt+${m.n}`"
        :title="t('monitorV88.modeHint', { n: m.n })"
        class="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-inner px-2.5 text-xs font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus"
        :class="ui.monitorMode === m.id ? 'mist-seg-on bg-surface text-ink shadow-card' : 'text-ink-muted hover:text-ink'"
        @click="ui.monitorMode = m.id"
      >
        <component :is="m.icon" :size="14" aria-hidden="true" />{{ t(`monitorV88.${m.id}`) }}
      </button>
    </div>
    <div v-if="ui.monitorMode === 'board'" ref="menu" class="relative ms-auto" @keydown="onMenuKey">
      <button
        type="button"
        class="ccm-btn h-7 gap-1.5 px-2.5 text-xs"
        :aria-expanded="open"
        aria-haspopup="true"
        :title="t('v107gon.monitor.viewTitle')"
        data-view-toggle
        @click="open = !open"
      >
        <SlidersHorizontal :size="14" aria-hidden="true" />{{ viewLabel }}
      </button>
      <div v-if="open" class="ccm-menu absolute end-0 top-full z-30 mt-1 flex w-56 flex-col gap-2 p-2" data-view-menu>
        <div role="radiogroup" :aria-label="t('boardV95.groupBy.label')" class="flex flex-col gap-0.5" data-group-by>
          <p class="px-2 text-xs font-semibold text-ink-muted">{{ t('v107gon.monitor.groupBy') }}</p>
          <button
            v-for="o in groupOptions"
            :key="o.value"
            type="button"
            role="radio"
            :aria-checked="ui.monitorGroupBy === o.value"
            class="flex h-7 cursor-pointer items-center rounded-inner px-2 text-start text-xs outline-none hover:bg-raised focus-visible:ring-2 focus-visible:ring-focus"
            :class="ui.monitorGroupBy === o.value ? 'font-semibold text-ink' : 'text-ink-muted'"
            @click="pickGroup(o.value)"
          >
            <span class="flex-1">{{ o.label }}</span><Check v-if="ui.monitorGroupBy === o.value" :size="14" aria-hidden="true" />
          </button>
        </div>
        <div v-if="canFilter" role="radiogroup" :aria-label="t('sessions.projects.label')" class="flex max-h-48 flex-col gap-0.5 overflow-y-auto" data-project-filter>
          <p class="px-2 text-xs font-semibold text-ink-muted">{{ t('v107gon.monitor.project') }}</p>
          <button
            v-for="g in [{ key: null, name: t('monitorV79.projectAll') }, ...projects]"
            :key="g.key ?? 'all'"
            type="button"
            role="radio"
            :aria-checked="ui.monitorProject === g.key"
            class="flex h-7 min-w-0 cursor-pointer items-center rounded-inner px-2 text-start text-xs outline-none hover:bg-raised focus-visible:ring-2 focus-visible:ring-focus"
            :class="ui.monitorProject === g.key ? 'font-semibold text-ink' : 'text-ink-muted'"
            @click="pickProject(g.key)"
          >
            <span class="min-w-0 flex-1 truncate">{{ g.name || t('sessions.projects.noProject') }}</span><Check v-if="ui.monitorProject === g.key" :size="14" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
