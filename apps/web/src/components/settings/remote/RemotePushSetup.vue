<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { BellRing, CircleAlert, CircleCheck, CircleDashed, ExternalLink, FileJson, LoaderCircle, X } from 'lucide-vue-next'
import { openExternal } from '../../../lib/preview'
import { activeDevices } from '../../../lib/remote'
import { useRemotePush } from '../../../composables/useRemotePush'
import { useRemoteStore } from '../../../stores/remote'
import type { PushFileKind } from '../../../composables/useRemotePushFiles'

const FIREBASE_URL = 'https://console.firebase.google.com/'

const { t, te } = useI18n()
const remote = useRemoteStore()
const p = useRemotePush()

const over = ref(false)
const asking = ref(false)
const picker = ref<HTMLInputElement | null>(null)
const target = ref('')

const devices = computed(() => activeDevices(remote.devices))
const slots = computed(() => [
  { kind: 'googleServices' as PushFileKind, held: p.files.googleServices.value },
  { kind: 'serviceAccount' as PushFileKind, held: p.files.serviceAccount.value },
])
const errorText = computed(() => {
  const code = p.problem.value ?? p.status.value?.error ?? null
  if (!code) return ''
  return t(te(`remoteV97.pushSetup.error.${code}`) ? `remoteV97.pushSetup.error.${code}` : 'remoteV97.pushSetup.error.unknown')
})
const statusErrorOnly = computed(() => !p.problem.value && !!p.status.value?.error)

function onDrop(e: DragEvent): void {
  over.value = false
  void p.add(Array.from(e.dataTransfer?.files ?? []))
}

function onPick(e: Event): void {
  const input = e.target as HTMLInputElement
  void p.add(Array.from(input.files ?? []))
  input.value = ''
}

async function remove(): Promise<void> {
  asking.value = false
  await p.remove()
}
</script>

