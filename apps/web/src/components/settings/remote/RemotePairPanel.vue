<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, Cloud, Hourglass, LoaderCircle, ShieldCheck, Wifi } from 'lucide-vue-next'
import { now } from '../../../lib/format'
import { clock, expMs, pairLeftMs, remotePairCancel, remotePairStart } from '../../../lib/remote'
import { useRemoteStore } from '../../../stores/remote'

const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const remote = useRemoteStore()

type Phase = 'starting' | 'waiting' | 'confirming' | 'expired' | 'tries' | 'failed' | 'paired'

const phase = ref<Phase>('starting')
const svg = ref('')
const exp = ref(0)
const pairedName = ref('')
const viaRelay = ref(false)
const knownDevices = new Set<string>()
let run = 0
let closeTimer: ReturnType<typeof setTimeout> | null = null

const left = computed(() => (phase.value === 'waiting' ? pairLeftMs(exp.value, now.value) : 0))

const META: Record<Phase, { icon: Component; cls: string; spin?: boolean }> = {
  starting: { icon: LoaderCircle, cls: 'text-ink-muted', spin: true },
  waiting: { icon: Hourglass, cls: 'text-ink-muted' },
  confirming: { icon: ShieldCheck, cls: 'text-ink-muted' },
  expired: { icon: CircleAlert, cls: 'text-warning' },
  tries: { icon: CircleAlert, cls: 'text-warning' },
  failed: { icon: CircleAlert, cls: 'text-warning' },
  paired: { icon: CircleCheck, cls: 'text-success' },
}
const meta = computed(() => META[phase.value])

async function start(): Promise<void> {
  const id = ++run
  phase.value = 'starting'
  svg.value = ''
  for (const d of remote.devices) knownDevices.add(d.deviceId)
  const out = await remotePairStart()
  if (id !== run) return
  if (!out) {
    phase.value = 'failed'
    return
  }
  const markup = await import('qrcode').then((m) => m.default.toString(out.qr, { type: 'svg', margin: 1, errorCorrectionLevel: 'L' })).catch(() => '')
  if (id !== run) return
  if (!markup) {
    phase.value = 'failed'
    return
  }
  svg.value = markup
  exp.value = out.exp
  viaRelay.value = out.relay === true
  phase.value = expMs(out.exp) > Date.now() ? 'waiting' : 'expired'
}

watch(left, (ms) => {
  if (phase.value === 'waiting' && ms <= 0) phase.value = 'expired'
})

watch(
  () => [remote.status?.pairing, remote.status?.pairEnded] as const,
  ([active, ended]) => {
    if (phase.value !== 'waiting' || active !== null) return
    phase.value = ended === 'tries' ? 'tries' : pairLeftMs(exp.value, Date.now()) <= 0 ? 'expired' : 'confirming'
  },
)

watch(
  () => remote.devices.map((d) => d.deviceId).join('|'),
  () => {
    if (phase.value !== 'waiting' && phase.value !== 'confirming') return
    const fresh = remote.devices.find((d) => !knownDevices.has(d.deviceId))
    if (!fresh) return
    pairedName.value = fresh.name
    phase.value = 'paired'
    closeTimer = setTimeout(() => emit('close'), 2500)
  },
)

async function cancel(): Promise<void> {
  run++
  if (phase.value === 'waiting' || phase.value === 'starting' || phase.value === 'confirming') await remotePairCancel()
  emit('close')
}

onMounted(() => void start())
onBeforeUnmount(() => {
  run++
  if (closeTimer) clearTimeout(closeTimer)
  if (phase.value === 'waiting') void remotePairCancel()
})
</script>

<template>
  <div class="flex flex-col items-center gap-3 px-3 py-4 sm:flex-row sm:items-start sm:gap-5" role="group" :aria-label="t('remoteV97.pair.title')" data-remote-pair>
    <div class="size-44 shrink-0 overflow-hidden rounded-inner bg-page p-1 [&>svg]:block [&>svg]:size-full" :class="svg ? '' : 'grid place-items-center bg-fill'">
      <Transition name="ccm-fade" mode="out-in">
        <div v-if="svg" :key="'qr'" class="size-full transition-opacity duration-(--ccm-dur-base)" :class="phase === 'waiting' || phase === 'paired' ? '' : 'opacity-25'" role="img" :aria-label="t('remoteV97.pair.qrLabel')" data-remote-qr v-html="svg" />
        <component :is="meta.icon" v-else :key="phase" :size="22" :class="[meta.cls, meta.spin ? 'animate-spin' : '']" aria-hidden="true" />
      </Transition>
    </div>
    <div class="flex min-w-0 flex-1 flex-col gap-2 self-stretch">
      <h4 class="text-sm font-semibold">{{ t('remoteV97.pair.title') }}</h4>
      <p class="text-xs text-ink-muted">{{ t('remoteV97.pair.help') }}</p>
      <p v-if="svg" class="flex min-w-0 items-center gap-1.5 text-xs text-ink-muted" data-remote-pair-route :data-relay="viaRelay">
        <component :is="viaRelay ? Cloud : Wifi" :size="14" class="shrink-0" aria-hidden="true" />
        <span class="min-w-0 truncate">{{ viaRelay ? t('remoteV97.pair.viaRelay') : t('remoteV97.pair.lanOnly') }}</span>
      </p>
      <p class="flex min-w-0 items-center gap-1.5 text-xs" :class="meta.cls" role="status" data-remote-pair-status>
        <component :is="meta.icon" :size="14" class="shrink-0" :class="meta.spin ? 'animate-spin' : ''" aria-hidden="true" />
        <span class="min-w-0 truncate">
          <template v-if="phase === 'waiting'">{{ t('remoteV97.pair.waiting') }}</template>
          <template v-else-if="phase === 'paired'">{{ t('remoteV97.pair.paired', { name: pairedName }) }}</template>
          <template v-else>{{ t(`remoteV97.pair.${phase}`) }}</template>
        </span>
        <span v-if="phase === 'waiting'" class="ml-auto shrink-0 font-mono text-ink tabular-nums" :aria-label="t('remoteV97.pair.left', { time: clock(left) })" data-remote-countdown>{{ clock(left) }}</span>
      </p>
      <div class="mt-auto flex flex-wrap justify-end gap-2 pt-1">
        <button v-if="phase === 'expired' || phase === 'failed' || phase === 'tries'" type="button" class="ccm-btn ccm-btn-primary" data-remote-pair-retry @click="start">{{ t('remoteV97.pair.retry') }}</button>
        <button type="button" class="ccm-btn" data-remote-pair-cancel @click="cancel">{{ phase === 'paired' ? t('remoteV97.pair.done') : t('remoteV97.pair.cancel') }}</button>
      </div>
    </div>
  </div>
</template>
