<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { monogram, type WsGroup, type WsPane } from '../lib/workspaceModel'

const props = defineProps<{ panes: WsPane[]; current: string | undefined; groups?: WsGroup[] }>()
const emit = defineEmits<{ pick: [key: string] }>()
const { t } = useI18n()

const labels = computed(() => {
  const counts = new Map<string, number>()
  for (const p of props.panes) counts.set(p.provider, (counts.get(p.provider) ?? 0) + 1)
  return new Map(props.panes.map((p) => [p.key, (counts.get(p.provider) ?? 0) > 1 ? p.title : t(`provider.${p.provider}`)]))
})

type Entry = { kind: 'group'; key: string; group: WsGroup } | { kind: 'pane'; key: string; pane: WsPane }

const entries = computed<Entry[]>(() => {
  const groups = props.groups ?? []
  if (groups.length === 0) return props.panes.map((pane) => ({ kind: 'pane', key: pane.key, pane }))
  return groups.flatMap((group) => [{ kind: 'group' as const, key: `g:${group.key}`, group }, ...group.panes.map((pane) => ({ kind: 'pane' as const, key: pane.key, pane }))])
})
</script>

<template>
  <TransitionGroup tag="div" name="pane-list" class="chips" data-testid="ws-chips">
    <template v-for="e in entries" :key="e.key">
      <span v-if="e.kind === 'group'" class="gl" :data-testid="`chip-group-${e.group.key}`"><span class="tone-mono mini" :data-tone="e.group.tone" aria-hidden="true">{{ monogram(e.group.name) }}</span>{{ e.group.name }}</span>
      <button v-else type="button" class="chip" :class="[{ on: e.pane.key === current }, `p-${e.pane.provider}`]" :data-testid="`chip-${e.pane.key}`" @click="emit('pick', e.pane.key)">
        <span class="dot" aria-hidden="true" /><span class="chip-text">{{ labels.get(e.pane.key) }}</span>
      </button>
    </template>
  </TransitionGroup>
</template>

<style scoped>
.chips {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  scrollbar-width: none;
}

.gl {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
  min-height: 44px;
  padding: 0 4px 0 2px;
  color: var(--ccm-ink-muted);
  font-size: 0.8125rem;
  font-weight: 700;
}

.mini {
  width: 22px;
  height: 22px;
  font-size: 0.6875rem;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex: none;
  min-height: 44px;
  max-width: 200px;
  padding: 0 16px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-weight: 600;
}

.chip.on {
  border-color: var(--ccm-accent);
  background: var(--ccm-selection);
}

.chip-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dot {
  width: 9px;
  height: 9px;
  flex: none;
  border-radius: 50%;
  background: var(--ccm-agent-claude);
}

.p-codex .dot {
  background: var(--ccm-agent-codex);
}

.p-shell .dot {
  background: var(--ccm-agent-shell);
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
