<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Hand, LayoutGrid, Orbit } from 'lucide-vue-next'
import type { ProjectGroup } from '../../lib/projectGroups'
import { countOf } from '../../lib/monitorRows'
import type { SessionGroup } from '../../lib/sessionGroups'
import { useUiStore, type MonitorMode } from '../../stores/ui'
import SegmentedControl, { type Segment } from '../ui/SegmentedControl.vue'
import { BOARD_GROUP_BY, type BoardGroupBy } from '../../lib/boardV95'
import ProjectStrip from '../ProjectStrip.vue'
import StatusIcon from '../StatusIcon.vue'

const props = defineProps<{ projects: readonly ProjectGroup[]; groups: readonly SessionGroup[] }>()
const { t } = useI18n()
const ui = useUiStore()

const MODES: { id: MonitorMode; icon: typeof LayoutGrid; n: number }[] = [
  { id: 'board', icon: LayoutGrid, n: 1 },
  { id: 'sky', icon: Orbit, n: 2 },
]
const groupOptions = computed<Segment<BoardGroupBy>[]>(() => BOARD_GROUP_BY.map((value) => ({ value, label: t(`boardV95.groupBy.${value}`) })))
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
    <div ref="tabs" role="tablist" :aria-label="t('monitorV88.modes')" class="inline-flex items-center gap-0.5 rounded-control bg-raised p-0.5" @keydown="onKey">
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
        :class="ui.monitorMode === m.id ? 'bg-surface text-ink shadow-card' : 'text-ink-muted hover:text-ink'"
        @click="ui.monitorMode = m.id"
      >
        <component :is="m.icon" :size="14" aria-hidden="true" />{{ t(`monitorV88.${m.id}`) }}
      </button>
    </div>
    <SegmentedControl v-if="ui.monitorMode === 'board'" v-model="ui.monitorGroupBy" :options="groupOptions" :label="t('boardV95.groupBy.label')" class="ms-auto" data-group-by />
    <ProjectStrip v-if="ui.monitorMode === 'board' && ui.monitorGroupBy === 'status'" v-model="ui.monitorProject" :groups="projects" />
  </div>
</template>
