<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { contextUsedPct } from '@ccm/shared'
import { gaugeLevel } from './chipTone'

const props = withDefaults(defineProps<{ tokens?: number; window?: number; pct?: number }>(), { tokens: undefined, window: undefined, pct: undefined })
const { t } = useI18n()

const value = computed<number | null>(() => {
  if (props.pct !== undefined) return props.pct
  if (props.tokens === undefined || !props.window) return null
  return contextUsedPct(props.tokens, props.window)
})
const level = computed(() => (value.value === null ? 'ok' : gaugeLevel(value.value)))
const label = computed(() => (value.value === null ? t('mist.ui.contextUnknown') : t('mist.ui.context', { pct: Math.round(value.value) })))
</script>

<template>
  <span
    class="inline-block h-1 w-7 shrink-0 overflow-hidden rounded-full bg-line-strong"
    role="meter"
    :aria-valuenow="value === null ? undefined : Math.round(value)"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-label="label"
    :title="label"
    :data-level="level"
    data-next-gauge
  >
    <span
      v-if="value !== null"
      class="block h-full origin-left rounded-full transition-transform duration-(--ccm-dur-slow) ease-(--ccm-ease-standard)"
      :class="level === 'high' ? 'bg-st-waiting' : 'bg-ink-muted'"
      :style="{ transform: `scaleX(${Math.min(100, Math.max(0, value)) / 100})` }"
    />
  </span>
</template>
