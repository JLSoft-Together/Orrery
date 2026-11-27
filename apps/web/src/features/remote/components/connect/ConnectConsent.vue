<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { CircleAlert, LoaderCircle, Pencil, ShieldCheck, Terminal } from 'lucide-vue-next'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const send = props.wizard.dispatch
const ITEMS = [
  { icon: Terminal, key: 'one' },
  { icon: Pencil, key: 'two' },
  { icon: ShieldCheck, key: 'three' },
] as const
</script>

<template>
  <div class="flex flex-col gap-3 px-3 py-3" data-connect-consent :data-connect-phase="wizard.name.value">
    <template v-if="wizard.name.value === 'consent'">
      <div class="flex flex-col gap-1">
        <h5 class="text-sm font-semibold">{{ t('v113Connect.consent.title') }}</h5>
        <p class="text-xs text-ink-muted">{{ t('v113Connect.consent.lead') }}</p>
      </div>
      <ul class="flex flex-col gap-2">
        <li v-for="item in ITEMS" :key="item.key" class="flex items-start gap-2.5 text-sm">
          <component :is="item.icon" :size="16" class="mt-0.5 shrink-0 text-ink-muted" aria-hidden="true" />
          <span class="min-w-0">{{ t(`v113Connect.consent.${item.key}`) }}</span>
        </li>
      </ul>
      <div class="flex flex-wrap justify-end gap-2">
        <button type="button" class="ccm-btn" data-connect-consent-no @click="send({ type: 'consentNo' })">{{ t('v113Connect.consent.no') }}</button>
        <button type="button" class="ccm-btn ccm-btn-primary" data-connect-consent-yes @click="send({ type: 'consentYes' })">{{ t('v113Connect.consent.yes') }}</button>
      </div>
    </template>

    <p v-else-if="wizard.name.value === 'enabling'" class="flex items-center gap-2 text-sm text-ink-muted" role="status" data-connect-enabling>
      <LoaderCircle :size="16" class="shrink-0 animate-spin" aria-hidden="true" />
      <span>{{ t('v113Connect.enabling.text') }}</span>
    </p>

    <template v-else>
      <div class="flex items-start gap-2.5" role="alert" data-connect-enable-fail :data-reason="wizard.state.value.enableFail">
        <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
        <p class="min-w-0 text-sm">{{ t(`v113Connect.enabling.${wizard.state.value.enableFail ?? 'timeout'}`) }}</p>
      </div>
      <div class="flex justify-end">
        <button type="button" class="ccm-btn ccm-btn-primary" data-connect-enable-retry @click="send({ type: 'retryEnable' })">{{ t('v113Connect.enabling.retry') }}</button>
      </div>
    </template>
  </div>
</template>
