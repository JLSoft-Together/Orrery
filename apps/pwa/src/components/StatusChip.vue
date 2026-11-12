<script setup lang="ts">
import { Check, Clock, LoaderCircle, MoonStar, TriangleAlert, WifiOff } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Rail } from '../lib/monitorModel'

const props = defineProps<{ rail: Rail; lost?: boolean }>()
const { t } = useI18n()

const meta = computed(() => {
  if (props.lost) return { icon: WifiOff, tone: 'idle', text: t('monitor.lost'), spin: false }
  switch (props.rail) {
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
  <span class="chip" :class="`tone-${meta.tone}`" data-testid="status-chip" :data-rail="rail">
    <component :is="meta.icon" :size="14" :stroke-width="2.2" :class="{ spin: meta.spin }" aria-hidden="true" />
    <span>{{ meta.text }}</span>
  </span>
</template>

<style scoped>
.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid color-mix(in srgb, currentColor 38%, transparent);
  font-size: 0.8125rem;
  font-weight: 600;
  white-space: nowrap;
  transition:
    color 220ms var(--ease-out),
    border-color 220ms var(--ease-out);
}

.tone-working {
  color: var(--ccm-working);
}

.tone-waiting {
  color: var(--ccm-waiting);
}

.tone-error {
  color: var(--ccm-error);
}

.tone-done {
  color: var(--ccm-done);
}

.tone-idle {
  color: var(--ccm-idle);
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
