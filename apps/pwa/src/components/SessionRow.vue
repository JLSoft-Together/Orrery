<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatElapsed, isRunning, type MonitorSession } from '../lib/monitorModel'
import AgentTile from './AgentTile.vue'

const props = defineProps<{ session: MonitorSession; now: number }>()
const emit = defineEmits<{ open: [] }>()
const { t } = useI18n()

const elapsed = computed(() => (props.session.since ? formatElapsed(props.now - props.session.since, t) : null))
const activeSubs = computed(() => props.session.subagents.filter((s) => isRunning(s.rail)).length)
const sub = computed(() => {
  const parts: string[] = []
  if (props.session.branch) parts.push(props.session.branch)
  if (activeSubs.value > 0) parts.push(t('monitor.subagentsShort', { n: activeSubs.value }))
  return parts.join(' · ')
})
</script>

<template>
  <button type="button" class="row" :data-testid="`row-${session.id}`" @click="emit('open')">
    <AgentTile :provider="session.provider" :size="32" />
    <span class="text">
      <span class="title">{{ session.title }}</span>
      <span v-if="sub" class="sub">{{ sub }}</span>
    </span>
    <span v-if="elapsed" class="since">{{ elapsed }}</span>
  </button>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 56px;
  padding: 10px 14px;
  border: 0;
  border-top: 1px solid var(--ccm-line);
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.row:first-child {
  border-top: 0;
}

.text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--ccm-ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.since {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--ccm-ink-faint);
}
</style>
