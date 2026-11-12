<script setup lang="ts">
import { computed, onBeforeUnmount, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, CircleAlert, CircleCheck, CircleDashed, CircleOff, Copy, ExternalLink, LoaderCircle } from 'lucide-vue-next'
import { openExternal } from '../../../lib/preview'
import { RELAY_STEPS, RELAY_TOKEN_URL, useRemoteRelay } from '../../../composables/useRemoteRelay'

const { t, te } = useI18n()
const r = useRemoteRelay()

const asking = ref(false)
const copied = ref(false)
let copyTimer: ReturnType<typeof setTimeout> | null = null

const shownError = computed(() => r.error.value ?? r.status.value?.error ?? null)
const errorText = computed(() => {
  const code = shownError.value
  if (!code) return ''
  return t(te(`remoteV97.relaySetup.error.${code}`) ? `remoteV97.relaySetup.error.${code}` : 'remoteV97.relaySetup.error.unknown')
})

type Tone = { icon: Component; cls: string; spin?: boolean }
const TONES: Record<string, Tone> = {
  none: { icon: CircleOff, cls: 'text-ink-muted' },
  deploying: { icon: LoaderCircle, cls: 'text-ink-muted', spin: true },
  ready: { icon: CircleCheck, cls: 'text-success' },
  error: { icon: CircleAlert, cls: 'text-warning' },
}
const tone = computed(() => (TONES[r.state.value] ?? TONES.none) as Tone)

const stepIndex = computed(() => (r.step.value ? RELAY_STEPS.indexOf(r.step.value) : -1))
const stepState = (i: number): 'done' | 'now' | 'todo' => (i < stepIndex.value ? 'done' : i === stepIndex.value ? 'now' : 'todo')

async function copy(): Promise<void> {
  const url = r.status.value?.url
  if (!url) return
  await navigator.clipboard?.writeText(url).catch(() => undefined)
  copied.value = true
  if (copyTimer) clearTimeout(copyTimer)
  copyTimer = setTimeout(() => (copied.value = false), 1800)
}

async function remove(deleteWorker: boolean): Promise<void> {
  asking.value = false
  await r.remove(deleteWorker)
}

onBeforeUnmount(() => {
  if (copyTimer) clearTimeout(copyTimer)
})
</script>

