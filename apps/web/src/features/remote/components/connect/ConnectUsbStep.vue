<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, CircleAlert, Link2, LoaderCircle, QrCode, RotateCcw } from 'lucide-vue-next'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const w = props.wizard

type Phase = 'done' | 'active' | 'todo'
const ROWS = ['apk', 'install', 'pair'] as const
const ORDER: Record<string, number> = { install: 1, reverse: 2, pair: 2, open: 2, waiting: 3 }

const phases = computed<Record<(typeof ROWS)[number], Phase>>(() => {
  const at = w.stage.value ? (ORDER[w.stage.value] ?? 0) : 0
  const of = (idx: number): Phase => (at > idx ? 'done' : at === idx ? 'active' : 'todo')
  return { apk: of(0), install: of(1), pair: of(2) }
})

const ICON: Record<Phase, { icon: Component; cls: string; spin?: boolean }> = {
  done: { icon: Check, cls: 'text-success' },
  active: { icon: LoaderCircle, cls: 'text-accent-ink', spin: true },
  todo: { icon: Link2, cls: 'text-ink-faint' },
}

const error = computed(() => w.state.value.usbError ?? 'other')
</script>

<template>
  <div v-if="wizard.name.value === 'usbInstalling'" class="flex flex-col gap-3 px-3 py-3" data-connect-usb-run :data-stage="wizard.stage.value ?? 'start'">
    <p class="text-sm font-semibold">{{ t('v113Connect.usb.title', { name: wizard.usbName.value }) }}</p>
    <ul class="flex flex-col gap-2" role="status">
      <li v-for="row in ROWS" :key="row" class="flex items-center gap-2.5 text-sm" :class="phases[row] === 'todo' ? 'text-ink-muted' : ''" :data-phase="phases[row]">
        <component :is="ICON[phases[row]].icon" :size="15" class="shrink-0" :class="[ICON[phases[row]].cls, ICON[phases[row]].spin ? 'animate-spin' : '']" aria-hidden="true" />
        <span class="min-w-0">{{ t(`v113Connect.usb.${row}`) }}</span>
      </li>
    </ul>
    <div class="flex justify-end">
      <button type="button" class="ccm-btn" data-connect-usb-cancel @click="wizard.dispatch({ type: 'cancel' })">{{ t('v113Connect.usb.cancel') }}</button>
    </div>
  </div>

  <div v-else class="flex flex-col gap-3 px-3 py-3" role="alert" data-connect-usb-fail :data-error="error">
    <div class="flex items-start gap-2.5">
      <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <div class="min-w-0">
        <p class="text-sm font-semibold">{{ t(`v113Connect.usbFail.${error}.title`) }}</p>
        <p class="mt-0.5 text-xs text-ink-muted">{{ t(`v113Connect.usbFail.${error}.hint`) }}</p>
      </div>
    </div>
    <div class="flex flex-wrap justify-end gap-2">
      <button type="button" class="ccm-btn" data-connect-usb-qr @click="wizard.dispatch({ type: 'chooseQr' })">
        <QrCode :size="14" aria-hidden="true" />
        {{ t('v113Connect.usbFail.useQr') }}
      </button>
      <button v-if="error === 'downgrade'" type="button" class="ccm-btn ccm-btn-primary" data-connect-usb-installed @click="wizard.dispatch({ type: 'alreadyInstalled' })">{{ t('v113Connect.install.installed') }}</button>
      <button v-else type="button" class="ccm-btn ccm-btn-primary" data-connect-usb-retry @click="wizard.dispatch({ type: 'chooseUsb' })">
        <RotateCcw :size="14" aria-hidden="true" />
        {{ t('v113Connect.usbFail.retry') }}
      </button>
    </div>
  </div>
</template>
