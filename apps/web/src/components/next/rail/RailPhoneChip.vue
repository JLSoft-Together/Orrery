<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { BellOff, ShieldAlert, Smartphone } from 'lucide-vue-next'
import { now } from '../../../lib/format'
import { phoneChip, pushMuted, type PhoneTone } from '../../../lib/next/phoneChip'
import { flashSettingRow } from '../../../lib/settingsFlash'
import { useRemoteStore } from '../../../stores/remote'
import { useUiStore } from '../../../stores/ui'

const props = withDefaults(defineProps<{ iconOnly?: boolean }>(), { iconOnly: false })
const { t } = useI18n()
const remote = useRemoteStore()
const ui = useUiStore()

const chip = computed(() => phoneChip(remote.status, remote.blocked, now.value))
const muted = computed(() => pushMuted(chip.value))

const text = computed(() => {
  const c = chip.value
  if (c.tone === 'alert') return t('mistRemote.chip.alert', { n: c.blocked }, c.blocked)
  if (c.tone === 'live') return t('mistRemote.chip.phones', { n: c.connected }, c.connected)
  return t('mistRemote.chip.idle')
})

const tip = computed(() => {
  const c = chip.value
  const lines = [c.paired ? t('mistRemote.chip.tipConnected', { n: c.connected, total: c.paired }) : t('mistRemote.chip.tipNone')]
  if (c.paired) lines.push(c.pushReady ? t('mistRemote.chip.tipPush', { on: c.pushOn, n: c.pushReady }) : t('mistRemote.chip.tipPushNone'))
  if (c.blocked) lines.push(t('mistRemote.chip.tipBlocked', { n: c.blocked }, c.blocked))
  lines.push(t('mistRemote.chip.tipOpen'))
  return lines.join('\n')
})

const label = computed(() => tip.value.split('\n').join(', '))

const signal = ref<PhoneTone | null>(null)
let signalTimer: ReturnType<typeof setTimeout> | null = null
watch(
  () => chip.value.tone,
  (tone, before) => {
    if (!chip.value.show || tone === before || tone === 'idle') return
    signal.value = tone
    if (signalTimer) clearTimeout(signalTimer)
    signalTimer = setTimeout(() => (signal.value = null), 500)
  },
)
onBeforeUnmount(() => {
  if (signalTimer) clearTimeout(signalTimer)
})

function open(): void {
  ui.settingsPage = 'remote'
  ui.settingsOpen = true
  flashSettingRow(chip.value.tone === 'alert' ? 'remoteBlocked' : 'remoteDevices')
}

const TONE_CLASS: Record<PhoneTone, string> = {
  idle: 'text-ink-muted hover:bg-fill hover:text-ink',
  live: 'bg-fill text-accent-ink hover:bg-fill-strong',
  alert: 'bg-warning-soft text-warning hover:brightness-95',
}
const DOT_CLASS: Record<PhoneTone, string> = {
  idle: 'bg-fill-strong',
  live: 'bg-success',
  alert: 'bg-warning',
}
</script>

<template>
  <Transition name="ccm-fade">
    <button
      v-if="chip.show"
      type="button"
      class="ccm-next-phone-chip relative inline-flex shrink-0 cursor-pointer items-center gap-1 font-medium whitespace-nowrap transition-colors duration-(--ccm-dur-base) outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus"
      :class="[props.iconOnly ? 'size-8 justify-center rounded-control' : 'h-6 rounded-full px-2 text-xs', TONE_CLASS[chip.tone]]"
      :title="tip"
      :aria-label="label"
      aria-haspopup="dialog"
      data-phone-chip
      :data-tone="chip.tone"
      :data-connected="chip.connected"
      :data-blocked="chip.blocked || undefined"
      :data-push-off="muted || undefined"
      :data-signal="signal ?? undefined"
      @click="open"
    >
      <Transition name="ccm-swap" mode="out-in">
        <ShieldAlert v-if="chip.tone === 'alert'" key="alert" :size="14" class="shrink-0" aria-hidden="true" />
        <Smartphone v-else key="phone" :size="14" class="shrink-0" aria-hidden="true" />
      </Transition>
      <Transition name="ccm-swap" mode="out-in">
        <span v-if="!props.iconOnly" :key="text" class="tabular-nums" data-phone-chip-text>{{ text }}</span>
      </Transition>
      <BellOff v-if="muted && !props.iconOnly" :size="12" class="shrink-0 opacity-80" :aria-label="t('mistRemote.chip.pushOff')" data-phone-push-off />
      <span v-if="props.iconOnly && chip.tone === 'live'" class="absolute top-0.5 right-0.5 grid h-3.5 min-w-3.5 place-items-center rounded-full bg-canvas px-0.5 text-[10px] leading-none font-bold text-ink tabular-nums ring-1 ring-line-strong" aria-hidden="true" data-phone-chip-badge>{{ chip.connected }}</span>
      <span v-else-if="props.iconOnly && chip.tone === 'alert'" class="absolute top-0.5 right-0.5 grid h-3.5 min-w-3.5 place-items-center rounded-full bg-canvas px-0.5 text-[10px] leading-none font-bold text-warning tabular-nums ring-1 ring-warning" aria-hidden="true" data-phone-chip-badge>{{ chip.blocked }}</span>
      <span v-else-if="!props.iconOnly" class="size-1.5 shrink-0 rounded-full transition-colors duration-(--ccm-dur-base)" :class="DOT_CLASS[chip.tone]" aria-hidden="true" />
    </button>
  </Transition>
</template>

<style>
:root[data-ui='next'] .ccm-next-phone-chip[data-signal]::after {
  content: '';
  position: absolute;
  inset: -2px;
  border-radius: inherit;
  border: 2px solid var(--ccm-success);
  pointer-events: none;
  animation: ccm-next-phone-signal var(--ccm-dur-signal) var(--ease-out-quint) 1 forwards;
}
:root[data-ui='next'] .ccm-next-phone-chip[data-signal='alert']::after {
  border-color: var(--ccm-warning);
}
@keyframes ccm-next-phone-signal {
  from {
    opacity: 0.9;
  }
  to {
    opacity: 0;
  }
}
</style>