<template>
  <div data-push-setup :data-state="p.configured.value ? 'ready' : 'none'">
    <div class="flex items-start gap-2 px-3 py-2.5">
      <BellRing :size="16" class="mt-0.5 shrink-0" :class="p.configured.value ? 'text-success' : 'text-ink-muted'" aria-hidden="true" />
      <p class="min-w-0 flex-1 text-xs text-ink-muted">{{ t('remoteV97.pushSetup.intro') }}</p>
    </div>

    <Transition name="ccm-fade">
      <div v-if="errorText" class="flex items-start gap-2 border-t border-line bg-warning-soft px-3 py-2 text-xs text-warning" role="alert" data-push-error :data-code="p.problem.value ?? p.status.value?.error">
        <CircleAlert :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
        <p class="min-w-0 flex-1">{{ statusErrorOnly ? t('remoteV97.pushSetup.ready.lastError', { text: errorText }) : errorText }}</p>
      </div>
    </Transition>

    <Transition name="ccm-fade" mode="out-in">
      <div v-if="!p.configured.value" key="setup" class="space-y-2 border-t border-line px-3 py-2.5" data-push-files>
        <div
          class="flex min-h-20 flex-col items-center justify-center gap-1 rounded-inner border border-dashed px-3 py-3 text-center text-xs transition-colors"
          :class="over ? 'border-accent bg-accent-soft text-accent-ink' : 'border-line-strong text-ink-muted'"
          data-push-drop
          @dragenter.prevent="over = true"
          @dragover.prevent="over = true"
          @dragleave.prevent="over = false"
          @drop.prevent="onDrop"
        >
          <FileJson :size="18" aria-hidden="true" />
          <p>{{ over ? t('remoteV97.pushSetup.dropActive') : t('remoteV97.pushSetup.drop') }}</p>
          <p>
            {{ t('remoteV97.pushSetup.dropOr') }}
            <button type="button" class="ccm-btn-plain inline-flex text-xs underline" data-push-choose @click="picker?.click()">{{ t('remoteV97.pushSetup.choose') }}</button>
          </p>
          <input ref="picker" type="file" accept=".json,application/json" multiple class="hidden" data-push-input @change="onPick" />
        </div>

        <ul class="space-y-1">
          <li v-for="s in slots" :key="s.kind" class="flex min-h-8 items-center gap-2 text-xs" :data-slot="s.kind" :data-slot-state="s.held ? 'ok' : 'missing'">
            <CircleCheck v-if="s.held" :size="14" class="shrink-0 text-success" aria-hidden="true" />
            <CircleDashed v-else :size="14" class="shrink-0 text-ink-faint" aria-hidden="true" />
            <span class="font-mono">{{ t(`remoteV97.pushSetup.slot.${s.kind}`) }}</span>
            <span class="min-w-0 flex-1 truncate" :class="s.held ? 'text-success' : 'text-ink-faint'">
              {{ s.held ? t('remoteV97.pushSetup.slot.project', { name: s.held.project }) : t('remoteV97.pushSetup.slot.missing') }}
            </span>
            <button v-if="s.held" type="button" class="ccm-btn-plain" :aria-label="t('remoteV97.pushSetup.slot.replace')" :disabled="p.busy.value" @click="p.files.clear(s.kind)">
              <X :size="14" aria-hidden="true" />
            </button>
          </li>
        </ul>

        <div class="space-y-1 text-2xs text-ink-faint" data-push-how>
          <p class="font-semibold text-ink-muted">{{ t('remoteV97.pushSetup.how.title') }}</p>
          <p><span class="font-mono">google-services.json</span>: {{ t('remoteV97.pushSetup.how.gs') }}</p>
          <p>{{ t('remoteV97.pushSetup.how.sa') }}</p>
        </div>

        <div class="flex items-center justify-between gap-2">
          <button type="button" class="ccm-btn" data-push-open @click="openExternal(FIREBASE_URL)">
            <ExternalLink :size="14" aria-hidden="true" />
            {{ t('remoteV97.pushSetup.how.open') }}
          </button>
          <button type="button" class="ccm-btn ccm-btn-primary" :disabled="!p.canSend.value" data-push-send @click="p.send()">
            <LoaderCircle v-if="p.phase.value === 'sending'" :size="14" class="animate-spin" aria-hidden="true" />
            {{ t(p.phase.value === 'sending' ? 'remoteV97.pushSetup.sending' : 'remoteV97.pushSetup.send') }}
          </button>
        </div>
      </div>

      <div v-else key="ready" class="space-y-2 border-t border-line px-3 py-2.5" data-push-ready>
        <p class="flex items-center gap-1.5 text-sm font-medium" data-push-project>
          <CircleCheck :size="14" class="shrink-0 text-success" aria-hidden="true" />
          {{ t('remoteV97.pushSetup.ready.project', { name: p.status.value?.projectId ?? '' }) }}
        </p>
        <p class="text-xs text-ink-muted" data-push-phones>{{ (p.status.value?.tokens ?? 0) > 0 ? t('remoteV97.pushSetup.ready.phones', { n: p.status.value?.tokens ?? 0 }) : t('remoteV97.pushSetup.ready.none') }}</p>
        <p v-if="p.sent.value !== null" class="flex items-center gap-1.5 text-xs text-success" role="status" data-push-sent>
          <CircleCheck :size="14" aria-hidden="true" />
          {{ t('remoteV97.pushSetup.ready.sent', { n: p.sent.value }) }}
        </p>
        <Transition name="ccm-fade" mode="out-in">
          <div v-if="!asking" key="actions" class="flex flex-wrap items-center justify-end gap-2">
            <select v-if="devices.length > 1" v-model="target" class="ccm-field w-auto text-xs" :aria-label="t('remoteV97.pushSetup.ready.pick')" data-push-target>
              <option value="">{{ t('remoteV97.pushSetup.ready.all') }}</option>
              <option v-for="d in devices" :key="d.deviceId" :value="d.deviceId">{{ d.name }}</option>
            </select>
            <button type="button" class="ccm-btn" :disabled="p.busy.value" data-push-test @click="p.test(target || undefined)">
              <LoaderCircle v-if="p.phase.value === 'testing'" :size="14" class="animate-spin" aria-hidden="true" />
              {{ t(p.phase.value === 'testing' ? 'remoteV97.pushSetup.ready.testing' : 'remoteV97.pushSetup.ready.test') }}
            </button>
            <button type="button" class="ccm-btn ccm-btn-danger" :disabled="p.busy.value" data-push-remove @click="asking = true">{{ t('remoteV97.pushSetup.ready.remove') }}</button>
          </div>
          <div v-else key="ask" class="flex flex-wrap items-center justify-end gap-2" role="group" :aria-label="t('remoteV97.pushSetup.ready.removeAsk')">
            <span class="min-w-0 text-xs text-ink-muted">{{ t('remoteV97.pushSetup.ready.removeAsk') }}</span>
            <button type="button" class="ccm-btn ccm-btn-danger" :disabled="p.busy.value" data-push-remove-yes @click="remove">{{ t('remoteV97.pushSetup.ready.removeYes') }}</button>
            <button type="button" class="ccm-btn" @click="asking = false">{{ t('remoteV97.cancel') }}</button>
          </div>
        </Transition>
      </div>
    </Transition>
  </div>
</template>
