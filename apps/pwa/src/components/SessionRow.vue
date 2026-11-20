<script setup lang="ts">
import { Check, Clock, LoaderCircle, MessageSquareReply, MoonStar, TriangleAlert, WifiOff } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatElapsed, isRunning, type MonitorSession } from '../lib/monitorModel'

const props = withDefaults(defineProps<{ session: MonitorSession; now: number; lost?: boolean; prefix?: string; label?: string; answer?: boolean }>(), { prefix: 'session' })
const emit = defineEmits<{ open: []; answer: [] }>()
const { t } = useI18n()

const elapsed = computed(() => (props.session.since ? formatElapsed(props.now - props.session.since, t) : null))
const canAnswer = computed(() => props.answer === true && !props.lost && props.session.rail === 'asked')
const activeSubs = computed(() => props.session.subagents.filter((s) => isRunning(s.rail)).length)
const detail = computed(() => {
  const s = props.session
  if (s.tool) return s.toolSummary ? `${s.tool} · ${s.toolSummary}` : s.tool
  if (s.waitingFor) return s.waitingFor
  if (activeSubs.value > 0) return t('monitor.subagentsRunning', { n: activeSubs.value })
  return null
})
const meta = computed(() => {
  if (props.lost) return { icon: WifiOff, tone: 'idle', text: t('monitor.lost'), spin: false }
  switch (props.session.rail) {
    case 'working':
      return { icon: LoaderCircle, tone: 'working', text: t('status.working'), spin: true }
    case 'asked':
      return { icon: Clock, tone: 'waiting', text: t('status.asked'), spin: false }
    case 'failed':
      return { icon: TriangleAlert, tone: 'error', text: t('status.failed'), spin: false }
    case 'done':
      return { icon: Check, tone: 'done', text: t('status.done'), spin: false }
    default:
      return { icon: MoonStar, tone: 'idle', text: t('status.idle'), spin: false }
  }
})
</script>

<template>
  <div class="item">
    <button type="button" class="row" :class="[`p-${session.provider}`, `tone-${meta.tone}`, { lost }]" :data-testid="`${prefix}-${session.id}`" :data-rail="session.rail" @click="emit('open')">
      <span class="mark" data-testid="status-chip" :data-rail="session.rail"><component :is="meta.icon" :size="22" :stroke-width="2.2" :class="{ spin: meta.spin }" aria-hidden="true" /></span>
      <span class="text">
        <span class="line1">
          <span class="title">{{ label ?? session.title }}</span>
          <span v-if="elapsed" class="since">{{ elapsed }}</span>
        </span>
        <span class="line2">
          <span class="agent">{{ t(`provider.${session.provider}`) }}</span>
          <span class="state">{{ meta.text }}</span>
          <span v-if="detail" class="detail" data-testid="session-tool">{{ detail }}</span>
        </span>
      </span>
    </button>
    <button v-if="canAnswer" type="button" class="answer" :aria-label="t('monitor.answerFor', { name: label ?? session.title })" :data-testid="`answer-${session.id}`" @click="emit('answer')">
      <MessageSquareReply :size="18" :stroke-width="2.2" aria-hidden="true" />
      {{ t('monitor.answer') }}
    </button>
  </div>
</template>

<style scoped>
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  border-top: 1px solid var(--ccm-line);
}

.item:has(.answer) {
  padding-right: 12px;
}

.item:first-child {
  border-top: 0;
}

.answer {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 44px;
  padding: 0 14px;
  border: 1px solid var(--ccm-waiting);
  border-radius: var(--radius-control);
  background: color-mix(in srgb, var(--ccm-waiting) 12%, var(--ccm-surface));
  color: var(--ccm-ink);
  font-weight: 700;
}

.answer:active {
  background: var(--ccm-fill);
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
  min-height: 60px;
  padding: 10px 16px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
  transition: background-color var(--ccm-dur-fast) var(--ccm-ease-standard);
}

.row:active {
  background: var(--ccm-fill);
}

.row.lost {
  opacity: 0.7;
}

.mark {
  display: grid;
  place-items: center;
  flex: none;
  width: 28px;
  height: 28px;
  color: var(--tone-color, var(--ccm-idle));
  transition: color var(--ccm-dur-slow) var(--ccm-ease-standard);
}

.tone-working {
  --tone-color: var(--ccm-working);
}

.tone-waiting {
  --tone-color: var(--ccm-waiting);
}

.tone-error {
  --tone-color: var(--ccm-error);
}

.tone-done {
  --tone-color: var(--ccm-done);
}

.tone-idle {
  --tone-color: var(--ccm-idle);
}

.text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.line1,
.line2 {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.title {
  flex: 1;
  min-width: 0;
  font-size: 1rem;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.since {
  flex: none;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
  font-variant-numeric: tabular-nums;
}

.line2 {
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
  white-space: nowrap;
}

.agent {
  flex: none;
  font-weight: 700;
  color: var(--ccm-agent-claude);
}

.p-codex .agent {
  color: var(--ccm-agent-codex);
}

.p-shell .agent {
  color: var(--ccm-agent-shell);
}

.state {
  flex: none;
  color: var(--tone-color);
  font-weight: 600;
}

.detail {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  overflow: hidden;
  text-overflow: ellipsis;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
