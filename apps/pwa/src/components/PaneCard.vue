<script setup lang="ts">
import { GitBranch } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { previewLines, type WsPane } from '../lib/workspaceModel'
import AgentTile from './AgentTile.vue'
import StatusChip from './StatusChip.vue'

const props = defineProps<{ pane: WsPane; lost?: boolean; selected?: boolean; large?: boolean }>()
const emit = defineEmits<{ open: [] }>()
const { t } = useI18n()
const lines = computed(() => {
  const found = previewLines(props.pane)
  return found.length ? found : [t(`status.${props.pane.rail}`)]
})
</script>

<template>
  <button type="button" class="card pane" :class="[`p-${pane.provider}`, { selected, large }]" :data-testid="`pane-${pane.key}`" :data-rail="pane.rail" @click="emit('open')">
    <span class="bar" aria-hidden="true" />
    <span class="head">
      <AgentTile v-if="large" :provider="pane.provider" :size="40" />
      <span class="who">
        <span class="title">{{ pane.title }}</span>
        <span v-if="large && pane.branch" class="branch mono"><GitBranch :size="14" :stroke-width="2" aria-hidden="true" /><span class="ellipsis">{{ pane.branch }}</span></span>
      </span>
      <StatusChip v-if="large" :rail="pane.rail" :lost="lost" />
    </span>
    <StatusChip v-if="!large" :rail="pane.rail" :lost="lost" />
    <span class="out mono" aria-hidden="true">
      <span v-for="(line, i) in lines" :key="i" class="line">{{ line }}</span>
    </span>
  </button>
</template>

<style scoped>
.pane {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  width: 100%;
  padding: 12px 12px 12px 16px;
  text-align: left;
  overflow: hidden;
  min-height: 44px;
}

.bar {
  position: absolute;
  inset: 0 auto 0 0;
  width: 4px;
  background: var(--ccm-agent-claude);
}

.p-codex .bar {
  background: var(--ccm-agent-codex);
}

.p-shell .bar {
  background: var(--ccm-agent-shell);
}

.selected {
  border-color: var(--ccm-accent);
}

.head {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.who {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.large .title {
  font-size: 1.0625rem;
  font-weight: 700;
}

.branch {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.out {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  min-height: 56px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-size: 0.75rem;
  line-height: 1.4;
}

.large .out {
  min-height: 140px;
  font-size: 0.8125rem;
}

.line {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
