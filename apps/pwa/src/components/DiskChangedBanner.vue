<script setup lang="ts">
import { FileWarning } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

defineProps<{ busy: boolean }>()
const emit = defineEmits<{ reload: []; keep: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="banner" role="alert" data-testid="edit-disk-changed">
    <div class="head">
      <FileWarning :size="20" :stroke-width="2" aria-hidden="true" />
      <div class="text">
        <strong>{{ t('edit.diskChanged') }}</strong>
        <span>{{ t('edit.diskChangedBody') }}</span>
      </div>
    </div>
    <div class="acts">
      <button type="button" class="btn" :disabled="busy" data-testid="edit-disk-keep" @click="emit('keep')">{{ t('edit.keepMine') }}</button>
      <button type="button" class="btn btn-danger" :disabled="busy" data-testid="edit-disk-reload" @click="emit('reload')">{{ t('edit.reload') }}</button>
    </div>
  </div>
</template>

<style scoped>
.banner {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0 16px 8px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-warning);
  background: color-mix(in srgb, var(--ccm-warning) 10%, var(--ccm-surface));
  animation: banner-in 240ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
}

.head {
  display: flex;
  gap: 10px;
  color: var(--ccm-warning);
}

.text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.text strong {
  color: var(--ccm-ink);
  font-size: 0.9375rem;
}

.acts {
  display: flex;
  gap: 8px;
}

.acts .btn {
  flex: 1;
}

@keyframes banner-in {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
