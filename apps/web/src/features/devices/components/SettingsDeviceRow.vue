<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bot, Camera, ChevronDown, CircleAlert, CircleCheck, Clock, Download, Info, LoaderCircle, Moon, Pencil, Play, Send, Smartphone, Undo2 } from 'lucide-vue-next'
import { REMOTE_DEVICE_ALIAS_MAX } from '@ccm/shared'
import { expMs } from '@features/remote'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import { now, relativeTime } from '../../../core/lib/format'
import { isOlder, type RowAction } from '../lib/deviceActions'
import type { DeviceRow } from '../lib/mergeDevices'
import type { LinkMs } from '../composables/useSettingsDevices'

const props = defineProps<{
  row: DeviceRow
  open: boolean
  action: RowAction | null
  busy: RowAction | 'test' | undefined
  version: string
  installed: boolean | null
  desktopVersion: string
  latency: LinkMs | undefined
}>()
const emit = defineEmits<{
  toggle: []
  run: [action: RowAction]
  test: []
  openApp: []
  capture: []
  push: [on: boolean]
  rename: [name: string]
  revoke: []
  undo: []
  merge: [yes: boolean]
  unmerge: []
}>()

const { t, locale } = useI18n()

const asking = ref(false)
const editing = ref(false)
const draft = ref('')
const field = ref<HTMLInputElement | null>(null)

watch(
  () => props.open,
  (open) => {
    if (open) return
    asking.value = false
    editing.value = false
  },
)

const remote = computed(() => props.row.remote)
const revoking = computed(() => remote.value?.revoking === true)
const live = computed(() => remote.value?.state === 'connected')
const adbReady = computed(() => !!props.row.adb && props.row.adb.online && props.row.adb.state === 'ready')
const outdated = computed(() => !!props.version && !!props.desktopVersion && isOlder(props.version, props.desktopVersion))
const icon = computed(() => (props.row.kind === 'emulator' ? Bot : Smartphone))
const pushOn = computed(() => remote.value?.pushOn !== false && !!remote.value?.push && remote.value.push !== 'none')
const pushNone = computed(() => !remote.value?.push || remote.value.push === 'none')
const detailId = computed(() => `dev-detail-${props.row.key.replace(/[^a-zA-Z0-9_-]/g, '_')}`)

const tags = computed(() => {
  const list: { key: string; label: string; tone: 'accent' | 'warn' | 'plain' }[] = []
  const r = remote.value
  if (r) {
    if (r.platform === 'web') list.push({ key: 'web', label: t('v113Devices.tag.web'), tone: 'accent' })
    else if (outdated.value) list.push({ key: 'remote', label: t('v113Devices.tag.remoteOld', { version: props.version }), tone: 'warn' })
    else list.push({ key: 'remote', label: props.version ? t('v113Devices.tag.remote', { version: props.version }) : t('v113Devices.tag.remoteBare'), tone: 'accent' })
  }
  const a = props.row.adb
  if (props.row.kind === 'emulator') list.push({ key: 'emu', label: t('v113Devices.tag.emulator'), tone: 'plain' })
  else if (a) list.push({ key: 'adb', label: t(a.transport === 'wifi' ? 'v113Devices.tag.wifi' : 'v113Devices.tag.usb'), tone: 'plain' })
  return list
})

const TAG_CLASS = { accent: 'bg-accent-soft text-accent-ink', warn: 'bg-warning-soft text-warning', plain: 'bg-fill text-ink-muted' }

const status = computed<{ tone: 'ok' | 'wait' | 'off' | 'warn'; text: string }>(() => {
  const r = remote.value
  if (props.busy && props.busy !== 'test') return { tone: 'wait', text: t(`v113Devices.state.${props.busy === 'boot' ? 'booting' : 'busy'}`) }
  if (revoking.value) return { tone: 'warn', text: t('v113Devices.state.revoking') }
  if (r && live.value) {
    const via = r.via ? t(`v113Devices.via.${r.via}`) : ''
    const ms = r.via === 'relay' ? props.latency?.relay : props.latency?.lan
    if (via && ms !== undefined) return { tone: 'ok', text: t('v113Devices.state.onlineViaMs', { via, ms }) }
    return { tone: 'ok', text: via ? t('v113Devices.state.onlineVia', { via }) : t('v113Devices.state.online') }
  }
  if (props.row.adb && props.row.adb.state === 'unauthorized') return { tone: 'warn', text: t('v113Devices.state.unauthorized') }
  if (adbReady.value && !r) return props.installed === false ? { tone: 'wait', text: t('v113Devices.state.notInstalled') } : { tone: 'ok', text: t('v113Devices.state.adbOnline') }
  if (adbReady.value) return { tone: 'off', text: t('v113Devices.state.offline') }
  if (props.row.avd && !props.row.adb) return { tone: 'off', text: t('v113Devices.state.stopped') }
  const when = r ? relativeTime(expMs(r.lastSeen), locale.value, now.value) : null
  return { tone: 'off', text: when ? t('v113Devices.state.offlineAgo', { when }) : t('v113Devices.state.offline') }
})

