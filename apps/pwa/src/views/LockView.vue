<script setup lang="ts">
import { Fingerprint, Lock } from 'lucide-vue-next'
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import OrbitArt from '../components/OrbitArt.vue'
import { useLockStore } from '../stores/lock'
import { usePrefsStore } from '../stores/prefs'

const { t } = useI18n()
const lock = useLockStore()
const prefs = usePrefsStore()
const timeoutKey: Record<number, string> = { 60000: 'm1', 900000: 'm15', 3600000: 'h1', 14400000: 'h4' }
const after = computed(() => t(`settings.timeouts.${timeoutKey[prefs.local.lockTimeout] ?? 'h1'}`))

onMounted(() => void lock.unlock(true))
</script>

<template>
  <main class="lock" data-testid="lock-screen" :data-failed="lock.failure ? '1' : '0'">
    <OrbitArt />
    <h1 class="title">{{ t('lock.title') }}</h1>
    <p class="body">{{ t('lock.body') }}</p>
    <button type="button" class="touch" :disabled="lock.busy" :aria-label="t('lock.touch')" data-testid="lock-unlock" @click="lock.unlock()">
      <Fingerprint :size="40" :stroke-width="1.6" aria-hidden="true" />
    </button>
    <p class="touch-label">{{ t('lock.touch') }}</p>
    <p v-if="lock.failure" class="fail" role="alert" data-testid="lock-error">{{ t(lock.failure === 'cancelled' ? 'lock.cancelled' : 'lock.failed') }}</p>
    <p class="foot"><Lock :size="16" :stroke-width="2" aria-hidden="true" />{{ t('lock.auto', { after }) }}</p>
  </main>
</template>

<style scoped>
.lock {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 100%;
  padding: calc(24px + env(safe-area-inset-top)) 24px calc(24px + env(safe-area-inset-bottom));
  text-align: center;
  animation: fade 250ms var(--ease-out);
}

.title {
  margin: 8px 0 0;
  font-size: 1.375rem;
}

.body {
  margin: 0;
  max-width: 30ch;
  color: var(--ccm-ink-muted);
}

.touch {
  display: grid;
  place-items: center;
  width: 80px;
  height: 80px;
  margin-top: 24px;
  border-radius: 50%;
  border: 1px solid var(--ccm-accent);
  background: var(--ccm-selection);
  color: var(--ccm-accent-ink);
}

.touch-label {
  margin: 0;
  font-weight: 600;
}

.fail {
  margin: 0;
  color: var(--ccm-error);
  animation: shake 240ms linear;
}

.foot {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 32px 0 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

@keyframes fade {
  from {
    opacity: 0;
  }
}

@keyframes shake {
  25% {
    transform: translateX(-6px);
  }
  75% {
    transform: translateX(6px);
  }
}
</style>
