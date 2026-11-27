<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, Clock, Cloud, LoaderCircle, RotateCcw, ShieldCheck, Wifi } from 'lucide-vue-next'
import { clock } from '../../lib/remote'
import ConnectLink from './ConnectLink.vue'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const w = props.wizard

const name = computed(() => w.name.value)
const showQr = computed(() => name.value === 'pairWait' || name.value === 'pairExpired')
const label = computed(() => w.paired.value?.alias ?? w.paired.value?.name ?? t('v113Connect.done.fallback'))
const failed = computed(() => name.value === 'pairWait' && w.pairQr.value.error)
</script>

<template>
  <div v-if="showQr" class="flex flex-col items-center gap-3 px-3 py-3 sm:flex-row sm:items-start sm:gap-5" data-remote-pair data-connect-pair :data-connect-phase="name">
    <div class="grid size-44 shrink-0 place-items-center overflow-hidden rounded-inner p-1 [&>svg]:block [&>svg]:size-full" :class="w.pairQr.value.svg ? 'bg-page' : 'bg-fill'" data-remote-qr-box>
      <Transition name="ccm-fade" mode="out-in">
        <div v-if="w.pairQr.value.svg" key="qr" class="size-full transition-opacity duration-(--ccm-dur-base)" :class="name === 'pairWait' ? '' : 'opacity-25'" role="img" :aria-label="t('v113Connect.pair.label')" data-remote-qr data-share-hide v-html="w.pairQr.value.svg" />
        <CircleAlert v-else-if="failed" key="err" :size="22" class="text-warning" aria-hidden="true" />
        <LoaderCircle v-else key="wait" :size="22" class="animate-spin text-ink-muted" aria-hidden="true" />
      </Transition>
    </div>

    <div class="flex min-w-0 flex-1 flex-col gap-2 self-stretch">
      <template v-if="name === 'pairWait'">
        <p class="text-sm font-semibold">{{ t('v113Connect.pair.title') }}</p>
        <p v-if="failed" class="flex items-start gap-1.5 text-xs text-warning" role="alert" data-connect-pair-failed>
          <CircleAlert :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
          {{ t('v113Connect.pair.failed') }}
        </p>
        <template v-else>
          <div class="flex flex-wrap items-center gap-2 text-xs text-ink-muted">
            <span class="inline-flex items-center gap-1 font-mono tabular-nums text-ink" role="timer" :aria-label="t('v113Connect.pair.left', { time: clock(w.pairLeft.value) })" data-remote-countdown>
              <Clock :size="13" aria-hidden="true" />
              {{ clock(w.pairLeft.value) }}
            </span>
            <span v-if="w.pairQr.value.svg" class="inline-flex items-center gap-1" data-remote-pair-route :data-relay="w.pairQr.value.relay">
              <component :is="w.pairQr.value.relay ? Cloud : Wifi" :size="13" aria-hidden="true" />
              {{ w.pairQr.value.relay ? t('v113Connect.pair.viaRelay') : t('v113Connect.pair.lanOnly') }}
            </span>
          </div>
          <p class="text-xs text-ink-muted">{{ t('v113Connect.pair.once') }}</p>
        </template>
      </template>

      <template v-else>
        <p class="inline-flex items-center gap-1.5 self-start text-sm font-semibold text-warning" role="status" data-connect-pair-expired>
          <Clock :size="15" aria-hidden="true" />
          {{ t('v113Connect.pair.expired') }}
        </p>
        <p v-if="wizard.state.value.pairHint === 'sameWifi'" class="text-xs text-ink-muted" data-connect-pair-hint>{{ t('v113Connect.pair.sameWifi') }}</p>
      </template>

      <div class="mt-auto flex flex-wrap justify-end gap-2 pt-1">
        <button v-if="name === 'pairExpired' || failed" type="button" class="ccm-btn ccm-btn-primary" data-remote-pair-retry data-connect-pair-renew @click="name === 'pairExpired' ? wizard.dispatch({ type: 'pairRenew' }) : wizard.retryPair()">
          <RotateCcw :size="14" aria-hidden="true" />
          {{ t('v113Connect.pair.renew') }}
        </button>
      </div>
    </div>
  </div>

  <div v-else-if="name === 'pairConfirm'" class="flex items-start gap-3 px-3 py-4" role="status" data-connect-pair-confirm>
    <ShieldCheck :size="18" class="mt-0.5 shrink-0 text-accent-ink" aria-hidden="true" />
    <div class="min-w-0 flex-1">
      <p class="text-sm font-semibold">{{ t('v113Connect.pair.confirmTitle') }}</p>
      <p class="mt-0.5 text-xs text-ink-muted">{{ t('v113Connect.pair.confirmHint') }}</p>
    </div>
    <LoaderCircle :size="16" class="mt-0.5 shrink-0 animate-spin text-ink-muted" aria-hidden="true" />
  </div>

  <div v-else-if="name === 'denied'" class="flex flex-col gap-3 px-3 py-3" role="alert" data-connect-denied>
    <div class="flex items-start gap-2.5">
      <CircleAlert :size="16" class="mt-0.5 shrink-0 text-st-error" aria-hidden="true" />
      <div class="min-w-0">
        <p class="text-sm font-semibold">{{ t('v113Connect.pair.denied') }}</p>
        <p class="mt-0.5 text-xs text-ink-muted">{{ t('v113Connect.pair.deniedHint') }}</p>
      </div>
    </div>
    <div class="flex justify-end">
      <button type="button" class="ccm-btn ccm-btn-primary" data-connect-denied-renew @click="wizard.dispatch({ type: 'pairRenew' })">
        <RotateCcw :size="14" aria-hidden="true" />
        {{ t('v113Connect.pair.renew') }}
      </button>
    </div>
  </div>

  <div v-else class="flex flex-col items-center gap-2 px-3 py-5 text-center" role="status" data-connect-paired>
    <ConnectLink :linked="true" />
    <p class="mt-1 text-sm font-semibold">{{ t('v113Connect.pair.paired', { name: label }) }}</p>
    <p class="text-xs text-ink-muted">{{ t('v113Connect.pair.pairedHint') }}</p>
  </div>
</template>
