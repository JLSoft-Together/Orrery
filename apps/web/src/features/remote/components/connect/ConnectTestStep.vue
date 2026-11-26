<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bell, CircleAlert, CircleCheck, CircleOff, Cloud, LoaderCircle, Send } from 'lucide-vue-next'
import { isUnknownCheck, type CheckLine, type CheckTone } from '../../lib/connectCheck'
import type { ConnectWizard } from './useConnectWizard'

const props = defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const w = props.wizard

const TONE: Record<CheckTone, { icon: Component; cls: string }> = {
  ok: { icon: CircleCheck, cls: 'text-success' },
  warn: { icon: CircleAlert, cls: 'text-warning' },
  muted: { icon: CircleOff, cls: 'text-ink-muted' },
}
const KEYS = ['lan', 'relay', 'push'] as const

const name = computed(() => w.name.value)
const summary = computed(() => w.summary.value)
const unknown = computed(() => !!summary.value && isUnknownCheck(summary.value))
const lineOf = (key: (typeof KEYS)[number]): CheckLine | null => summary.value?.lines.find((l) => l.key === key) ?? null
const relayMissing = computed(() => lineOf('relay')?.code === 'none')
const showLines = computed(() => name.value !== 'pushHelp' && !unknown.value)
</script>

<template>
  <div class="flex flex-col" data-connect-test :data-connect-phase="name">
    <ul v-if="showLines" class="flex flex-col divide-y divide-line" :aria-busy="name === 'testing'" data-connect-checks>
      <li v-for="k in KEYS" :key="k" class="flex items-center gap-2.5 px-3 py-2.5 text-sm" :data-check="k" :data-tone="lineOf(k)?.tone ?? 'pending'">
        <LoaderCircle v-if="!lineOf(k)" :size="15" class="shrink-0 animate-spin text-accent-ink" aria-hidden="true" />
        <component :is="TONE[lineOf(k)!.tone].icon" v-else :size="15" class="shrink-0" :class="TONE[lineOf(k)!.tone].cls" aria-hidden="true" />
        <span class="min-w-0 flex-1">
          <span class="block" :class="lineOf(k) ? '' : 'text-ink-muted'">{{ lineOf(k) ? t(`v113Connect.test.msg.${k}.${lineOf(k)!.code}`) : t(`v113Connect.test.${k}`) }}</span>
        </span>
        <span v-if="lineOf(k)?.ms !== undefined" class="shrink-0 font-mono text-xs tabular-nums text-ink-muted">{{ t('v113Connect.test.ms', { ms: lineOf(k)!.ms }) }}</span>
        <button v-if="k === 'relay' && relayMissing && name === 'testWarn'" type="button" class="ccm-btn shrink-0" data-connect-test-relay @click="wizard.toRelay()">
          <Cloud :size="14" aria-hidden="true" />
          {{ t('v113Connect.test.setupRelay') }}
        </button>
      </li>
    </ul>

    <p v-if="unknown" class="flex items-start gap-2.5 px-3 py-3 text-sm" role="status" data-connect-test-unknown>
      <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <span class="min-w-0">{{ t('v113Connect.test.unknown') }}</span>
    </p>

    <div v-if="name === 'testDone'" class="flex flex-wrap items-center gap-2 border-t border-line px-3 py-2.5" data-connect-test-ask>
      <Bell :size="15" class="shrink-0 text-ink-muted" aria-hidden="true" />
      <p class="min-w-0 flex-1 text-sm font-semibold">{{ t('v113Connect.test.asks') }}</p>
      <button type="button" class="ccm-btn" data-connect-push-no @click="wizard.dispatch({ type: 'pushNo' })">{{ t('v113Connect.test.no') }}</button>
      <button type="button" class="ccm-btn ccm-btn-primary" data-connect-push-yes @click="wizard.dispatch({ type: 'pushYes' })">{{ t('v113Connect.test.yes') }}</button>
    </div>

    <div v-else-if="name === 'testWarn'" class="flex justify-end border-t border-line px-3 py-2.5">
      <button type="button" class="ccm-btn ccm-btn-primary" data-connect-test-later @click="wizard.dispatch({ type: 'skip' })">{{ t('v113Connect.test.later') }}</button>
    </div>

    <div v-else-if="name === 'pushHelp'" class="flex flex-col gap-3 px-3 py-3" data-connect-push-help>
      <div class="flex items-start gap-2.5">
        <Bell :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
        <div class="min-w-0">
          <p class="text-sm font-semibold">{{ t('v113Connect.test.helpTitle') }}</p>
          <p class="mt-0.5 text-xs text-ink-muted">{{ t('v113Connect.test.helpHint') }}</p>
        </div>
      </div>
      <ol class="flex list-decimal flex-col gap-1 pl-9 text-sm">
        <li>{{ t('v113Connect.test.helpOne') }}</li>
        <li>{{ t('v113Connect.test.helpTwo') }}</li>
      </ol>
      <div class="flex flex-wrap justify-end gap-2">
        <button type="button" class="ccm-btn" data-connect-help-skip @click="wizard.dispatch({ type: 'skip' })">{{ t('v113Connect.test.skip') }}</button>
        <button type="button" class="ccm-btn ccm-btn-primary" data-connect-help-resend @click="wizard.dispatch({ type: 'recheck' })">
          <Send :size="14" aria-hidden="true" />
          {{ t('v113Connect.test.resend') }}
        </button>
      </div>
    </div>
  </div>
</template>
