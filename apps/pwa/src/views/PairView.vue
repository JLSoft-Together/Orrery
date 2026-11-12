<script setup lang="ts">
import { Check, Cloud, LoaderCircle, RefreshCw, ShieldCheck, TriangleAlert, Wifi, X } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import PageBar from '../components/PageBar.vue'
import QrScanner from '../components/QrScanner.vue'
import { checkQrText, pairErrorKey } from '../lib/qrInput'
import { back, navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'

const { t } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()

const tab = ref<'scan' | 'paste'>('scan')
const phase = ref<'idle' | 'working' | 'error'>('idle')
const tried = ref<Array<'lan' | 'relay'>>([])
const failure = ref<string | null>(null)
const text = ref('')
const pasteError = ref<string | null>(null)
const deviceName = ref(defaultName())
let lastQr = ''

function defaultName(): string {
  const ua = typeof navigator === 'undefined' ? '' : navigator.userAgent
  if (/iPad/.test(ua)) return 'iPad'
  if (/iPhone/.test(ua)) return 'iPhone'
  return 'Orrery web'
}

const showScanner = computed(() => tab.value === 'scan' && phase.value === 'idle')

async function run(qr: string): Promise<void> {
  lastQr = qr
  phase.value = 'working'
  tried.value = []
  failure.value = null
  try {
    const machine = await hub.pair(qr, undefined, deviceName.value.trim() || defaultName(), (route) => tried.value.push(route))
    prefs.setLastMachine(machine.id)
    navigate({ name: 'monitor', machine: machine.id }, { replace: true })
  } catch (error) {
    const code = error && typeof error === 'object' && 'code' in error ? String((error as { code: unknown }).code) : String(error)
    failure.value = pairErrorKey(code)
    phase.value = 'error'
  }
}

function submitPaste(): void {
  const checked = checkQrText(text.value)
  if (!checked.ok) {
    pasteError.value = checked.error
    return
  }
  pasteError.value = null
  void run(checked.text)
}

function onScan(value: string): void {
  void run(value)
}

function retry(): void {
  if (lastQr) void run(lastQr)
}

function other(): void {
  phase.value = 'idle'
  tab.value = tab.value === 'scan' ? 'paste' : 'scan'
}
</script>

<template>
  <section class="page" data-testid="pair-view" :data-phase="phase">
    <PageBar :title="t('pair.title')" />

    <template v-if="phase === 'working'">
      <div class="card progress" role="status" data-testid="pair-progress">
        <h2 class="p-title">{{ t('pair.working') }}</h2>
        <ul class="steps">
          <li v-for="(route, i) in tried" :key="i" class="step" :data-route="route">
            <LoaderCircle v-if="i === tried.length - 1" :size="18" :stroke-width="2.2" class="spin" aria-hidden="true" />
            <X v-else :size="18" :stroke-width="2.2" class="fail" aria-hidden="true" />
            <component :is="route === 'lan' ? Wifi : Cloud" :size="18" :stroke-width="2" aria-hidden="true" />
            <span>{{ t(route === 'lan' ? 'pair.tryLan' : 'pair.tryRelay') }}</span>
          </li>
          <li v-if="tried.length === 0" class="step"><LoaderCircle :size="18" :stroke-width="2.2" class="spin" aria-hidden="true" /><span>{{ t('pair.reading') }}</span></li>
        </ul>
        <p class="hint">{{ t('pair.approve') }}</p>
      </div>
    </template>

    <template v-else-if="phase === 'error'">
      <div class="card alert" role="alert" data-testid="pair-error" :data-code="failure">
        <TriangleAlert :size="22" :stroke-width="2" aria-hidden="true" />
        <div class="alert-text">
          <strong>{{ t(`pair.err.${failure}.title`) }}</strong>
          <span>{{ t(`pair.err.${failure}.body`) }}</span>
        </div>
      </div>
      <div class="actions">
        <button type="button" class="btn btn-primary" data-testid="pair-retry" @click="retry"><RefreshCw :size="18" :stroke-width="2" aria-hidden="true" />{{ t('monitor.retry') }}</button>
        <button type="button" class="btn" data-testid="pair-other" @click="other">{{ t(tab === 'scan' ? 'pair.useCode' : 'pair.useScan') }}</button>
      </div>
    </template>

    <template v-else>
      <div class="seg" role="tablist">
        <button type="button" role="tab" class="seg-btn" :class="{ on: tab === 'scan' }" :aria-selected="tab === 'scan'" data-testid="pair-tab-scan" @click="tab = 'scan'">{{ t('pair.tabScan') }}</button>
        <button type="button" role="tab" class="seg-btn" :class="{ on: tab === 'paste' }" :aria-selected="tab === 'paste'" data-testid="pair-tab-paste" @click="tab = 'paste'">{{ t('pair.tabPaste') }}</button>
      </div>

      <template v-if="showScanner">
        <QrScanner @scan="onScan" @manual="tab = 'paste'" />
        <p class="cap">{{ t('pair.scanTitle') }}</p>
        <p class="cap-sub">{{ t('pair.scanHint') }}</p>
        <div class="card info">
          <ShieldCheck :size="22" :stroke-width="2" aria-hidden="true" />
          <div class="info-text">
            <strong>{{ t('pair.e2eTitle') }}</strong>
            <span>{{ t('pair.e2eBody') }}</span>
          </div>
        </div>
      </template>

      <template v-else>
        <label class="field">
          <span class="f-label">{{ t('pair.codeLabel') }}</span>
          <textarea v-model="text" class="input code" rows="5" spellcheck="false" autocomplete="off" autocapitalize="off" autocorrect="off" :placeholder="t('pair.codePlaceholder')" data-testid="pair-code" />
        </label>
        <p v-if="pasteError" class="f-error" role="alert" data-testid="pair-paste-error">{{ t(`pair.paste.${pasteError}`) }}</p>
        <p v-else class="f-hint">{{ t('pair.codeHint') }}</p>
      </template>

      <label class="field">
        <span class="f-label">{{ t('pair.deviceName') }}</span>
        <input v-model="deviceName" class="input" maxlength="40" autocomplete="off" data-testid="pair-name" />
      </label>

      <div class="spacer" />
      <button v-if="tab === 'paste'" type="button" class="btn btn-primary wide" data-testid="pair-submit" :disabled="!text.trim()" @click="submitPaste"><Check :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('pair.submit') }}</button>
      <button v-else type="button" class="btn wide" data-testid="pair-manual" @click="tab = 'paste'">{{ t('pair.useCode') }}</button>
      <button type="button" class="btn wide ghost" @click="back()">{{ t('common.cancel') }}</button>
    </template>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px calc(16px + env(safe-area-inset-bottom));
  min-height: 100%;
}

