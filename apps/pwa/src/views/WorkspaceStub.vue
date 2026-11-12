<script setup lang="ts">
import { SquareTerminal } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import PageBar from '../components/PageBar.vue'

defineProps<{ machine: string; pane?: string }>()
const { t } = useI18n()
</script>

<template>
  <section class="page" data-testid="workspace-view" :data-machine="machine" :data-pane="pane ?? ''">
    <PageBar :title="t('tabs.workspace')" />
    <div class="soon" data-testid="workspace-soon">
      <SquareTerminal :size="32" :stroke-width="1.8" aria-hidden="true" />
      <h2>{{ t('workspace.soonTitle') }}</h2>
      <p>{{ t('workspace.soonBody') }}</p>
    </div>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  padding: 0 16px 16px;
}

.soon {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 64px 16px;
  text-align: center;
  color: var(--ccm-ink-muted);
}

.soon h2 {
  margin: 8px 0 0;
  font-size: 1.125rem;
  color: var(--ccm-ink);
}

.soon p {
  margin: 0;
  max-width: 30ch;
}
</style>