<template>
  <div data-relay-setup :data-state="r.state.value">
    <div class="flex items-start gap-2 px-3 py-2.5">
      <Transition name="ccm-fade" mode="out-in">
        <component :is="tone.icon" :key="r.state.value" :size="16" class="mt-0.5 shrink-0" :class="[tone.cls, tone.spin ? 'animate-spin' : '']" aria-hidden="true" />
      </Transition>
      <div class="min-w-0 flex-1">
        <p class="text-sm font-medium" data-relay-state>{{ t(`remoteV97.relaySetup.state.${r.state.value}`) }}</p>
        <p v-if="r.state.value === 'none' || r.state.value === 'error'" class="mt-0.5 text-xs text-ink-muted">{{ t('remoteV97.relaySetup.intro') }}</p>
      </div>
    </div>

    <Transition name="ccm-fade">
      <div v-if="errorText" class="flex items-start gap-2 border-t border-line bg-warning-soft px-3 py-2 text-xs text-warning" role="alert" data-relay-error :data-code="shownError">
        <CircleAlert :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
        <p class="min-w-0 flex-1">{{ r.state.value === 'ready' ? t('remoteV97.relaySetup.ready.lastError', { text: errorText }) : errorText }}</p>
      </div>
    </Transition>

    <Transition name="ccm-fade">
      <ol v-if="r.state.value === 'deploying'" class="space-y-1.5 border-t border-line px-3 py-2.5" data-relay-progress>
        <li v-for="(s, i) in RELAY_STEPS" :key="s" class="flex items-center gap-2 text-xs" :data-step="s" :data-step-state="stepState(i)">
          <CircleCheck v-if="stepState(i) === 'done'" :size="14" class="shrink-0 text-success" aria-hidden="true" />
          <LoaderCircle v-else-if="stepState(i) === 'now'" :size="14" class="shrink-0 animate-spin text-ink-muted" aria-hidden="true" />
          <CircleDashed v-else :size="14" class="shrink-0 text-ink-faint" aria-hidden="true" />
          <span :class="stepState(i) === 'todo' ? 'text-ink-faint' : 'text-ink'">{{ t(`remoteV97.relaySetup.progress.${s}`) }}</span>
        </li>
      </ol>
    </Transition>

    <Transition name="ccm-fade">
      <div v-if="r.state.value === 'ready'" class="space-y-2 border-t border-line px-3 py-2.5" data-relay-ready>
        <div class="flex items-center gap-2">
          <div class="min-w-0 flex-1">
            <p class="text-2xs text-ink-faint">{{ t('remoteV97.relaySetup.ready.url') }}</p>
            <p class="font-mono text-xs break-all" data-relay-url>{{ r.status.value?.url }}</p>
          </div>
          <button type="button" class="ccm-btn shrink-0" data-relay-copy @click="copy">
            <Check v-if="copied" :size="14" class="text-success" aria-hidden="true" />
            <Copy v-else :size="14" aria-hidden="true" />
            {{ t(copied ? 'remoteV97.relaySetup.ready.copied' : 'remoteV97.relaySetup.ready.copy') }}
          </button>
        </div>
        <p v-if="r.status.value?.account" class="text-2xs text-ink-faint" data-relay-account>{{ t('remoteV97.relaySetup.ready.account', { name: r.status.value.account }) }}</p>
        <p v-if="r.status.value?.warning === 'health'" class="flex items-start gap-1.5 text-xs text-warning" data-relay-health>
          <CircleAlert :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
          {{ t('remoteV97.relaySetup.ready.healthWarn') }}
        </p>
        <p class="text-2xs text-ink-faint">{{ t('remoteV97.relaySetup.ready.repair') }}</p>
        <p v-if="r.status.value?.updateAvailable" class="text-xs text-ink-muted" data-relay-update-note>
          {{ t('remoteV97.relaySetup.ready.updateAvail') }}
          <span v-if="r.updating.value">{{ t('remoteV97.relaySetup.ready.updatePaste') }}</span>
        </p>
        <p v-if="r.phase.value === 'removing'" class="flex items-center gap-1.5 text-xs text-ink-muted" role="status" data-relay-removing>
          <LoaderCircle :size="14" class="animate-spin" aria-hidden="true" />
          {{ t('remoteV97.relaySetup.remove.working') }}
        </p>
        <Transition name="ccm-fade" mode="out-in">
          <div v-if="!asking" key="actions" class="flex flex-wrap items-center justify-end gap-2">
            <button v-if="r.status.value?.updateAvailable && !r.updating.value" type="button" class="ccm-btn ccm-btn-primary" :disabled="r.busy.value" data-relay-update @click="r.update()">
              {{ t('remoteV97.relaySetup.ready.update') }}
            </button>
            <button type="button" class="ccm-btn ccm-btn-danger" :disabled="r.busy.value" data-relay-remove @click="asking = true">{{ t('remoteV97.relaySetup.ready.remove') }}</button>
          </div>
          <div v-else key="ask" class="flex flex-wrap items-center justify-end gap-2" role="group" :aria-label="t('remoteV97.relaySetup.remove.ask')">
            <span class="min-w-0 text-xs text-ink-muted">{{ t('remoteV97.relaySetup.remove.ask') }}</span>
            <button type="button" class="ccm-btn ccm-btn-danger" data-relay-remove-worker @click="remove(true)">{{ t('remoteV97.relaySetup.remove.withWorker') }}</button>
            <button type="button" class="ccm-btn" data-relay-remove-local @click="remove(false)">{{ t('remoteV97.relaySetup.remove.keepWorker') }}</button>
            <button type="button" class="ccm-btn" data-relay-remove-no @click="asking = false">{{ t('remoteV97.relaySetup.remove.cancel') }}</button>
          </div>
        </Transition>
      </div>
    </Transition>

    <Transition name="ccm-fade">
      <p v-if="r.notice.value" class="border-t border-line px-3 py-2 text-xs text-ink-muted" role="status" data-relay-notice>{{ t(`remoteV97.relaySetup.remove.${r.notice.value}`) }}</p>
    </Transition>

    <Transition name="ccm-fade">
      <div v-if="r.showSteps.value && r.state.value !== 'deploying'" class="border-t border-line" data-relay-steps>
        <div class="space-y-1.5 px-3 py-2.5">
          <h4 class="text-xs font-semibold">1. {{ t('remoteV97.relaySetup.step1.title') }}</h4>
          <p class="text-xs text-ink-muted">{{ t('remoteV97.relaySetup.step1.body') }}</p>
          <ul class="space-y-0.5 text-xs">
            <li class="font-mono">{{ t('remoteV97.relaySetup.step1.permScripts') }}</li>
            <li class="font-mono">{{ t('remoteV97.relaySetup.step1.permAccount') }}</li>
          </ul>
          <button type="button" class="ccm-btn" data-relay-open @click="openExternal(RELAY_TOKEN_URL)">
            <ExternalLink :size="14" aria-hidden="true" />
            {{ t('remoteV97.relaySetup.step1.open') }}
          </button>
        </div>

        <div class="space-y-1.5 border-t border-line px-3 py-2.5">
          <h4 class="text-xs font-semibold">2. {{ t('remoteV97.relaySetup.step2.title') }}</h4>
          <p v-if="r.status.value?.hasToken && !r.tokenChecked.value" class="text-2xs text-ink-faint" data-relay-has-token>{{ t('remoteV97.relaySetup.step2.saved') }}</p>
          <form class="flex items-center gap-2" @submit.prevent="r.check()">
            <input
              v-model="r.token.value"
              type="password"
              maxlength="512"
              autocomplete="off"
              spellcheck="false"
              class="ccm-field min-w-0 flex-1 font-mono text-xs"
              :aria-label="t('remoteV97.relaySetup.step2.label')"
              :placeholder="t('remoteV97.relaySetup.step2.placeholder')"
              :disabled="r.busy.value"
              data-relay-token
            />
            <button type="submit" class="ccm-btn shrink-0" :disabled="r.busy.value || !r.token.value.trim()" data-relay-check>
              <LoaderCircle v-if="r.phase.value === 'checking'" :size="14" class="animate-spin" aria-hidden="true" />
              {{ t(r.phase.value === 'checking' ? 'remoteV97.relaySetup.step2.checking' : 'remoteV97.relaySetup.step2.check') }}
            </button>
          </form>
          <p v-if="r.tokenChecked.value" class="flex items-center gap-1.5 text-xs text-success" data-relay-token-ok>
            <CircleCheck :size="14" aria-hidden="true" />
            {{ t('remoteV97.relaySetup.step2.ok') }}
          </p>
        </div>

        <div class="space-y-1.5 border-t border-line px-3 py-2.5">
          <h4 class="text-xs font-semibold">3. {{ t('remoteV97.relaySetup.step3.title') }}</h4>
          <p v-if="!r.tokenChecked.value" class="text-xs text-ink-faint">{{ t('remoteV97.relaySetup.step3.needToken') }}</p>
          <div v-else class="space-y-1.5" role="radiogroup" :aria-label="t('remoteV97.relaySetup.step3.pick')" data-relay-accounts>
            <label v-for="a in r.accounts.value" :key="a.id" class="flex min-h-8 cursor-pointer items-center gap-2 text-xs" :data-account="a.id">
              <input v-model="r.picked.value" type="radio" name="relay-account" :value="a.id" class="accent-[var(--ccm-accent)]" />
              <span class="min-w-0 flex-1 truncate" :title="a.name">{{ a.name }}</span>
            </label>
          </div>
          <div class="flex justify-end">
            <button type="button" class="ccm-btn ccm-btn-primary" :disabled="r.busy.value || !r.picked.value" data-relay-deploy @click="r.deploy()">
              {{ t('remoteV97.relaySetup.step3.deploy') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>
