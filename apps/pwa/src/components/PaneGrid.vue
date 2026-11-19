<script setup lang="ts">
import { ChevronDown, Ellipsis } from 'lucide-vue-next'
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { monogram, type WsGroup, type WsPane } from '../lib/workspaceModel'
import PaneCard from './PaneCard.vue'

type GridFilter = 'all' | 'running' | 'waiting'

const props = defineProps<{ panes: WsPane[]; lost: boolean; viewOnly: boolean; selected?: string; groups?: WsGroup[] }>()
const emit = defineEmits<{ open: [key: string]; actions: [key: string] }>()
const { t } = useI18n()
const filter = ref<GridFilter>('all')

const isWaiting = (p: WsPane): boolean => p.rail === 'asked' || p.rail === 'failed'
const running = computed(() => props.panes.filter((p) => p.rail === 'working').length)
const waiting = computed(() => props.panes.filter(isWaiting).length)
const shown = computed(() => props.panes.filter((p) => filter.value === 'all' || (filter.value === 'running' ? p.rail === 'working' : isWaiting(p))))
const folds = reactive(new Map<string, boolean>())
const isFolded = (g: WsGroup): boolean => folds.get(g.key) ?? g.folded
const toggle = (g: WsGroup): void => {
  folds.set(g.key, !isFolded(g))
}
const keep = computed(() => new Set(shown.value.map((p) => p.key)))
const sections = computed(() => (props.groups ?? []).map((g) => ({ g, panes: g.panes.filter((p) => keep.value.has(p.key)) })).filter((x) => x.panes.length > 0))
</script>

<template>
  <div class="filters" data-testid="ws-filters">
    <button type="button" class="filter" :class="{ on: filter === 'all' }" data-testid="ws-filter-all" @click="filter = 'all'">{{ t('workspace.all', { n: panes.length }) }}</button>
    <button type="button" class="filter" :class="{ on: filter === 'running' }" data-testid="ws-filter-running" @click="filter = 'running'">{{ t('workspace.running', { n: running }) }}</button>
    <button type="button" class="filter" :class="{ on: filter === 'waiting' }" data-testid="ws-filter-waiting" @click="filter = 'waiting'">{{ t('workspace.waiting', { n: waiting }) }}</button>
  </div>
  <div v-if="groups && groups.length > 0" class="sections" data-testid="ws-grid">
    <section v-for="x in sections" :key="x.g.key" class="grp" :class="{ folded: isFolded(x.g) }" :data-testid="`ws-group-${x.g.key}`" :data-folded="isFolded(x.g) ? '1' : '0'">
      <button type="button" class="gh" :aria-expanded="!isFolded(x.g)" :data-testid="`ws-group-toggle-${x.g.key}`" @click="toggle(x.g)">
        <ChevronDown class="chev" :size="18" :stroke-width="2.2" aria-hidden="true" />
        <span class="tone-mono" :data-tone="x.g.tone" aria-hidden="true">{{ monogram(x.g.name) }}</span>
        <span class="gname">{{ x.g.name }}</span>
        <span class="gcount">{{ x.panes.length }}</span>
      </button>
      <div class="gbody"><div class="gin">
        <div class="grid">
          <div v-for="p in x.panes" :key="p.key" class="cell">
            <PaneCard :pane="p" :lost="lost" :selected="selected === p.key" @open="emit('open', p.key)" />
            <button type="button" class="more" :disabled="lost || viewOnly" :aria-label="t('paneActions.title')" :data-testid="`more-${p.key}`" @click="emit('actions', p.key)"><Ellipsis :size="18" :stroke-width="2" aria-hidden="true" /></button>
          </div>
        </div>
      </div></div>
    </section>
  </div>
  <TransitionGroup v-else tag="div" name="pane-list" class="grid" data-testid="ws-grid">
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

.sections {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.gh {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 44px;
  padding: 0 4px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
  font-weight: 700;
  text-align: left;
}

.gname {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gcount {
  min-width: 22px;
  padding: 1px 8px;
  border-radius: var(--radius-chip);
  background: var(--ccm-control);
  color: var(--ccm-ink-muted);
  font-size: 0.75rem;
  font-weight: 700;
  text-align: center;
}

.chev {
  flex: none;
  color: var(--ccm-ink-faint);
  transition: transform var(--ccm-dur-base) var(--ccm-ease-standard);
}

.folded .chev {
  transform: rotate(-90deg);
}

.gbody {
  display: grid;
  grid-template-rows: 1fr;
  transition: grid-template-rows var(--ccm-dur-base) var(--ccm-ease-standard);
}

.folded .gbody {
  grid-template-rows: 0fr;
}

.gin {
  min-height: 0;
  overflow: hidden;
}

.folded .gin {
  visibility: hidden;
  transition: visibility 0s var(--ccm-dur-base);
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