const STATUS_CLASS = { ok: 'bg-success-soft text-success', wait: 'bg-warning-soft text-warning', warn: 'bg-warning-soft text-warning', off: 'bg-fill text-ink-muted' }

const actionLabel = computed(() => {
  const a = props.busy && props.busy !== 'test' ? props.busy : props.action
  if (!a || a === 'blocked') return ''
  if (props.busy && props.busy !== 'test') return t(`v113Devices.action.${a === 'update' ? 'updating' : a === 'install' ? 'installing' : 'booting'}`)
  if (a === 'update') return props.desktopVersion ? t('v113Devices.action.update', { version: props.desktopVersion }) : t('v113Devices.action.updateBare')
  return t(`v113Devices.action.${a}`)
})

const pairedText = computed(() => {
  const via = remote.value?.pairedVia
  return via ? t('v113Devices.detail.paired', { via: t(`v113Devices.detail.paired${via === 'usb' ? 'Usb' : via === 'qr' ? 'Qr' : 'Lan'}`) }) : ''
})

async function startEdit(): Promise<void> {
  draft.value = remote.value?.alias ?? props.row.adb?.alias ?? ''
  editing.value = true
  await nextTick()
  field.value?.focus()
  field.value?.select()
}

function saveEdit(): void {
  if (!editing.value) return
  editing.value = false
  const next = Array.from(draft.value.replace(/[\u0000-\u001f]/g, '').trim()).slice(0, REMOTE_DEVICE_ALIAS_MAX).join('').trim()
  if (next !== (remote.value?.alias ?? props.row.adb?.alias ?? '')) emit('rename', next)
}

function askRevoke(): void {
  asking.value = true
}

function confirmRevoke(): void {
  asking.value = false
  emit('revoke')
}
</script>

