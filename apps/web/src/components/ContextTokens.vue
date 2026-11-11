<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { TriangleAlert } from 'lucide-vue-next'
import { contextOverWarn } from '../lib/contextWarn'
import { compactNumber, fullNumber } from '../lib/format'
import { useSettingsStore } from '../stores/settings'

const props = defineProps<{ tokens: number | undefined }>()

const { t, locale } = useI18n()
const settings = useSettingsStore()

const shown = computed(() => props.tokens !== undefined && Number.isFinite(props.tokens) && props.tokens > 0)
const over = computed(() => contextOverWarn(props.tokens, settings.contextWarnTokens))
const label = computed(() => compactNumber(props.tokens, locale.value))
const title = computed(() => {
  const n = fullNumber(props.tokens, locale.value)
  return over.value ? t('context.tokensWarn', { n, limit: compactNumber(settings.contextWarnTokens, locale.value) }) : t('context.tokens', { n })
})
</script>

<template>
  <span
    v-if="shown"
    class="inline-flex shrink-0 items-center gap-1 text-xs tabular-nums"
    :class="over ? 'text-warning font-medium' : 'text-ink-faint'"
    :title="title"
    :aria-label="title"
    :data-context-warn="over ? '' : undefined"
  >
    <TriangleAlert v-if="over" :size="12" aria-hidden="true" class="shrink-0" />
    <span>{{ label }}</span>
  </span>
</template>
