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
  <nav class="tabs glass-bar" :aria-label="t('tabs.label')" data-testid="tab-bar">
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
  position: absolute;
  left: 16px;
  right: 16px;
  bottom: calc(var(--ccm-tabbar-gap) + var(--ccm-safe-bottom));
  z-index: 20;
  height: var(--ccm-tabbar-h);
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2px;
  padding: 6px;
  border: 1px solid var(--ccm-glass-edge);
  border-radius: var(--radius-pill);
  box-shadow: var(--ccm-shadow-pop), inset 0 1px 0 var(--ccm-glass-edge);
}

.tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  min-height: 44px;
  padding: 0 4px;
  border: 0;
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--ccm-ink-muted);
  font-size: 0.6875rem;
  font-weight: 700;
  transition:
    color var(--ccm-dur-base) var(--ccm-ease-standard),
    background-color var(--ccm-dur-base) var(--ccm-ease-standard),
    transform var(--ccm-dur-press) var(--ccm-ease-standard);
}

.tab:active {
  transform: scale(var(--ccm-motion-scale, 0.98));
}

.tab.on {
  background: var(--ccm-accent-soft);
  color: var(--ccm-accent-ink);
}

.icon {
  position: relative;
  display: grid;
}

.badge {
  position: absolute;
  top: -6px;
  right: -14px;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--ccm-waiting);
  color: var(--ccm-material);
  font-size: 0.6875rem;
  font-weight: 800;
  line-height: 18px;
  text-align: center;
}
</style>