<template>
  <div class="transition-opacity duration-(--ccm-dur-base)" :class="revoking ? 'opacity-70' : ''" :data-device-row="row.key" :data-device-kind="row.kind" :data-device-open="open ? 'true' : 'false'" :data-device-revoking="revoking ? 'true' : undefined">
    <div class="flex min-h-12 flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2">
      <button type="button" class="flex min-w-0 flex-1 cursor-pointer items-center gap-3 text-left focus-visible:-outline-offset-2" :aria-expanded="open" :aria-controls="detailId" data-device-toggle @click="emit('toggle')">
        <component :is="icon" :size="18" class="shrink-0" :class="status.tone === 'ok' ? 'text-accent-ink' : 'text-ink-muted'" aria-hidden="true" />
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-medium" :title="row.name" data-device-name data-share-hide>{{ row.name }}</span>
          <span class="mt-0.5 flex flex-wrap items-center gap-1">
            <span v-for="tag in tags" :key="tag.key" class="inline-flex h-5 items-center rounded-inner px-1.5 text-2xs font-medium" :class="TAG_CLASS[tag.tone]" :data-device-tag="tag.key">{{ tag.label }}</span>
          </span>
        </span>
        <span class="inline-flex h-5 shrink-0 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium" :class="STATUS_CLASS[status.tone]" :data-device-state="status.tone">
          <LoaderCircle v-if="busy && busy !== 'test'" :size="12" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
          <CircleCheck v-else-if="status.tone === 'ok'" :size="12" aria-hidden="true" />
          <CircleAlert v-else-if="status.tone === 'warn'" :size="12" aria-hidden="true" />
          <Clock v-else-if="status.tone === 'wait'" :size="12" aria-hidden="true" />
          <Moon v-else :size="12" aria-hidden="true" />
          {{ status.text }}
        </span>
        <ChevronDown :size="15" class="shrink-0 text-ink-faint transition-transform duration-(--ccm-dur-base) ease-out-quint motion-reduce:transition-none" :class="open ? 'rotate-180' : ''" aria-hidden="true" />
      </button>
      <button v-if="revoking" type="button" class="ccm-btn shrink-0" data-device-undo @click="emit('undo')">
        <Undo2 :size="13" aria-hidden="true" />
        {{ t('v113Devices.undo') }}
      </button>
      <button v-else-if="busy && busy !== 'test'" type="button" class="ccm-btn shrink-0" disabled data-device-action-busy>
        <LoaderCircle :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
        {{ actionLabel }}
      </button>
      <button v-else-if="action && action !== 'blocked'" type="button" class="ccm-btn shrink-0" :class="action === 'boot' ? '' : 'ccm-btn-primary'" :data-device-action="action" @click="emit('run', action)">
        <Play v-if="action === 'boot'" :size="13" aria-hidden="true" />
        <Download v-else :size="13" aria-hidden="true" />
        {{ actionLabel }}
      </button>
    </div>

    <p v-if="action === 'blocked' && !revoking" class="mx-3 mb-2 flex items-start gap-1.5 text-2xs text-ink-muted" data-device-blocked>
      <Info :size="13" class="mt-px shrink-0" aria-hidden="true" />
      {{ t('v113Devices.blocked') }}
    </p>

    <Transition name="ccm-fade">
      <div v-if="row.suggestion && !revoking" class="mx-3 mb-2 flex flex-wrap items-center gap-2 rounded-control bg-accent-soft px-2.5 py-1.5" role="group" :aria-label="t('v113Devices.suggest.title')" data-device-suggest>
        <Info :size="14" class="shrink-0 text-accent-ink" aria-hidden="true" />
        <p class="min-w-0 flex-1 text-xs text-ink">
          <span class="font-medium">{{ t('v113Devices.suggest.title') }}</span>
          {{ t('v113Devices.suggest.body', { adb: row.suggestion.adbName, remote: row.suggestion.remoteName }) }}
        </p>
        <button type="button" class="ccm-btn ccm-btn-primary" data-device-merge @click="emit('merge', true)">{{ t('v113Devices.suggest.merge') }}</button>
        <button type="button" class="ccm-btn" data-device-keep @click="emit('merge', false)">{{ t('v113Devices.suggest.keep') }}</button>
      </div>
    </Transition>

    <Transition name="ccm-fade">
      <div v-if="open && !revoking" :id="detailId" class="border-t border-line bg-fill/40 px-3 py-2.5" data-device-detail>
        <Transition name="ccm-fade" mode="out-in">
          <div v-if="asking" key="ask" class="flex flex-wrap items-center gap-2" role="group" :aria-label="t('v113Devices.revokeAsk', { name: row.name })" data-device-revoke-ask>
            <CircleAlert :size="14" class="shrink-0 text-danger" aria-hidden="true" />
            <p class="min-w-0 flex-1 text-xs text-ink">{{ t('v113Devices.revokeAsk', { name: row.name }) }}</p>
            <button type="button" class="ccm-btn ccm-btn-danger" data-device-revoke-yes @click="confirmRevoke">{{ t('v113Devices.revokeYes') }}</button>
            <button type="button" class="ccm-btn" data-device-revoke-no @click="asking = false">{{ t('v113Devices.cancel') }}</button>
          </div>
          <div v-else key="info" class="flex flex-col gap-2">
            <ToggleSwitch
              v-if="remote"
              :model-value="pushOn"
              :label="t('v113Devices.detail.push')"
              :hint="pushNone ? t('v113Devices.detail.pushNone') : undefined"
              :disabled="pushNone"
              data-device-push
              @update:model-value="(on) => emit('push', on)"
            />
            <div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-2xs text-ink-faint">
              <span v-if="row.model" class="min-w-0 truncate" data-device-model>{{ row.model }}</span>
              <span v-if="pairedText">{{ pairedText }}</span>
              <span v-if="row.link" data-device-link>{{ t(row.link === 'hash' ? 'v113Devices.detail.linkedHash' : 'v113Devices.detail.linkedChosen') }}</span>
            </div>
            <div v-if="editing" class="flex items-center gap-2">
              <input
                ref="field"
                v-model="draft"
                type="text"
                class="ccm-field min-w-0 flex-1 text-sm"
                :maxlength="REMOTE_DEVICE_ALIAS_MAX"
                :placeholder="remote?.originalName || row.name"
                :aria-label="t('v113Devices.detail.renameField')"
                :title="t('v113Devices.detail.renameHint')"
                data-device-rename-input
                @keydown.enter.prevent="saveEdit"
                @keydown.esc.prevent.stop="editing = false"
                @blur="saveEdit"
              />
            </div>
            <div class="flex flex-wrap items-center gap-2" data-device-actions>
              <button v-if="remote" type="button" class="ccm-btn" :disabled="busy === 'test'" data-device-test @click="emit('test')">
                <LoaderCircle v-if="busy === 'test'" :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
                <Send v-else :size="13" aria-hidden="true" />
                {{ t('v113Devices.detail.test') }}
              </button>
              <button v-if="row.adb && adbReady && installed" type="button" class="ccm-btn" data-device-open-app @click="emit('openApp')">
                <Play :size="13" aria-hidden="true" />
                {{ t('v113Devices.detail.openApp') }}
              </button>
              <button v-if="row.adb && adbReady" type="button" class="ccm-btn" data-device-capture @click="emit('capture')">
                <Camera :size="13" aria-hidden="true" />
                {{ t('v113Devices.detail.capture') }}
              </button>
              <button v-if="remote || row.adb" type="button" class="ccm-btn" :aria-label="t('v113Devices.detail.renameLabel', { name: row.name })" data-device-rename @click="startEdit">
                <Pencil :size="13" aria-hidden="true" />
                {{ t('v113Devices.detail.rename') }}
              </button>
              <button v-if="row.link === 'chosen'" type="button" class="ccm-btn" data-device-split @click="emit('unmerge')">{{ t('v113Devices.detail.split') }}</button>
              <span class="flex-1" />
              <button v-if="remote" type="button" class="ccm-btn ccm-btn-danger" data-device-revoke @click="askRevoke">{{ t('v113Devices.detail.revoke') }}</button>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </div>
</template>
