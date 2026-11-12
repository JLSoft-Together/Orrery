<script setup lang="ts">
import { Laptop } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MachineCard from '../components/MachineCard.vue'
import { useHubStore } from '../stores/hub'

const { t } = useI18n()
const hub = useHubStore()
const connected = computed(() => hub.machines.filter((m) => hub.linkOf(m.id).status === 'ready'))
const others = computed(() => hub.machines.filter((m) => hub.linkOf(m.id).status !== 'ready'))
</script>

<template>
  <section class="page" data-testid="machines-view">
    <h1 class="title">{{ t('machines.title') }}</h1>
    <div v-if="hub.machines.length === 0" class="empty card" data-testid="machines-empty">
      <span class="empty-icon" aria-hidden="true"><Laptop :size="28" :stroke-width="1.8" /></span>
      <h2>{{ t('machines.emptyTitle') }}</h2>
      <p>{{ t('machines.emptyBody') }}</p>
      <p class="hint">{{ t('machines.emptyHint') }}</p>
    </div>
    <template v-else>
      <h2 v-if="connected.length" class="label group">{{ t('machines.connected', { n: connected.length }) }}</h2>
      <MachineCard v-for="machine in connected" :key="machine.id" :machine="machine" :link="hub.linkOf(machine.id)" :view="hub.viewOf(machine.id)" :rtt="hub.rtt[machine.id]" @policy="hub.setPolicy(machine.id, $event)" @forget="hub.forget(machine.id)" />
      <h2 v-if="others.length" class="label group">{{ t('machines.others', { n: others.length }) }}</h2>
      <MachineCard v-for="machine in others" :key="machine.id" :machine="machine" :link="hub.linkOf(machine.id)" :view="hub.viewOf(machine.id)" :rtt="hub.rtt[machine.id]" @policy="hub.setPolicy(machine.id, $event)" @forget="hub.forget(machine.id)" />
    </template>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.title {
  margin: 8px 0 4px;
  font-size: 30px;
  font-weight: 800;
  letter-spacing: -0.01em;
}

.group {
  margin: 8px 0 0;
  font-weight: 500;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 32px 24px;
  text-align: center;
}

.empty h2 {
  margin: 0;
  font-size: 18px;
}

.empty p {
  margin: 0;
  color: var(--ccm-ink-muted);
}

.empty .hint {
  color: var(--ccm-accent-ink);
  font-weight: 600;
}

.empty-icon {
  display: grid;
  place-items: center;
  width: 56px;
  height: 56px;
  border-radius: var(--radius-card);
  background: var(--ccm-raised);
  color: var(--ccm-accent-ink);
}
</style>
