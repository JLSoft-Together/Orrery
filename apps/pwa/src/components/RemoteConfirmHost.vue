<script setup lang="ts">
import { REMOTE_CONFIRM_MS } from '../lib/limits'
import { ShieldQuestion } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useNow } from '../lib/useNow'
import { useHubStore } from '../stores/hub'

const { t } = useI18n()
const hub = useHubStore()
const now = useNow(1000)
const busy = ref(false)

const pending = computed(() => {
  for (const machine of hub.machines) {
    const ask = hub.viewOf(machine.id).confirms[0]
    if (ask) return { machine: machine.id, name: machine.alias ?? machine.name, ask }
  }
  return null
})
const left = computed(() => (pending.value ? Math.min(REMOTE_CONFIRM_MS / 1000, Math.max(0, Math.ceil((pending.value.ask.receivedAt + REMOTE_CONFIRM_MS - now.value) / 1000))) : 0))

async function answer(ok: boolean): Promise<void> {
  const current = pending.value
  if (!current || busy.value) return
  busy.value = true
  try {
    await hub.answerConfirm(current.machine, current.ask.cid, ok)
  } catch {
    return
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div v-if="pending" class="scrim" data-testid="remote-confirm" :data-cid="pending.ask.cid">
    <div class="sheet" role="alertdialog" aria-modal="true" aria-labelledby="rc-title">
      <p class="from"><ShieldQuestion :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('confirm.from', { name: pending.name }) }}</p>
      <h2 id="rc-title" class="title" data-testid="remote-confirm-title">{{ pending.ask.title }}</h2>
      <p v-if="pending.ask.detail" class="detail mono" data-testid="remote-confirm-detail">{{ pending.ask.detail }}</p>
      <p class="timer" data-testid="remote-confirm-timer">{{ t('confirm.expires', { n: left }) }}</p>
      <div class="buttons">
        <button type="button" class="btn" :disabled="busy" data-testid="remote-confirm-cancel" @click="answer(false)">{{ pending.ask.cancel }}</button>
        <button type="button" class="btn btn-danger" :disabled="busy" data-testid="remote-confirm-ok" @click="answer(true)">{{ pending.ask.ok }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  align-items: end;
  justify-items: center;
  padding: 16px 16px calc(16px + env(safe-area-inset-bottom));
  background: rgba(0, 0, 0, 0.5);
  animation: fade 160ms var(--ease-out);
}

.sheet {
  width: min(100%, 440px);
  padding: 20px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
  animation: rise 260ms var(--ease-out);
}

.from {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.title {
  margin: 0 0 8px;
  font-size: 1.0625rem;
}

.detail {
  margin: 0 0 8px;
  padding: 8px 10px;
  border-radius: var(--radius-control);
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-size: 0.8125rem;
  overflow-wrap: anywhere;
}

.timer {
  margin: 0 0 16px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-faint);
}

.buttons {
  display: flex;
  gap: 8px;
}

.buttons .btn {
  flex: 1;
}

@keyframes fade {
  from {
    opacity: 0;
  }
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(24px);
  }
}
</style>
