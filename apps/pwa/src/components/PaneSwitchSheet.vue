<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { WsPane } from '../lib/workspaceModel'
import AgentTile from './AgentTile.vue'
import BottomSheet from './BottomSheet.vue'
import StatusChip from './StatusChip.vue'

defineProps<{ title: string; panes: WsPane[]; current: string }>()
const emit = defineEmits<{ pick: [key: string]; close: [] }>()
const { t } = useI18n()
</script>

<template>
  <BottomSheet :title="title" testid="pane-sheet" @close="emit('close')">
    <ul class="list">
      <li v-for="pane in panes" :key="pane.key">
        <button type="button" class="row" :class="{ on: pane.key === current }" :data-testid="`pick-${pane.key}`" @click="emit('pick', pane.key)">
          <AgentTile :provider="pane.provider" :size="36" />
          <span class="who">
            <span class="name">{{ pane.title }}</span>
            <span class="sub">{{ t(`provider.${pane.provider}`) }}</span>
          </span>
          <StatusChip :rail="pane.rail" />
        </button>
      </li>
    </ul>
  </BottomSheet>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 56px;
  padding: 10px 12px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-control);
  text-align: left;
}

.row.on {
  border-color: var(--ccm-accent);
  background: var(--ccm-selection);
}

.who {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.sub {
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}
</style>
