<script setup lang="ts">
import { GitBranch } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatElapsed, isRunning, type MonitorSession } from '../lib/monitorModel'
import AgentTile from './AgentTile.vue'
import StatusChip from './StatusChip.vue'

const props = defineProps<{ session: MonitorSession; now: number; lost?: boolean }>()
const emit = defineEmits<{ open: [] }>()
const { t } = useI18n()

const elapsed = computed(() => (props.session.since ? formatElapsed(props.now - props.session.since) : null))
const activeSubs = computed(() => props.session.subagents.filter((s) => isRunning(s.rail)).length)
const line = computed(() => {
  const s = props.session
  if (s.tool) return s.toolSummary ? `${s.tool} · ${s.toolSummary}` : s.tool
  if (s.waitingFor) return s.waitingFor
  if (activeSubs.value > 0) return t('monitor.subagentsRunning', { n: activeSubs.value })
  return null
})
</script>

<template>
  <button type="button" class="card session" :class="[`p-${session.provider}`, { lost }]" :data-testid="`session-${session.id}`" @click="emit('open')">
    <span class="bar" aria-hidden="true" />
    <span class="head">
      <AgentTile :provider="session.provider" />
      <span class="title">{{ session.title }}</span>
      <StatusChip :rail="session.rail" :lost="lost" />
    </span>
    <span v-if="session.branch || elapsed" class="meta">
      <span v-if="session.branch" class="branch"><GitBranch :size="14" :stroke-width="2" aria-hidden="true" /><span class="mono ellipsis">{{ session.branch }}</span></span>
      <span v-if="elapsed" class="mono since">{{ elapsed }}</span>
    </span>
    <span v-if="line" class="tool mono" data-testid="session-tool"><span aria-hidden="true">▸ </span>{{ line }}</span>
  </button>
</template>

<style scoped>
.session {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  padding: 14px 14px 14px 18px;
  text-align: left;
  overflow: hidden;
  min-height: 44px;
  color: var(--ccm-ink);
}

.session.lost {
  opacity: 0.62;
}

.bar {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--ccm-agent-claude);
}

.p-codex .bar {
  background: var(--ccm-agent-codex);
}

.p-shell .bar {
  background: var(--ccm-agent-shell);
}

.head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title {
  flex: 1;
  min-width: 0;
  font-size: 1.0625rem;
  font-weight: 600;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  color: var(--ccm-ink-muted);
  font-size: 0.8125rem;
}

.branch {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.since {
  color: var(--ccm-ink-faint);
  flex: none;
}

.tool {
  display: block;
  padding: 6px 8px;
  border-radius: var(--radius-inner);
  background: var(--ccm-raised);
  color: var(--ccm-ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
