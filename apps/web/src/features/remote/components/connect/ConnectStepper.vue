<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Check } from 'lucide-vue-next'

defineProps<{ step: number }>()
const { t } = useI18n()
const KEYS = ['install', 'pair', 'test'] as const
</script>

<template>
  <ol class="flex items-center gap-2" :aria-label="t('v113Connect.wizard.progress')" data-connect-stepper :data-step="step">
    <template v-for="(k, i) in KEYS" :key="k">
      <li
        class="flex shrink-0 items-center gap-1.5 text-xs transition-colors duration-(--ccm-dur-base)"
        :class="step === i + 1 ? 'font-semibold text-ink' : step > i + 1 ? 'text-success' : 'text-ink-muted'"
        :aria-current="step === i + 1 ? 'step' : undefined"
        :data-connect-step-item="i + 1"
      >
        <span
          class="grid size-5 place-items-center rounded-full text-2xs font-semibold tabular-nums transition-colors duration-(--ccm-dur-base)"
          :class="step === i + 1 ? 'bg-accent text-on-accent' : step > i + 1 ? 'bg-st-done-soft text-success' : 'bg-fill text-ink-muted'"
        >
          <Check v-if="step > i + 1" :size="12" aria-hidden="true" />
          <template v-else>{{ i + 1 }}</template>
        </span>
        <span>{{ t(`v113Connect.wizard.step.${k}`) }}</span>
      </li>
      <li v-if="i < KEYS.length - 1" aria-hidden="true" class="h-px min-w-4 flex-1 transition-colors duration-(--ccm-dur-base)" :class="step > i + 1 ? 'bg-success' : 'bg-line'" />
    </template>
  </ol>
</template>
