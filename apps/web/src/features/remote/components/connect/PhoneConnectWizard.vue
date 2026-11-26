<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowLeft } from 'lucide-vue-next'
import { connectStep, type ConnectName } from '../../lib/connectFlow'
import ConnectConsent from './ConnectConsent.vue'
import ConnectInstallStep from './ConnectInstallStep.vue'
import ConnectPairStep from './ConnectPairStep.vue'
import ConnectQrStep from './ConnectQrStep.vue'
import ConnectStepper from './ConnectStepper.vue'
import ConnectTestStep from './ConnectTestStep.vue'
import ConnectUsbStep from './ConnectUsbStep.vue'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()

type Panel = { key: string; view: Component }
const PANELS: Partial<Record<ConnectName, Panel>> = {
  consent: { key: 'consent', view: ConnectConsent },
  enabling: { key: 'consent', view: ConnectConsent },
  enableFail: { key: 'consent', view: ConnectConsent },
  install: { key: 'install', view: ConnectInstallStep },
  installIos: { key: 'install', view: ConnectInstallStep },
  iosNoRelay: { key: 'install', view: ConnectInstallStep },
  usbInstalling: { key: 'usb', view: ConnectUsbStep },
  usbFail: { key: 'usb', view: ConnectUsbStep },
  qrInstall: { key: 'qr', view: ConnectQrStep },
  qrInstalled: { key: 'qr', view: ConnectQrStep },
  qrExpired: { key: 'qr', view: ConnectQrStep },
  pairWait: { key: 'pair', view: ConnectPairStep },
  pairExpired: { key: 'pair', view: ConnectPairStep },
  pairConfirm: { key: 'pair', view: ConnectPairStep },
  denied: { key: 'pair', view: ConnectPairStep },
  paired: { key: 'pair', view: ConnectPairStep },
  testing: { key: 'test', view: ConnectTestStep },
  testDone: { key: 'test', view: ConnectTestStep },
  testWarn: { key: 'test', view: ConnectTestStep },
  pushHelp: { key: 'test', view: ConnectTestStep },
}

const name = computed(() => props.wizard.name.value)
const panel = computed(() => PANELS[name.value] ?? null)
const step = computed(() => connectStep(name.value))
const canBack = computed(() => name.value !== 'paired' && name.value !== 'testing')
</script>

<template>
  <div class="overflow-hidden" data-connect-wizard :data-connect-state="name" :data-connect-step="step">
    <div class="flex items-center gap-2 border-b border-line px-3 py-2">
      <button v-if="canBack" type="button" class="ccm-btn" :aria-label="t('v113Connect.wizard.back')" data-connect-back @click="wizard.dispatch({ type: 'cancel' })">
        <ArrowLeft :size="14" aria-hidden="true" />
        {{ t('v113Connect.wizard.back') }}
      </button>
      <h4 class="min-w-0 flex-1 truncate text-sm font-semibold">{{ t('v113Connect.wizard.title') }}</h4>
    </div>
    <div v-if="step > 0" class="border-b border-line px-3 py-2.5">
      <ConnectStepper :step="step" />
    </div>
    <div class="relative">
      <Transition name="connect-step" mode="out-in">
        <component :is="panel.view" v-if="panel" :key="panel.key" :wizard="wizard" />
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.connect-step-enter-active {
  transition:
    opacity var(--ccm-dur-base) var(--ease-out-quint),
    transform var(--ccm-dur-base) var(--ease-out-quint);
}
.connect-step-leave-active {
  transition:
    opacity 160ms var(--ease-exit),
    transform 160ms var(--ease-exit);
}
.connect-step-enter-from {
  opacity: 0;
  transform: translateX(12px);
}
.connect-step-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}
@media (prefers-reduced-motion: reduce) {
  .connect-step-enter-active,
  .connect-step-leave-active {
    transition: opacity 160ms linear;
  }
  .connect-step-enter-from,
  .connect-step-leave-to {
    transform: none;
  }
}
</style>
