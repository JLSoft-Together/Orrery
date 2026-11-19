<script setup lang="ts">
import { Ellipsis } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { WsPane } from '../lib/workspaceModel'
import PaneCard from './PaneCard.vue'

type GridFilter = 'all' | 'running' | 'waiting'

const props = defineProps<{ panes: WsPane[]; lost: boolean; viewOnly: boolean; selected?: string }>()
const emit = defineEmits<{ open: [key: string]; actions: [key: string] }>()
const { t } = useI18n()
const filter = ref<GridFilter>('all')

const isWaiting = (p: WsPane): boolean => p.rail === 'asked' || p.rail === 'failed'
const running = computed(() => props.panes.filter((p) => p.rail === 'working').length)
const waiting = computed(() => props.panes.filter(isWaiting).length)
const shown = computed(() => props.panes.filter((p) => filter.value === 'all' || (filter.value === 'running' ? p.rail === 'working' : isWaiting(p))))
</script>

<template>
  <div class="filters" data-testid="ws-filters">
    <button type="button" class="filter" :class="{ on: filter === 'all' }" data-testid="ws-filter-all" @click="filter = 'all'">{{ t('workspace.all', { n: panes.length }) }}</button>
    <button type="button" class="filter" :class="{ on: filter === 'running' }" data-testid="ws-filter-running" @click="filter = 'running'">{{ t('workspace.running', { n: running }) }}</button>
    <button type="button" class="filter" :class="{ on: filter === 'waiting' }" data-testid="ws-filter-waiting" @click="filter = 'waiting'">{{ t('workspace.waiting', { n: waiting }) }}</button>
  </div>
  <TransitionGroup tag="div" name="pane-list" class="grid" data-testid="ws-grid">
    <div v-for="p in shown" :key="p.key" class="cell">
      <PaneCard :pane="p" :lost="lost" :selected="selected === p.key" @open="emit('open', p.key)" />
      <button type="button" class="more" :disabled="lost || viewOnly" :aria-label="t('paneActions.title')" :data-testid="`more-${p.key}`" @click="emit('actions', p.key)"><Ellipsis :size="18" :stroke-width="2" aria-hidden="true" /></button>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.filters {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  scrollbar-width: none;
}

.filter {
  flex: none;
  min-height: 40px;
  padding: 0 16px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-weight: 600;
}

.filter.on {
  border-color: var(--ccm-accent);
  background: var(--ccm-selection);
}

.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.cell {
  position: relative;
}

.more {
  position: absolute;
  top: 4px;
  right: 4px;
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.pane-list-enter-active,
.pane-list-leave-active,
.pane-list-move {
  transition: opacity 220ms var(--ease-out), transform 220ms var(--ease-out);
}

.pane-list-enter-from,
.pane-list-leave-to {
  opacity: 0;
  transform: scale(0.92);
}

.pane-list-leave-active {
  position: absolute;
}
</style>
