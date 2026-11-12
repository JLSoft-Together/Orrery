<script setup lang="ts">
import { Bug, Laptop } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

export type TabId = 'machines' | 'debug'

defineProps<{ active: TabId }>()
const emit = defineEmits<{ select: [tab: TabId] }>()
const { t } = useI18n()
const tabs = [
  { id: 'machines' as const, icon: Laptop },
  { id: 'debug' as const, icon: Bug },
]
</script>

<template>
  <nav class="tabs" aria-label="tabs">
    <button v-for="tab in tabs" :key="tab.id" type="button" class="tab" :class="{ on: active === tab.id }" :aria-current="active === tab.id ? 'page' : undefined" :data-testid="`tab-${tab.id}`" @click="emit('select', tab.id)">
      <component :is="tab.icon" :size="22" :stroke-width="2" aria-hidden="true" />
      <span>{{ t(`tabs.${tab.id}`) }}</span>
    </button>
  </nav>
</template>

<style scoped>
.tabs {
  display: flex;
  border-top: 1px solid var(--ccm-line);
  background: var(--ccm-canvas);
  padding-bottom: env(safe-area-inset-bottom);
}

.tab {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-height: 56px;
  padding: 8px 4px;
  border: 0;
  border-top: 2px solid transparent;
  background: transparent;
  color: var(--ccm-ink-muted);
  font-size: 12px;
  font-weight: 600;
}

.tab.on {
  color: var(--ccm-accent-ink);
  border-top-color: var(--ccm-accent);
}
</style>
