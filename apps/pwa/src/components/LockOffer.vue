<script setup lang="ts">
import { Fingerprint, ScanFace, ShieldCheck, TriangleAlert } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { lockKind } from '../lib/lockOffer'
import { useLockStore } from '../stores/lock'

const emit = defineEmits<{ done: [] }>()
const { t } = useI18n()
const lock = useLockStore()
const failure = ref<'cancelled' | 'failed' | null>(null)

const kind = computed(() => (typeof navigator === 'undefined' ? 'screen' : lockKind(navigator.userAgent, navigator.maxTouchPoints)))

async function enable(): Promise<void> {
  if (lock.busy) return
  failure.value = null
  const outcome = await lock.enable()
  if (outcome === 'ok') {
    emit('done')
    return
  }
  failure.value = outcome === 'cancelled' ? 'cancelled' : 'failed'
}
</script>

<template>
  <div class="offer" data-testid="lock-offer" :data-kind="kind">
    <div class="hero" aria-hidden="true">
      <ScanFace v-if="kind === 'faceId'" :size="40" :stroke-width="1.8" />
      <Fingerprint v-else :size="40" :stroke-width="1.8" />
    </div>
    <h2 class="title">{{ t(`lockOffer.title.${kind}`) }}</h2>
    <p class="body">{{ t('lockOffer.body') }}</p>
    <div class="note"><ShieldCheck :size="18" :stroke-width="2" aria-hidden="true" /><span>{{ t('lockOffer.note') }}</span></div>
    <p v-if="failure" class="fail" role="status" data-testid="lock-offer-fail"><TriangleAlert :size="18" :stroke-width="2" aria-hidden="true" />{{ t(`lockOffer.fail.${failure}`) }}</p>
    <div class="spacer" />
    <button type="button" class="btn btn-primary wide" :disabled="lock.busy" data-testid="lock-offer-enable" @click="enable">{{ t(`lockOffer.enable.${kind}`) }}</button>
    <button type="button" class="btn wide ghost" :disabled="lock.busy" data-testid="lock-offer-later" @click="emit('done')">{{ t('lockOffer.later') }}</button>
  </div>
</template>

<style scoped>
.offer {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
  animation: offer-in 320ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
}

.hero {
  display: grid;
  place-items: center;
  width: 80px;
  height: 80px;
  margin-top: 24px;
  border-radius: 24px;
  background: color-mix(in srgb, var(--ccm-accent) 14%, var(--ccm-surface));
  color: var(--ccm-accent);
}

.title {
  margin: 4px 0 0;
  font-size: 1.375rem;
}

.body {
  margin: 0;
  max-width: 30ch;
  color: var(--ccm-ink-muted);
}

.note,
.fail {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0;
  max-width: 34ch;
  font-size: 0.8125rem;
  text-align: left;
  color: var(--ccm-ink-muted);
  animation: offer-in 240ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
}

.fail {
  color: var(--ccm-error);
  font-weight: 600;
}

.spacer {
  flex: 1;
}

.wide {
  width: 100%;
}

.ghost {
  border-color: transparent;
  background: transparent;
  color: var(--ccm-ink-muted);
}

@keyframes offer-in {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
