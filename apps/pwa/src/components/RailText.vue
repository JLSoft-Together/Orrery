<script setup lang="ts">
import { Check, Clock, LoaderCircle, MoonStar, TriangleAlert } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Rail } from '../lib/monitorModel'

const props = withDefaults(defineProps<{ rail: Rail; text?: boolean; size?: number }>(), { text: true, size: 14 })
const { t } = useI18n()

const META: Record<Rail, { icon: typeof Clock; tone: string; spin: boolean }> = {
  working: { icon: LoaderCircle, tone: 'working', spin: true },
  asked: { icon: Clock, tone: 'waiting', spin: false },
  failed: { icon: TriangleAlert, tone: 'error', spin: false },
  done: { icon: Check, tone: 'done', spin: false },
  idle: { icon: MoonStar, tone: 'idle', spin: false },
}
const meta = computed(() => META[props.rail])
</script>

<template>
  <span class="st" :class="`st-${meta.tone}`" :data-rail="rail">
    <component :is="meta.icon" :size="size" :stroke-width="2.2" :class="{ spin: meta.spin }" aria-hidden="true" />
    <span v-if="text">{{ t(`status.${rail}`) }}</span>
    <span v-else class="sr-only">{{ t(`status.${rail}`) }}</span>
  </span>
</template>

<style scoped>
.st {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: none;
  font-weight: 600;
  white-space: nowrap;
}

.st-working {
  color: var(--ccm-working);
}

.st-waiting {
  color: var(--ccm-waiting);
}

.st-done {
  color: var(--ccm-done);
}

.st-error {
  color: var(--ccm-error);
}

.st-idle {
  color: var(--ccm-ink-muted);
}

.spin {
  animation: spin 1.1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .spin {
    animation: none;
  }
}
</style>
