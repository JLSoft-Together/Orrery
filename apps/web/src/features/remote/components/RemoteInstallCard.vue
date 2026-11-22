<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, Cloud, FileBox, Link2, LoaderCircle, Wifi } from 'lucide-vue-next'
import { now } from '../../../core/lib/format'
import { clock } from '../lib/remote'
import { flashSettingRow } from '@features/settings'
import {
  formatSize,
  installClear,
  installGet,
  installPick,
  installSetUrl,
  installStart,
  installStop,
  installUrl,
  installUseSuggested,
  isHttpsUrl,
  relayAppUrl,
  ticketLeftMs,
  type InstallError,
  type InstallReply,
  type InstallState,
  type InstallTicket,
} from '../lib/remoteInstall'
import { useRemoteStore } from '../stores/remote'
import SegmentedControl from '../../../core/components/ui/SegmentedControl.vue'

const { t } = useI18n()
const remote = useRemoteStore()

type Platform = 'android' | 'iphone'
type Way = 'wifi' | 'link'

const platform = ref<Platform>('android')
const way = ref<Way>('wifi')
const state = ref<InstallState | null>(null)
const error = ref<InstallError | null>(null)
const busy = ref(false)
const ticket = ref<InstallTicket | null>(null)
const ip = ref('')
const draft = ref('')
const saved = ref(false)
const qrSvg = ref('')
let run = 0

const platforms = computed(() => [
  { value: 'android' as const, label: t('v106Install.android') },
  { value: 'iphone' as const, label: t('v106Install.iphone') },
])
const ways = computed(() => [
  { value: 'wifi' as const, label: t('v106Install.way.wifi'), icon: Wifi },
  { value: 'link' as const, label: t('v106Install.way.link'), icon: Link2 },
])

const enabled = computed(() => !!remote.status?.enabled && !!remote.status?.listening)
const appUrl = computed(() => relayAppUrl(remote.status?.relayUrl))
const left = computed(() => (ticket.value ? ticketLeftMs(ticket.value.exp, now.value) : 0))
const expired = computed(() => !!ticket.value && left.value <= 0)
const wifiUrl = computed(() => (ticket.value && ip.value ? installUrl(ip.value, ticket.value.port, ticket.value.path) : ''))
const draftOk = computed(() => !draft.value.trim() || isHttpsUrl(draft.value))

const qrText = computed(() => {
  if (platform.value === 'iphone') return appUrl.value
  if (way.value === 'link') return state.value?.url ?? ''
  return expired.value ? '' : wifiUrl.value
})

watch(
  qrText,
  async (text) => {
    const id = ++run
    if (!text) {
      qrSvg.value = ''
      return
    }
    const markup = await import('qrcode').then((m) => m.default.toString(text, { type: 'svg', margin: 1, errorCorrectionLevel: 'M' })).catch(() => '')
    if (id === run) qrSvg.value = markup
  },
  { immediate: true },
)

function apply(out: InstallReply<InstallState>): void {
  if (out.ok) {
    state.value = out.value
    error.value = null
  } else {
    error.value = out.error
  }
}

async function act(fn: () => Promise<InstallReply<InstallState>>): Promise<void> {
  busy.value = true
  const out = await fn()
  busy.value = false
  apply(out)
  if (out.ok && ticket.value) await hideQr()
}

async function showQr(): Promise<void> {
  busy.value = true
  const out = await installStart()
  busy.value = false
  if (!out.ok) {
    error.value = out.error
    ticket.value = null
    return
  }
  error.value = null
  ticket.value = out.value
  if (!out.value.addresses.includes(ip.value)) ip.value = out.value.addresses[0] ?? ''
}

async function hideQr(): Promise<void> {
  ticket.value = null
  await installStop()
}

async function saveUrl(): Promise<void> {
  if (!draftOk.value) return
  busy.value = true
  const out = await installSetUrl(draft.value.trim())
  busy.value = false
  apply(out)
  saved.value = out.ok
}

async function clearUrl(): Promise<void> {
  draft.value = ''
  await saveUrl()
}

