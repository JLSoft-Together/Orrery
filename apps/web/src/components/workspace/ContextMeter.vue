<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { TriangleAlert } from 'lucide-vue-next'
import { compactNumber } from '../../lib/format'
import { meterState, type MeterTone } from '../../lib/paneV93'

const props = defineProps<{ tokens: number | undefined; limit: number | undefined; canCompact: boolean }>()
const emit = defineEmits<{ compact: [] }>()
const { t, locale } = useI18n()

const state = computed(() => meterState(props.tokens, props.limit))
const FILL: Record<MeterTone, string> = { ok: 'bg-ink-faint', warn: 'bg-warning', over: 'bg-st-error' }
const nowText = computed(() => compactNumber(state.value?.now, locale.value))
const maxText = computed(() => compactNumber(state.value?.max, locale.value))
const title = computed(() => t('paneV93.meter.title', { now: nowText.value, max: maxText.value }))
</script>

<template>
  <div v-if="state" class="shrink-0" :data-pane-meter="state.tone">
    <div
      class="h-[3px] w-full overflow-hidden rounded-full bg-fill"
      role="meter"
      :aria-label="t('paneV93.meter.aria')"
      aria-valuemin="0"
      :aria-valuemax="state.max"
      :aria-valuenow="Math.min(state.now, state.max)"
      :aria-valuetext="title"
      :title="title"
    >
      <div class="h-full origin-left rounded-full transition-[transform,background-color] duration-(--ccm-dur-base) ease-out-quint" :class="FILL[state.tone]" :style="{ transform: `scaleX(${state.fill})`, width: '100%' }" />
    </div>
    <div v-if="state.tone === 'over'" class="flex h-6 items-center gap-1.5 px-1 text-2xs text-st-error" data-pane-meter-over>
      <TriangleAlert :size="12" class="shrink-0" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate">{{ t('paneV93.meter.over', { now: nowText, max: maxText }) }}</span>
      <button v-if="canCompact" type="button" class="ccm-btn h-5 shrink-0 px-2 text-2xs" :title="t('paneV93.meter.compactTitle')" data-pane-meter-compact @click="emit('compact')">
        {{ t('paneV93.meter.compact') }}
      </button>
    </div>
  </div>
</template>
