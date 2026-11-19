<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { WsPane } from '../lib/workspaceModel'

const props = defineProps<{ panes: WsPane[]; current: string | undefined }>()
const emit = defineEmits<{ pick: [key: string] }>()
const { t } = useI18n()

const labels = computed(() => {
  const counts = new Map<string, number>()
  for (const p of props.panes) counts.set(p.provider, (counts.get(p.provider) ?? 0) + 1)
  return new Map(props.panes.map((p) => [p.key, (counts.get(p.provider) ?? 0) > 1 ? p.title : t(`provider.${p.provider}`)]))
})
</script>

<template>
  <TransitionGroup tag="div" name="pane-list" class="chips" data-testid="ws-chips">
    <button v-for="p in panes" :key="p.key" type="button" class="chip" :class="[{ on: p.key === current }, `p-${p.provider}`]" :data-testid="`chip-${p.key}`" @click="emit('pick', p.key)">
      <span class="dot" aria-hidden="true" /><span class="chip-text">{{ labels.get(p.key) }}</span>
    </button>
  </TransitionGroup>
</template>

<style scoped>
.chips {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  scrollbar-width: none;
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