watch(draft, () => (saved.value = false))
watch(enabled, (on) => {
  if (!on) ticket.value = null
})

function toRelay(): void {
  flashSettingRow('remoteRelay')
}

onMounted(async () => {
  const out = await installGet()
  apply(out)
  if (out.ok) draft.value = out.value.url
  saved.value = false
})
onBeforeUnmount(() => {
  run++
  if (ticket.value) void installStop()
})
</script>

<template>
  <div class="flex flex-col gap-3 px-3 py-3" data-remote-install>
    <p class="text-xs text-ink-muted">{{ t('v106Install.hint') }}</p>
    <SegmentedControl v-model="platform" :options="platforms" :label="t('v106Install.tabs')" fill class="w-full" data-remote-install-platform />

    <div class="flex flex-col items-center gap-3 sm:flex-row sm:items-start sm:gap-5">
      <div
        v-if="qrSvg"
        class="size-44 shrink-0 overflow-hidden rounded-inner bg-page p-1 [&>svg]:block [&>svg]:size-full"
        role="img"
        :aria-label="t('v106Install.qrLabel')"
        data-remote-install-qr
        v-html="qrSvg"
      />

      <div class="flex min-w-0 flex-1 flex-col gap-2 self-stretch">
        <template v-if="platform === 'iphone'">
          <template v-if="appUrl">
            <p class="text-xs text-ink-muted">{{ t('v106Install.ios.intro') }}</p>
            <ol class="flex list-decimal flex-col gap-1 pl-5 text-sm" data-remote-install-ios-steps>
              <li>{{ t('v106Install.ios.step1') }}</li>
              <li>{{ t('v106Install.ios.step2') }}</li>
              <li>{{ t('v106Install.ios.step3') }}</li>
            </ol>
            <p class="font-mono text-2xs break-all text-ink-muted" data-remote-install-ios-url>{{ appUrl }}</p>
          </template>
          <div v-else class="flex flex-col items-start gap-2" data-remote-install-no-relay>
            <p class="flex min-w-0 items-start gap-1.5 text-xs text-ink-muted">
              <Cloud :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
              <span class="min-w-0">{{ t('v106Install.ios.noRelay') }}</span>
            </p>
            <button type="button" class="ccm-btn" data-remote-install-to-relay @click="toRelay">{{ t('v106Install.ios.toRelay') }}</button>
          </div>
        </template>

        <template v-else>
          <SegmentedControl v-model="way" :options="ways" :label="t('v106Install.way.label')" class="self-start" data-remote-install-way />

          <template v-if="way === 'wifi'">
            <p class="text-xs text-ink-muted">{{ t('v106Install.wifi.intro') }}</p>
            <div class="flex min-w-0 flex-wrap items-center gap-2" data-remote-install-apk>
              <FileBox :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
              <p v-if="state?.apk" class="min-w-0 flex-1 truncate text-sm" :title="state.apk.path">
                {{ state.apk.name }} <span class="text-xs text-ink-muted">{{ formatSize(state.apk.size) }}</span>
              </p>
              <p v-else class="min-w-0 flex-1 text-xs" :class="state?.missing ? 'text-warning' : 'text-ink-muted'">
                {{ state?.missing ? t('v106Install.wifi.missing') : t('v106Install.wifi.none') }}
              </p>
              <button type="button" class="ccm-btn shrink-0" :disabled="busy" data-remote-install-pick @click="act(installPick)">
                {{ state?.apk ? t('v106Install.wifi.change') : t('v106Install.wifi.pick') }}
              </button>
              <button v-if="state?.apk" type="button" class="ccm-btn shrink-0" :disabled="busy" data-remote-install-clear @click="act(installClear)">{{ t('v106Install.wifi.clear') }}</button>
            </div>
            <div v-if="state?.suggested" class="flex min-w-0 flex-wrap items-center gap-2 text-xs text-ink-muted" data-remote-install-suggested>
              <span class="min-w-0 flex-1 truncate" :title="state.suggested.path">{{ t('v106Install.wifi.suggested', { name: state.suggested.name }) }}</span>
              <button type="button" class="ccm-btn shrink-0" :disabled="busy" @click="act(installUseSuggested)">{{ t('v106Install.wifi.useSuggested') }}</button>
            </div>

            <p v-if="!enabled" class="text-xs text-warning" data-remote-install-off>{{ t('v106Install.wifi.needsOn') }}</p>
            <template v-else-if="state?.apk">
              <template v-if="ticket">
                <label v-if="ticket.addresses.length > 1" class="flex min-w-0 items-center gap-2 text-xs text-ink-muted">
                  <span class="shrink-0">{{ t('v106Install.wifi.address') }}</span>
                  <select v-model="ip" class="ccm-field w-auto min-w-0 font-mono text-xs" data-remote-install-ip>
                    <option v-for="a in ticket.addresses" :key="a" :value="a">{{ a }}</option>
                  </select>
                </label>
                <p class="font-mono text-2xs break-all text-ink-muted" data-remote-install-url>{{ wifiUrl }}</p>
                <p class="flex min-w-0 items-center gap-1.5 text-xs" :class="expired ? 'text-warning' : 'text-ink-muted'" role="status" data-remote-install-left>
                  <span class="min-w-0 truncate">{{ expired ? t('v106Install.wifi.expired') : t('v106Install.wifi.left', { time: clock(left) }) }}</span>
                </p>
                <p class="text-2xs text-ink-faint">{{ t('v106Install.wifi.unknownSources') }}</p>
              </template>
              <div class="flex flex-wrap justify-end gap-2">
                <button v-if="ticket && !expired" type="button" class="ccm-btn" data-remote-install-hide @click="hideQr">{{ t('v106Install.wifi.hide') }}</button>
                <button v-else type="button" class="ccm-btn ccm-btn-primary" :disabled="busy" data-remote-install-show @click="showQr">
                  <LoaderCircle v-if="busy" :size="14" class="animate-spin" aria-hidden="true" />
                  {{ ticket ? t('v106Install.wifi.renew') : t('v106Install.wifi.show') }}
                </button>
              </div>
            </template>
          </template>

          <template v-else>
            <p class="text-xs text-ink-muted">{{ t('v106Install.link.intro') }}</p>
            <div class="flex min-w-0 flex-wrap items-center gap-2">
              <input
                v-model="draft"
                type="url"
                inputmode="url"
                spellcheck="false"
                autocomplete="off"
                class="ccm-field min-w-0 flex-1 font-mono text-xs"
                :placeholder="t('v106Install.link.placeholder')"
                :aria-label="t('v106Install.way.link')"
                :aria-invalid="!draftOk"
                data-remote-install-link
                @keydown.enter.prevent="saveUrl"
              />
              <button type="button" class="ccm-btn ccm-btn-primary shrink-0" :disabled="busy || !draftOk || draft.trim() === (state?.url ?? '')" data-remote-install-link-save @click="saveUrl">
                {{ saved ? t('v106Install.link.saved') : t('v106Install.link.save') }}
              </button>
              <button v-if="state?.url" type="button" class="ccm-btn shrink-0" :disabled="busy" @click="clearUrl">{{ t('v106Install.link.clear') }}</button>
            </div>
            <p v-if="!draftOk" class="text-xs text-warning">{{ t('v106Install.link.invalid') }}</p>
            <p v-else-if="!state?.url" class="text-xs text-ink-muted">{{ t('v106Install.link.empty') }}</p>
            <p v-else class="font-mono text-2xs break-all text-ink-muted">{{ state.url }}</p>
          </template>
        </template>

        <p v-if="error" class="flex min-w-0 items-center gap-1.5 text-xs text-warning" role="alert" data-remote-install-error>
          <CircleAlert :size="14" class="shrink-0" aria-hidden="true" />
          <span class="min-w-0">{{ t(`v106Install.error.${error}`) }}</span>
        </p>
        <p v-if="qrSvg" class="text-xs text-ink-muted" data-remote-install-after>{{ t('v106Install.afterInstall') }}</p>
      </div>
    </div>
  </div>
</template>
