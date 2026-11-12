<script setup lang="ts">
import { Activity, LayoutGrid, SlidersHorizontal } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

export type TabId = 'monitor' | 'workspace' | 'settings'

defineProps<{ active: TabId | null; badge?: number }>()
const emit = defineEmits<{ select: [tab: TabId] }>()
const { t } = useI18n()
const tabs = [
  { id: 'monitor' as const, icon: Activity },
  { id: 'workspace' as const, icon: LayoutGrid },
  { id: 'settings' as const, icon: SlidersHorizontal },
]
</script>

<template>
  <nav class="tabs" :aria-label="t('tabs.label')" data-testid="tab-bar">
    <button v-for="tab in tabs" :key="tab.id" type="button" class="tab" :class="{ on: active === tab.id }" :aria-current="active === tab.id ? 'page' : undefined" :data-testid="`tab-${tab.id}`" @click="emit('select', tab.id)">
      <span class="icon">
        <component :is="tab.icon" :size="22" :stroke-width="2" aria-hidden="true" />
        <span v-if="tab.id === 'monitor' && badge" class="badge" data-testid="tab-badge">{{ badge }}</span>
      </span>
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
  font-size: 0.75rem;
  font-weight: 600;
  transition: color 180ms var(--ease-out);
}

.tab.on {
  color: var(--ccm-accent-ink);
  border-top-color: var(--ccm-accent);
}

.icon {
  position: relative;
  display: grid;
}

.badge {
  position: absolute;
  top: -6px;
  right: -12px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  background: var(--ccm-waiting);
  color: var(--ccm-canvas);
  font-size: 0.6875rem;
  line-height: 16px;
  text-align: center;
}
</style>
