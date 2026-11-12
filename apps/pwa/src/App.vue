<script setup lang="ts">
import { TriangleAlert, X } from 'lucide-vue-next'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import TabBar, { type TabId } from './components/TabBar.vue'
import UpdateBar from './components/UpdateBar.vue'
import { useHubStore } from './stores/hub'
import DebugView from './views/DebugView.vue'
import MachinesView from './views/MachinesView.vue'

const { t } = useI18n()
const hub = useHubStore()
const tab = ref<TabId>('machines')
</script>

<template>
  <div class="shell" :data-ready="hub.ready ? '1' : '0'">
    <UpdateBar />
    <div class="notices">
      <div v-if="hub.bootError" class="notice" role="alert" data-testid="boot-error">
        <TriangleAlert :size="18" :stroke-width="2.2" aria-hidden="true" />
        <span class="notice-text">{{ t('boot.error') }}</span>
      </div>
      <div v-for="notice in hub.notices" :key="notice.id" class="notice" role="alert" data-testid="notice">
        <TriangleAlert :size="18" :stroke-width="2.2" aria-hidden="true" />
        <span class="notice-text">{{ t('notice.revoked', { name: notice.name }) }}</span>
        <button type="button" class="notice-close" :aria-label="t('notice.dismiss')" @click="hub.dismiss(notice.id)"><X :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
      </div>
    </div>
    <div class="body">
      <div class="master">
        <main class="scroll">
          <MachinesView v-if="tab === 'machines'" />
          <DebugView v-else />
        </main>
        <TabBar :active="tab" @select="tab = $event" />
      </div>
      <aside class="detail" data-testid="detail-pane" aria-hidden="true">
        <p>{{ t('machines.detailEmpty') }}</p>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding-top: env(safe-area-inset-top);
  padding-left: env(safe-area-inset-left);
  padding-right: env(safe-area-inset-right);
}

.notices {
  display: flex;
  flex-direction: column;
}

.notice {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--ccm-surface);
  border-bottom: 1px solid var(--ccm-line);
  color: var(--ccm-error);
  font-weight: 600;
}

.notice-text {
  flex: 1;
  color: var(--ccm-ink);
}

.notice-close {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.master {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.detail {
  display: none;
}

@media (min-width: 768px) {
  .master {
    flex: 0 0 380px;
    border-right: 1px solid var(--ccm-line);
  }

  .detail {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--ccm-ink-faint);
    padding: 24px;
    text-align: center;
  }
}
</style>
