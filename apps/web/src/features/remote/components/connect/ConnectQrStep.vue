<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, Clock, LoaderCircle, RotateCcw, Wifi } from 'lucide-vue-next'
import { clock } from '../../lib/remote'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const w = props.wizard

const name = computed(() => w.name.value)
const dim = computed(() => name.value !== 'qrInstall')
const error = computed(() => w.qr.value.error)
const errorKey = computed(() => (error.value === 'address' ? 'noAddress' : 'failed'))
</script>

<template>
  <div class="flex flex-col items-center gap-3 px-3 py-3 sm:flex-row sm:items-start sm:gap-5" data-connect-qr :data-connect-phase="name">
    <div class="grid size-44 shrink-0 place-items-center overflow-hidden rounded-inner p-1 [&>svg]:block [&>svg]:size-full" :class="w.qr.value.svg ? 'bg-page' : 'bg-fill'">
      <Transition name="ccm-fade" mode="out-in">
        <div v-if="w.qr.value.svg" key="qr" class="size-full transition-opacity duration-(--ccm-dur-base)" :class="dim ? 'opacity-25' : ''" role="img" :aria-label="t('v113Connect.qr.label')" data-connect-qr-code data-share-hide v-html="w.qr.value.svg" />
        <CircleAlert v-else-if="error" key="err" :size="22" class="text-warning" aria-hidden="true" />
        <LoaderCircle v-else key="wait" :size="22" class="animate-spin text-ink-muted" aria-hidden="true" />
      </Transition>
    </div>

    <div class="flex min-w-0 flex-1 flex-col gap-2 self-stretch">
      <template v-if="error">
        <p class="flex items-start gap-1.5 text-sm" role="alert" data-connect-qr-error :data-error="error">
          <CircleAlert :size="15" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
          <span class="min-w-0">{{ t(`v113Connect.qr.${errorKey}`) }}</span>
        </p>
      </template>

      <template v-else-if="name === 'qrInstall'">
        <p class="text-sm font-semibold">{{ t('v113Connect.qr.title') }}</p>
        <p class="text-xs text-ink-muted">{{ t('v113Connect.qr.desc') }}</p>
        <div class="flex flex-wrap items-center gap-2 text-xs text-ink-muted">
          <span class="inline-flex items-center gap-1 font-mono tabular-nums text-ink" role="timer" :aria-label="t('v113Connect.qr.left', { time: clock(w.qrLeft.value) })" data-connect-qr-left>
            <Clock :size="13" aria-hidden="true" />
            {{ clock(w.qrLeft.value) }}
          </span>
          <span v-if="w.qr.value.ip" class="inline-flex items-center gap-1"><Wifi :size="13" aria-hidden="true" />{{ t('v113Connect.qr.wifi', { ip: w.qr.value.ip }) }}</span>
        </div>
        <p class="font-mono text-2xs break-all text-ink-muted" data-connect-qr-url>{{ w.qr.value.url }}</p>
      </template>

      <template v-else-if="name === 'qrInstalled'">
        <p class="inline-flex items-center gap-1.5 self-start text-sm font-semibold text-success" role="status" data-connect-qr-fetched>
          <CircleCheck :size="15" aria-hidden="true" />
          {{ t('v113Connect.qr.fetched') }}
        </p>
        <p class="text-xs text-ink-muted">{{ t('v113Connect.qr.fetchedHint') }}</p>
      </template>

      <template v-else>
        <p class="inline-flex items-center gap-1.5 self-start text-sm font-semibold text-warning" role="status" data-connect-qr-expired>
          <Clock :size="15" aria-hidden="true" />
          {{ t('v113Connect.qr.expired') }}
        </p>
        <p class="text-xs text-ink-muted">{{ t('v113Connect.qr.expiredHint') }}</p>
      </template>

      <div class="mt-auto flex flex-wrap justify-end gap-2 pt-1">
        <button v-if="name === 'qrInstall' && !error" type="button" class="ccm-btn" data-connect-qr-hide @click="wizard.dispatch({ type: 'back' })">{{ t('v113Connect.qr.hide') }}</button>
        <button v-if="name === 'qrExpired' || error" type="button" class="ccm-btn ccm-btn-primary" data-connect-qr-renew @click="wizard.retryQr()">
          <RotateCcw :size="14" aria-hidden="true" />
          {{ t('v113Connect.qr.renew') }}
        </button>
      </div>
    </div>
  </div>
</template>
