<script setup lang="ts">
import { RefreshCw, TriangleAlert } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useUpdateStore } from '../stores/update'

const { t } = useI18n()
const update = useUpdateStore()
const visible = computed(() => update.state !== 'current' || update.workerWaiting)
const outdated = computed(() => update.state === 'protoOutdated')
</script>

<template>
  <div v-if="visible" class="bar" :class="{ outdated }" role="status" data-testid="update-bar">
    <component :is="outdated ? TriangleAlert : RefreshCw" :size="18" :stroke-width="2.2" aria-hidden="true" />
    <span class="text">{{ outdated ? t('update.protoOutdated') : t('update.available') }}</span>
    <button type="button" class="reload" data-testid="update-reload" @click="update.reload()">{{ t('update.reload') }}</button>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--ccm-accent);
  color: var(--ccm-on-accent);
  font-weight: 600;
}

.bar.outdated {
  background: var(--ccm-error);
  color: var(--ccm-on-accent);
}

.text {
  flex: 1;
}

.reload {
  min-height: 36px;
  padding: 0 14px;
  border-radius: var(--radius-control);
  border: 1px solid currentColor;
  background: transparent;
  color: inherit;
  font-weight: 700;
}
</style>
