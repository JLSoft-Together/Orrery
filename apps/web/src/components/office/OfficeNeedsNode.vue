<script setup lang="ts">
import { TriangleAlert } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

defineProps<{ inline?: boolean }>()
const { t } = useI18n()
</script>

<template>
  <p role="status" class="flex items-center gap-2 text-sm text-warning" :class="inline ? 'px-3 py-1' : 'h-full justify-center p-6 text-center'" data-office-needs-node>
    <TriangleAlert class="size-4 shrink-0" aria-hidden="true" />
    <span>{{ t('settings.officeNeedsNode') }}</span>
  </p>
</template>
