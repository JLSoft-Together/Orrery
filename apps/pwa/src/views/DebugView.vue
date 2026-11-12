<script setup lang="ts">
import { Trash2 } from 'lucide-vue-next'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useHubStore } from '../stores/hub'
import { useUpdateStore } from '../stores/update'
import { DEFAULT_CONFIG } from '../remote/types'

const { t } = useI18n()
const hub = useHubStore()
const update = useUpdateStore()
const qr = ref('')
const token = ref('')
const deviceName = ref('Orrery web')
const busy = ref(false)
const message = ref('')

async function pair(): Promise<void> {
  busy.value = true
  message.value = t('debug.pairing')
  try {
    const machine = await hub.pair(qr.value.trim(), token.value.trim() || undefined, deviceName.value.trim() || 'Orrery web')
    message.value = t('debug.pairOk', { name: machine.name })
    qr.value = ''
    token.value = ''
  } catch (error) {
    console.error('pair failed', error instanceof Error ? error.stack : error)
    const code = error instanceof Error ? error.message : String(error)
    message.value = t('debug.pairFail', { code })
  } finally {
    busy.value = false
  }
}

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
    <h1 class="title">{{ t('debug.title') }}</h1>

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
      <h2 class="label">{{ t('debug.pairTitle') }}</h2>
      <label class="field">
        <span>{{ t('debug.pairQr') }}</span>
        <textarea v-model="qr" rows="3" spellcheck="false" autocapitalize="off" autocomplete="off" data-testid="pair-qr"></textarea>
      </label>
      <label class="field">
        <span>{{ t('debug.pairToken') }}</span>
        <input v-model="token" type="text" spellcheck="false" autocapitalize="off" autocomplete="off" data-testid="pair-token" />
      </label>
      <label class="field">
        <span>{{ t('debug.pairName') }}</span>
        <input v-model="deviceName" type="text" data-testid="pair-name" />
      </label>
      <button type="button" class="btn btn-primary" :disabled="busy || !qr.trim()" data-testid="pair-go" @click="pair()">{{ t('debug.pairGo') }}</button>
      <p v-if="message" class="message" role="status" data-testid="debug-message">{{ message }}</p>
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
  padding: 16px;
}

.title {
  margin: 8px 0 4px;
  font-size: 30px;
  font-weight: 800;
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

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
  color: var(--ccm-ink-muted);
}

textarea,
input {
  width: 100%;
  min-height: 44px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-family: var(--font-mono);
  font-size: 14px;
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
