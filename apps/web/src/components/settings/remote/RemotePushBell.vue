<script setup lang="ts">
import { computed, nextTick, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bell, BellOff, BellRing, ChevronDown, CircleDashed, Smartphone } from 'lucide-vue-next'
import type { RemoteDevice } from '@ccm/shared'

const props = defineProps<{ devices: readonly RemoteDevice[] }>()
const emit = defineEmits<{ toggle: [deviceId: string, on: boolean] }>()
const { t } = useI18n()

const open = ref(false)
const bell = ref<HTMLButtonElement | null>(null)
const pending = ref<Record<string, boolean>>({})

type PushState = 'on' | 'off' | 'none'
const stateOf = (d: RemoteDevice): PushState => (d.pushOn === false ? 'off' : !d.push || d.push === 'none' ? 'none' : 'on')
const TONE: Record<PushState, { icon: Component; cls: string }> = {
  on: { icon: BellRing, cls: 'text-success' },
  off: { icon: BellOff, cls: 'text-ink-muted' },
  none: { icon: CircleDashed, cls: 'text-warning' },
}

const onCount = computed(() => props.devices.filter((d) => stateOf(d) === 'on').length)
const anyOn = computed(() => onCount.value > 0)

function detail(d: RemoteDevice): string {
  const state = stateOf(d)
  const parts = [t(`remoteV98.push.state.${state}`)]
  if (state === 'on' && (d.push === 'web' || d.push === 'fcm')) {
    parts.push(t(`remoteV98.push.channel.${d.push}`))
    if (d.push === 'web') parts.push(t(`remoteV98.push.content.${d.pushContent === false ? 'off' : 'on'}`))
  }
  return parts.join(' · ')
}

function flip(d: RemoteDevice): void {
  const on = d.pushOn === false
  pending.value = { ...pending.value, [d.deviceId]: true }
  emit('toggle', d.deviceId, on)
  setTimeout(() => {
    const next = { ...pending.value }
    delete next[d.deviceId]
    pending.value = next
  }, 600)
}

async function close(): Promise<void> {
  open.value = false
  await nextTick()
  bell.value?.focus()
}
</script>

<template>
  <div class="mt-2 px-3" data-remote-push-bell :data-open="open ? 'true' : 'false'" @keydown.esc.stop="close">
    <button
      ref="bell"
      type="button"
      class="ccm-btn"
      :aria-expanded="open"
      aria-controls="remote-push-phones"
      data-remote-push-toggle
      @click="open = !open"
    >
      <component :is="anyOn ? Bell : BellOff" :size="14" :class="anyOn ? 'text-success' : 'text-ink-muted'" aria-hidden="true" />
      <span>{{ t('remoteV98.push.bell') }}</span>
      <span class="text-2xs text-ink-muted" data-remote-push-count>{{ t('remoteV98.push.count', { on: onCount, n: devices.length }) }}</span>
      <ChevronDown :size="14" class="transition-transform duration-(--ccm-dur-base)" :class="open ? 'rotate-180' : ''" aria-hidden="true" />
    </button>
    <Transition name="ccm-fade">
      <div v-if="open" id="remote-push-phones" class="ccm-group mt-2 overflow-hidden" role="group" :aria-label="t('remoteV98.push.bell')" data-remote-push-list>
        <p class="border-b border-line px-3 py-2 text-2xs text-ink-faint">{{ t('remoteV98.push.hint') }}</p>
        <div v-if="!devices.length" class="px-3 py-3 text-sm text-ink-muted">{{ t('remoteV98.push.empty') }}</div>
        <div v-for="d in devices" :key="d.deviceId" class="flex min-h-11 items-center gap-3 border-t border-line px-3 py-2 first-of-type:border-t-0" :data-remote-push-device="d.deviceId" :data-push-state="stateOf(d)">
          <Smartphone :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm text-ink" :title="d.name">{{ d.name }}</p>
            <p class="flex items-center gap-1 text-2xs" :class="TONE[stateOf(d)].cls" data-remote-push-state>
              <component :is="TONE[stateOf(d)].icon" :size="12" class="shrink-0" aria-hidden="true" />
              <span class="truncate">{{ detail(d) }}</span>
            </p>
          </div>
          <button
            type="button"
            role="switch"
            :aria-checked="d.pushOn !== false"
            :aria-label="t('remoteV98.push.toggle', { name: d.name })"
            :disabled="pending[d.deviceId]"
            class="relative inline-flex h-[22px] w-[38px] shrink-0 cursor-pointer items-center rounded-full transition-colors duration-(--ccm-dur-base) ease-out-quint disabled:cursor-wait"
            :class="d.pushOn !== false ? 'bg-accent' : 'bg-fill-strong'"
            data-remote-push-switch
            @click="flip(d)"
          >
            <span class="inline-block size-[18px] rounded-full bg-on-accent shadow-control transition-transform duration-(--ccm-dur-base) ease-spring" :class="d.pushOn !== false ? 'translate-x-[18px]' : 'translate-x-[2px]'" aria-hidden="true" />
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>