.seg {
  display: flex;
  padding: 3px;
  border-radius: var(--radius-control);
  background: var(--ccm-raised);
}

.seg-btn {
  flex: 1;
  min-height: 44px;
  border: 0;
  border-radius: calc(var(--radius-control) - 2px);
  background: transparent;
  font-weight: 600;
  color: var(--ccm-ink-muted);
  transition:
    background-color 180ms var(--ease-out),
    color 180ms var(--ease-out);
}

.seg-btn.on {
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  box-shadow: 0 0 0 1px var(--ccm-line);
}

.cap {
  margin: 4px 0 0;
  text-align: center;
  font-weight: 700;
}

.cap-sub {
  margin: 0;
  text-align: center;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.info,
.alert {
  display: flex;
  gap: 12px;
  padding: 14px;
}

.info {
  color: var(--ccm-info);
  border-color: color-mix(in srgb, var(--ccm-info) 38%, transparent);
  background: color-mix(in srgb, var(--ccm-info) 8%, var(--ccm-surface));
}

.alert {
  color: var(--ccm-error);
  border-color: color-mix(in srgb, var(--ccm-error) 38%, transparent);
  background: color-mix(in srgb, var(--ccm-error) 8%, var(--ccm-surface));
}

.info-text,
.alert-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.info-text strong,
.alert-text strong {
  color: var(--ccm-ink);
  font-size: 0.9375rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.f-label {
  font-weight: 700;
}

.input {
  width: 100%;
  min-height: 48px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-size: 1rem;
}

.code {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  resize: none;
  overflow-wrap: anywhere;
}

.f-hint,
.f-error {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.f-error {
  color: var(--ccm-error);
}

.spacer {
  flex: 1;
}

.wide {
  width: 100%;
}

.ghost {
  border-color: transparent;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.progress {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.p-title {
  margin: 0;
  font-size: 1.0625rem;
}

.steps {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.step {
  display: flex;
  align-items: center;
  gap: 10px;
}

.spin {
  color: var(--ccm-working);
  animation: spin 1s linear infinite;
}

.fail {
  color: var(--ccm-error);
}

.hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
