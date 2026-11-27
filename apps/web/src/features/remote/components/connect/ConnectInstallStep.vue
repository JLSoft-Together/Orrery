<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronRight, Cloud, CircleAlert, Download, QrCode, Usb } from 'lucide-vue-next'
import SegmentedControl from '../../../../core/components/ui/SegmentedControl.vue'
import type { ConnectPlatform } from '../../lib/connectFlow'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const w = props.wizard

const platform = computed<ConnectPlatform>({
  get: () => w.state.value.platform,
  set: (p) => w.choosePlatform(p),
})
const platforms = computed(() => [
  { value: 'android' as const, label: t('v113Connect.install.android') },
  { value: 'ios' as const, label: t('v113Connect.install.ios') },
])

const usbReady = computed(() => w.target.value)
const blocked = computed(() => w.targets.value.find((x) => !x.check.ok) ?? null)
const usbFirst = computed(() => !!usbReady.value)
const usbDesc = computed(() => {
  if (!w.hasUsb) return t('v113Connect.install.usb.unavailable')
  if (usbReady.value) return t('v113Connect.install.usb.ready', { name: usbReady.value.device.name })
  const b = blocked.value
  if (b && !b.check.ok) return t(`v113Connect.install.usb.${b.check.reason}`, { name: b.device.name })
  return t('v113Connect.install.usb.none')
})
const IOS_STEPS = ['one', 'two', 'three'] as const
</script>

<template>
  <div class="flex flex-col" data-connect-install :data-connect-platform="platform">
    <div class="px-3 pt-3">
      <SegmentedControl v-model="platform" :options="platforms" :label="t('v113Connect.install.platform')" fill class="w-full" data-connect-platform />
    </div>

    <template v-if="wizard.name.value === 'install'">
      <div class="flex flex-col pt-1">
        <div class="flex items-start gap-3 px-3 py-3" :class="usbFirst ? 'order-1' : 'order-2 border-t border-line'" data-connect-usb-tile :data-ready="!!usbReady">
          <span class="grid size-8 shrink-0 place-items-center rounded-inner" :class="usbReady ? 'bg-accent-soft text-accent-ink' : 'bg-fill text-ink-muted'">
            <Usb :size="16" aria-hidden="true" />
          </span>
          <div class="min-w-0 flex-1">
            <p class="flex flex-wrap items-center gap-1.5 text-sm font-semibold">
              {{ t('v113Connect.install.usb.title') }}
              <span v-if="usbReady" class="rounded-full bg-accent-soft px-1.5 py-px text-2xs font-semibold text-accent-ink">{{ t('v113Connect.install.best') }}</span>
            </p>
            <p class="mt-0.5 text-xs text-ink-muted" data-connect-usb-desc>{{ usbDesc }}</p>
          </div>
          <button v-if="usbReady" type="button" class="ccm-btn ccm-btn-primary shrink-0" data-connect-usb-install @click="wizard.dispatch({ type: 'chooseUsb' })">
            <Download :size="14" aria-hidden="true" />
            {{ t('v113Connect.install.usb.button', { name: usbReady.device.name }) }}
          </button>
          <button v-else type="button" class="ccm-btn shrink-0" disabled data-connect-usb-wait>{{ t('v113Connect.install.usb.wait') }}</button>
        </div>

        <div class="flex items-start gap-3 px-3 py-3" :class="usbFirst ? 'order-2 border-t border-line' : 'order-1'" data-connect-qr-tile>
          <span class="grid size-8 shrink-0 place-items-center rounded-inner" :class="usbFirst ? 'bg-fill text-ink-muted' : 'bg-accent-soft text-accent-ink'">
            <QrCode :size="16" aria-hidden="true" />
          </span>
          <div class="min-w-0 flex-1">
            <p class="flex flex-wrap items-center gap-1.5 text-sm font-semibold">
              {{ t('v113Connect.install.qr.title') }}
              <span v-if="!usbFirst" class="rounded-full bg-accent-soft px-1.5 py-px text-2xs font-semibold text-accent-ink">{{ t('v113Connect.install.suggest') }}</span>
            </p>
            <p class="mt-0.5 text-xs text-ink-muted">{{ t('v113Connect.install.qr.desc') }}</p>
          </div>
          <button type="button" class="ccm-btn shrink-0" :class="usbFirst ? '' : 'ccm-btn-primary'" data-connect-qr-show @click="wizard.dispatch({ type: 'chooseQr' })">
            <QrCode :size="14" aria-hidden="true" />
            {{ t('v113Connect.install.qr.button') }}
          </button>
        </div>
      </div>
      <div class="border-t border-line px-3 py-2">
        <button type="button" class="ccm-btn-plain inline-flex items-center gap-1 text-xs text-accent-ink" data-connect-installed @click="wizard.dispatch({ type: 'alreadyInstalled' })">
          {{ t('v113Connect.install.installed') }}
          <ChevronRight :size="13" aria-hidden="true" />
        </button>
      </div>
    </template>

    <div v-else-if="wizard.name.value === 'installIos'" class="flex flex-col items-center gap-3 px-3 py-3 sm:flex-row sm:items-start sm:gap-5" data-connect-ios>
      <div class="size-40 shrink-0 overflow-hidden rounded-inner bg-page p-1 [&>svg]:block [&>svg]:size-full" role="img" :aria-label="t('v113Connect.ios.label')" data-share-hide v-html="wizard.iosSvg.value" />
      <div class="flex min-w-0 flex-1 flex-col gap-2">
        <p class="text-sm font-semibold">{{ t('v113Connect.ios.title') }}</p>
        <ol class="flex list-decimal flex-col gap-1 pl-5 text-sm">
          <li v-for="k in IOS_STEPS" :key="k">{{ t(`v113Connect.ios.${k}`) }}</li>
        </ol>
        <p class="font-mono text-2xs break-all text-ink-muted">{{ wizard.appUrl.value }}</p>
        <button type="button" class="ccm-btn ccm-btn-primary self-start" data-connect-ios-next @click="wizard.dispatch({ type: 'alreadyInstalled' })">
          {{ t('v113Connect.ios.next') }}
          <ChevronRight :size="13" aria-hidden="true" />
        </button>
      </div>
    </div>

    <div v-else class="flex flex-col gap-3 px-3 py-3" data-connect-ios-no-relay>
      <div class="flex items-start gap-2.5" role="alert">
        <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
        <div class="min-w-0">
          <p class="text-sm font-semibold">{{ t('v113Connect.ios.noRelayTitle') }}</p>
          <p class="mt-0.5 text-xs text-ink-muted">{{ t('v113Connect.ios.noRelayHint') }}</p>
        </div>
      </div>
      <div class="flex justify-end">
        <button type="button" class="ccm-btn ccm-btn-primary" data-connect-to-relay @click="wizard.toRelay()">
          <Cloud :size="14" aria-hidden="true" />
          {{ t('v113Connect.ios.toRelay') }}
        </button>
      </div>
    </div>
  </div>
</template>
