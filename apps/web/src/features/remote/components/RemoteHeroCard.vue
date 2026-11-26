<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, CircleOff, Plus, Smartphone, SmartphoneNfc } from 'lucide-vue-next'
import type { RemoteStatus } from '@ccm/shared'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import { activeDevices, isConnected } from '../lib/remote'
import { useRemoteStore } from '../stores/remote'
import type { ConnectWizard } from './connect/useConnectWizard'

defineProps<{ wizard: ConnectWizard }>()
const { t } = useI18n()
const remote = useRemoteStore()

type Tone = 'ok' | 'warn' | 'muted'
const TONE: Record<Tone, { icon: Component; cls: string }> = {
  ok: { icon: CircleCheck, cls: 'text-success' },
  warn: { icon: CircleAlert, cls: 'text-warning' },
  muted: { icon: CircleOff, cls: 'text-ink-muted' },
}
const RELAY: Record<RemoteStatus['relay'], Tone> = { off: 'muted', connecting: 'muted', online: 'ok', error: 'warn' }

const enabled = computed(() => !!remote.status?.enabled)
const devices = computed(() => activeDevices(remote.devices))
const live = computed(() => devices.value.filter(isConnected).length)
const state = computed<'off' | 'none' | 'ready'>(() => (!enabled.value ? 'off' : devices.value.length ? 'ready' : 'none'))
const title = computed(() => (state.value === 'ready' ? (devices.value.length > 1 ? t('v113Connect.hero.many', { n: devices.value.length }) : t('v113Connect.hero.one')) : t(`v113Connect.hero.${state.value}.title`)))
const hint = computed(() => (state.value === 'ready' ? (live.value ? t('v113Connect.hero.live', { n: live.value }) : '') : t(`v113Connect.hero.${state.value}.hint`)))

const tags = computed(() => {
  const s = remote.status
  const relay = RELAY[s?.relay ?? 'off']
  const pushOn = devices.value.some((d) => d.pushOn === true && d.push !== 'none')
  return [
    { key: 'lan', tone: (s?.listening ? 'ok' : 'warn') as Tone },
    { key: 'relay', tone: relay },
    { key: 'push', tone: (pushOn ? 'ok' : 'muted') as Tone },
  ]
})

const toggle = computed({
  get: () => enabled.value,
  set: (on: boolean) => void remote.setEnabled(on),
})
</script>

<template>
  <div data-connect-hero :data-connect-hero-state="state">
    <div class="flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-start sm:gap-4">
      <span class="grid size-12 shrink-0 place-items-center rounded-card" :class="state === 'ready' ? 'bg-st-done-soft text-success' : 'bg-fill text-ink-muted'">
        <component :is="state === 'ready' ? Smartphone : SmartphoneNfc" :size="22" aria-hidden="true" />
      </span>
      <div class="flex min-w-0 flex-1 flex-col gap-2">
        <div class="min-w-0">
          <p class="text-sm font-semibold" data-connect-hero-title>{{ title }}</p>
          <p v-if="hint" class="mt-0.5 text-xs text-ink-muted">{{ hint }}</p>
        </div>
        <ul v-if="state === 'ready'" class="flex flex-wrap gap-1.5" data-connect-tags>
          <li v-for="tag in tags" :key="tag.key" class="inline-flex items-center gap-1 rounded-full bg-fill px-2 py-0.5 text-2xs font-medium" :class="TONE[tag.tone].cls" :data-tag="tag.key" :data-tone="tag.tone">
            <component :is="TONE[tag.tone].icon" :size="11" aria-hidden="true" />
            <span class="text-ink">{{ t(`v113Connect.hero.${tag.key}`) }}</span>
          </li>
        </ul>
        <div class="flex flex-wrap gap-2">
          <button type="button" class="ccm-btn" :class="state === 'ready' ? '' : 'ccm-btn-primary'" data-connect-start data-remote-add @click="wizard.start()">
            <Plus :size="14" aria-hidden="true" />
            {{ state === 'ready' ? t('v113Connect.hero.more') : t('v113Connect.hero.connect') }}
          </button>
          <button v-if="state === 'ready'" type="button" class="ccm-btn" data-connect-recheck @click="wizard.dispatch({ type: 'recheck' })">
            {{ t('v113Connect.hero.recheck') }}
          </button>
        </div>
      </div>
    </div>
    <div class="border-t border-line">
      <ToggleSwitch v-model="toggle" class="px-3" :label="t('remoteV97.enable')" :hint="t('remoteV97.enableHint')" data-remote-enable />
    </div>
  </div>
</template>
