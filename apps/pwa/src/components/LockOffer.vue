<script setup lang="ts">
import { CircleCheck, Fingerprint, ScanFace, ShieldCheck, TriangleAlert } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { lockKind } from '../lib/lockOffer'
import { useLockStore } from '../stores/lock'

defineProps<{ name?: string }>()
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
    <section class="card panel">
      <div class="dashes" aria-hidden="true"><span /><span /><span /></div>
      <p v-if="name" class="paired" data-testid="lock-offer-paired"><CircleCheck :size="20" :stroke-width="2.2" aria-hidden="true" />{{ t('lockOffer.paired', { name }) }}</p>
      <div class="hero" aria-hidden="true">
        <ScanFace v-if="kind === 'faceId'" :size="48" :stroke-width="1.8" />
        <Fingerprint v-else :size="48" :stroke-width="1.8" />
      </div>
      <h2 class="title">{{ t(`lockOffer.title.${kind}`) }}</h2>
      <p class="body">{{ t('lockOffer.body') }}</p>
      <div class="note"><ShieldCheck :size="18" :stroke-width="2" aria-hidden="true" /><span>{{ t('lockOffer.note') }}</span></div>
      <p v-if="failure" class="fail" role="status" data-testid="lock-offer-fail"><TriangleAlert :size="18" :stroke-width="2" aria-hidden="true" />{{ t(`lockOffer.fail.${failure}`) }}</p>
    </section>
    <div class="spacer" />
    <button type="button" class="btn btn-primary wide" :disabled="lock.busy" data-testid="lock-offer-enable" @click="enable">
      <ScanFace v-if="kind === 'faceId'" :size="20" :stroke-width="2" aria-hidden="true" />
      <Fingerprint v-else :size="20" :stroke-width="2" aria-hidden="true" />
      {{ t(`lockOffer.enable.${kind}`) }}
    </button>
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
  animation: offer-in var(--ccm-dur-slow, 320ms) var(--ccm-ease-enter, ease-out) both;
}

.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 16px 20px 20px;
}

.dashes {
  display: flex;
  gap: 8px;
}

.dashes span {
  width: 24px;
  height: 4px;
  border-radius: 999px;
  background: var(--ccm-accent);
}

.paired {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  padding: 8px 16px;
  border-radius: var(--radius-chip);
  background: color-mix(in srgb, var(--ccm-done) 16%, transparent);
  color: var(--ccm-done);
  font-weight: 700;
}

.hero {
  display: grid;
  place-items: center;
  width: 96px;
  height: 96px;
  border-radius: 28px;
  background: var(--ccm-accent-soft);
  color: var(--ccm-accent-ink);
}

.title {
  margin: 4px 0 0;
  font-size: 1.625rem;
  font-weight: 800;
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
  color: var(--ccm-accent-ink);
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
