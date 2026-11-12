<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleCheck, Moon, Smartphone, Wifi, Cloud } from 'lucide-vue-next'
import type { RemoteDevice } from '@ccm/shared'
import { now, relativeTime } from '../../../lib/format'
import { expMs } from '../../../lib/remote'

const props = defineProps<{ device: RemoteDevice }>()
const emit = defineEmits<{ revoke: [deviceId: string] }>()
const { t, locale } = useI18n()

const asking = ref(false)
const busy = ref(false)

const live = computed(() => props.device.state === 'connected')
const seen = computed(() => (live.value ? null : relativeTime(expMs(props.device.lastSeen), locale.value, now.value)))
const paired = computed(() => relativeTime(expMs(props.device.pairedAt), locale.value, now.value))
const platformLabel = computed(() => t(`remoteV97.devices.platform.${props.device.platform === 'android' || props.device.platform === 'ios' ? props.device.platform : 'unknown'}`))
const via = computed(() => (props.device.via === 'relay' ? { icon: Cloud, label: t('remoteV97.devices.relay') } : props.device.via === 'lan' ? { icon: Wifi, label: t('remoteV97.devices.lan') } : null))

async function confirm(): Promise<void> {
  busy.value = true
  emit('revoke', props.device.deviceId)
}
</script>

<template>
  <div class="flex min-h-12 items-center gap-3 px-3 py-2" :data-remote-device="device.deviceId" :data-device-state="device.state">
    <Smartphone :size="18" class="shrink-0" :class="live ? 'text-accent-ink' : 'text-ink-muted'" aria-hidden="true" />
    <div class="min-w-0 flex-1">
      <p class="truncate text-sm font-medium" :title="device.name">{{ device.name }}</p>
      <p class="mt-0.5 flex min-w-0 flex-wrap items-center gap-x-2 text-2xs text-ink-faint">
        <span>{{ platformLabel }}</span>
        <span v-if="paired">{{ t('remoteV97.devices.paired', { when: paired }) }}</span>
        <span v-if="seen">{{ t('remoteV97.devices.lastSeen', { when: seen }) }}</span>
      </p>
    </div>
    <Transition name="ccm-fade" mode="out-in">
      <div v-if="!asking" key="state" class="flex shrink-0 items-center gap-2">
        <span
          class="inline-flex h-5 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium"
          :class="live ? 'bg-success-soft text-success' : 'bg-fill text-ink-muted'"
          data-remote-device-state
        >
          <CircleCheck v-if="live" :size="12" aria-hidden="true" />
          <Moon v-else :size="12" aria-hidden="true" />
          {{ t(live ? 'remoteV97.devices.connected' : 'remoteV97.devices.idle') }}
        </span>
        <span v-if="via && live" class="inline-flex h-5 items-center gap-1 rounded-inner bg-fill px-1.5 text-2xs font-medium text-ink-muted" data-remote-device-via>
          <component :is="via.icon" :size="12" aria-hidden="true" />
          {{ via.label }}
        </span>
        <button type="button" class="ccm-btn ccm-btn-danger" :aria-label="t('remoteV97.devices.revokeLabel', { name: device.name })" data-remote-revoke @click="asking = true">{{ t('remoteV97.devices.revoke') }}</button>
      </div>
      <div v-else key="ask" class="flex shrink-0 items-center gap-2" role="group" :aria-label="t('remoteV97.devices.revokeAsk', { name: device.name })">
        <span class="hidden text-xs text-ink-muted sm:inline">{{ t('remoteV97.devices.revokeAsk', { name: device.name }) }}</span>
        <button type="button" class="ccm-btn ccm-btn-danger" :disabled="busy" data-remote-revoke-yes @click="confirm">{{ t('remoteV97.devices.revokeYes') }}</button>
        <button type="button" class="ccm-btn" :disabled="busy" data-remote-revoke-no @click="asking = false">{{ t('remoteV97.cancel') }}</button>
      </div>
    </Transition>
  </div>
</template>
