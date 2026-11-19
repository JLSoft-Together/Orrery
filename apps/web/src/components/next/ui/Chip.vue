<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { statusMeta } from '../../../lib/status'
import StatusIcon from '../../StatusIcon.vue'
import { chipTone, type ChipTone } from './chipTone'

const props = withDefaults(defineProps<{ tone?: ChipTone; status?: string; label?: string; icon?: boolean }>(), {
  tone: undefined,
  status: undefined,
  label: undefined,
  icon: true,
})
const { t } = useI18n()

const TONE: Record<ChipTone, string> = {
  working: 'bg-st-working-soft text-st-working',
  waiting: 'bg-st-waiting-soft text-st-waiting',
  error: 'bg-st-error-soft text-st-error',
  done: 'bg-st-done-soft text-st-done',
  idle: 'bg-fill text-st-idle',
  meta: 'bg-fill text-ink-muted',
  accent: 'bg-accent-soft text-accent-ink',
  warning: 'bg-warning-soft text-warning',
}

const tone = computed<ChipTone>(() => props.tone ?? (props.status ? chipTone(props.status) : 'meta'))
const text = computed(() => props.label ?? (props.status ? t(statusMeta(props.status).labelKey) : ''))
</script>

<template>
  <span class="inline-flex h-5 max-w-full shrink-0 items-center gap-1 rounded-full px-[7px] text-2xs font-semibold whitespace-nowrap tabular-nums" :class="TONE[tone]" data-next-chip :data-tone="tone">
    <slot name="icon">
      <StatusIcon v-if="status && icon" :status="status" :size="12" />
    </slot>
    <span class="min-w-0 truncate"><slot>{{ text }}</slot></span>
  </span>
</template>
