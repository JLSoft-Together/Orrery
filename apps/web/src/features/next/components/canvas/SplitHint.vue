<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Info } from 'lucide-vue-next'

defineProps<{ total: number; count: number }>()
const emit = defineEmits<{ split: []; skip: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="mist-split-hint" role="status" data-split-hint>
    <Info :size="14" class="shrink-0" aria-hidden="true" />
    <b class="shrink-0">{{ t('mist.drag.hint.title', { n: total }) }}</b>
    <span class="min-w-0 truncate text-ink-muted">{{ t('mist.drag.hint.body', { n: count }, count) }}</span>
    <span class="flex-1" />
    <button type="button" class="mist-hint-btn mist-hint-pri" data-split-hint-go @click="emit('split')">{{ t('mist.drag.hint.split', { n: count }, count) }}</button>
    <button type="button" class="mist-hint-btn" data-split-hint-skip @click="emit('skip')">{{ t('mist.drag.hint.skip') }}</button>
  </div>
</template>
