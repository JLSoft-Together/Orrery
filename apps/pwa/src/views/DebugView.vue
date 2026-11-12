<script setup lang="ts">
import { Trash2 } from 'lucide-vue-next'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import PageBar from '../components/PageBar.vue'
import { useHubStore } from '../stores/hub'
import { useUpdateStore } from '../stores/update'
import { DEFAULT_CONFIG } from '../remote/types'

const { t } = useI18n()
const hub = useHubStore()
const update = useUpdateStore()
const message = ref('')

async function wipe(): Promise<void> {
  await hub.wipeAll()
  message.value = t('debug.cleared')
}

function machineLine(id: string): string {
  const view = hub.viewOf(id)
  const tabs = Array.isArray(view.workspace?.tabs) ? view.workspace.tabs.length : 0
  const terminals = Array.isArray(view.snapshot?.terminals) ? view.snapshot.terminals.length : 0
  return t('debug.machineLine', { seq: view.seq, rev: view.wsRev, tabs, terminals, state: view.stale ? t('debug.staleLabel') : t('debug.freshLabel') })
}

function stamp(at: number): string {
  return new Date(at).toISOString().slice(11, 19)
}
</script>

<template>
  <section class="page" data-testid="debug-view">
    <PageBar :title="t('debug.title')" />

    <div class="card block">
      <h2 class="label">{{ t('debug.version') }}</h2>
      <dl class="kv">
        <dt>{{ t('debug.web') }}</dt>
        <dd data-testid="debug-web">{{ update.info?.web ?? '-' }}</dd>
        <dt>{{ t('debug.proto') }}</dt>
        <dd data-testid="debug-proto">{{ DEFAULT_CONFIG.proto }}</dd>
      </dl>
      <button type="button" class="btn" data-testid="debug-check" @click="update.check()">{{ t('debug.check') }}</button>
    </div>

    <div class="card block">
      <h2 class="label">{{ t('debug.machineData') }}</h2>
      <ul class="log" data-testid="debug-machines">
        <li v-for="machine in hub.machines" :key="machine.id" :data-testid="`debug-machine-${machine.id}`">
          <span>{{ machine.name }}</span>
          <span>{{ machineLine(machine.id) }}</span>
        </li>
      </ul>
    </div>

    <div class="card block">
      <h2 class="label">{{ t('debug.logTitle') }}</h2>
      <p v-if="hub.log.length === 0" class="muted">{{ t('debug.logEmpty') }}</p>
      <ol v-else class="log" data-testid="debug-log">
        <li v-for="(entry, index) in hub.log.slice().reverse().slice(0, 60)" :key="index">
          <time>{{ stamp(entry.at) }}</time>
          <span>{{ entry.text }}</span>
        </li>
      </ol>
    </div>

    <p v-if="message" class="message" role="status" data-testid="debug-message">{{ message }}</p>
    <div class="card block">
      <h2 class="label">{{ t('debug.dataTitle') }}</h2>
      <button type="button" class="btn btn-danger" data-testid="debug-wipe" @click="wipe()">
        <Trash2 :size="18" :stroke-width="2" aria-hidden="true" />
        {{ t('debug.clear') }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 16px;
}

.block {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.block h2 {
  margin: 0;
  font-weight: 500;
}

.kv {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 16px;
  margin: 0;
}

.kv dt {
  color: var(--ccm-ink-muted);
}

.kv dd {
  margin: 0;
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}

.message,
.muted {
  margin: 0;
  color: var(--ccm-ink-muted);
}

.log {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
  font-family: var(--font-mono);
  font-size: 12px;
}

.log li {
  display: flex;
  gap: 10px;
}

.log time {
  color: var(--ccm-ink-faint);
  flex: none;
}

.log span {
  overflow-wrap: anywhere;
}
</style>
