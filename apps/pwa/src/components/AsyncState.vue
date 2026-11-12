<script setup lang="ts">
import { FileQuestion, LoaderCircle, RefreshCw, TriangleAlert } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

defineProps<{ blank: boolean; error: string | null; empty?: boolean; emptyText?: string }>()
const emit = defineEmits<{ retry: [] }>()
const { t } = useI18n()
</script>

<template>
  <div v-if="error" class="state" role="alert" data-testid="async-error">
    <TriangleAlert :size="28" :stroke-width="1.8" aria-hidden="true" />
    <p>{{ t(`err.${error}`) }}</p>
    <button type="button" class="btn" data-testid="async-retry" @click="emit('retry')"><RefreshCw :size="18" :stroke-width="2" aria-hidden="true" />{{ t('common.retry') }}</button>
  </div>
  <div v-else-if="blank" class="state" data-testid="async-loading">
    <LoaderCircle :size="28" :stroke-width="1.8" class="spin" aria-hidden="true" />
    <p>{{ t('common.loading') }}</p>
  </div>
  <div v-else-if="empty" class="state" data-testid="async-empty">
    <FileQuestion :size="28" :stroke-width="1.8" aria-hidden="true" />
    <p>{{ emptyText }}</p>
  </div>
  <slot v-else />
</template>

<style scoped>
.state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 48px 24px;
  color: var(--ccm-ink-muted);
  text-align: center;
}

.state p {
  margin: 0;
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
