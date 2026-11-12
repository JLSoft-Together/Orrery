<script setup lang="ts">
import { Bell, Bug, Check, Clock, EyeOff, Fingerprint, Globe, Lock, Moon, Trash2, TriangleAlert, ChevronRight } from 'lucide-vue-next'
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import PageBar from '../components/PageBar.vue'
import SettingsMachines from '../components/SettingsMachines.vue'
import Toggle from '../components/Toggle.vue'
import { askConfirm } from '../lib/confirm'
import { LOCK_TIMEOUTS } from '../lib/lockPolicy'
import { DEFAULT_CONFIG } from '../remote/types'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { useLockStore } from '../stores/lock'
import { usePrefsStore, type NotifyKey } from '../stores/prefs'
import { useUpdateStore } from '../stores/update'
import type { LanguagePref, ThemePref } from '../storage/localPrefs'

const { t } = useI18n()
const hub = useHubStore()
const lock = useLockStore()
const prefs = usePrefsStore()
const update = useUpdateStore()

const timeoutKey: Record<number, string> = { 60000: 'm1', 900000: 'm15', 3600000: 'h1', 14400000: 'h4' }
const notifyRows: Array<{ key: NotifyKey; icon: typeof Bell; label: string }> = [
  { key: 'waiting', icon: Bell, label: 'settings.notifyWaiting' },
  { key: 'done', icon: Check, label: 'settings.notifyDone' },
  { key: 'error', icon: TriangleAlert, label: 'settings.notifyError' },
]

onMounted(() => void lock.checkSupport())

async function toggleLock(on: boolean): Promise<void> {
  if (on) {
    await lock.enable()
    return
  }
  const ok = await askConfirm({ title: t('settings.lockOffTitle'), body: t('settings.lockOffBody'), ok: t('settings.lockOffOk'), cancel: t('common.cancel'), danger: true })
  if (ok) lock.disable()
}

async function wipe(): Promise<void> {
  const ok = await askConfirm({ title: t('settings.wipeTitle'), body: t('debug.clearConfirm'), ok: t('settings.wipeOk'), cancel: t('common.cancel'), danger: true })
  if (!ok) return
  await hub.wipeAll()
  prefs.resetAll()
  lock.disable()
  navigate({ name: 'home' }, { replace: true })
}

function setLanguage(event: Event): void {
  prefs.setLanguage((event.target as HTMLSelectElement).value as LanguagePref)
}

function setTheme(event: Event): void {
  prefs.setTheme((event.target as HTMLSelectElement).value as ThemePref)
}
</script>

<template>
  <section class="page" data-testid="settings-view">
    <PageBar :title="t('tabs.settings')" backless />

    <h2 class="label head">{{ t('settings.security') }}</h2>
    <div class="card group" data-testid="settings-security">
      <div class="row">
        <Lock :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <div class="text">
          <span class="name">{{ t('settings.lock') }}</span>
          <span class="desc">{{ t(lock.supported === false ? 'settings.lockUnsupported' : 'settings.lockDesc') }}</span>
        </div>
        <Toggle :model-value="prefs.local.lockEnabled" :label="t('settings.lock')" :disabled="lock.supported === false || lock.busy" data-testid="lock-toggle" @update:model-value="toggleLock" />
      </div>
      <p v-if="lock.failure && !prefs.local.lockEnabled" class="fail" role="alert" data-testid="lock-failure">{{ t(`settings.lockFail.${lock.failure}`) }}</p>
      <template v-if="prefs.local.lockEnabled">
        <div class="row stack">
          <div class="inline">
            <Clock :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
            <span class="name">{{ t('settings.lockAfter') }}</span>
          </div>
          <div class="seg" role="radiogroup" :aria-label="t('settings.lockAfter')" data-testid="lock-timeout">
            <button v-for="ms in LOCK_TIMEOUTS" :key="ms" type="button" role="radio" class="seg-btn" :class="{ on: prefs.local.lockTimeout === ms }" :aria-checked="prefs.local.lockTimeout === ms" :data-testid="`lock-timeout-${ms}`" @click="prefs.patchLocal({ lockTimeout: ms })">
              {{ t(`settings.timeouts.${timeoutKey[ms]}`) }}
            </button>
          </div>
        </div>
        <div class="row">
          <EyeOff :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
          <div class="text">
            <span class="name">{{ t('settings.lockClose') }}</span>
            <span class="desc">{{ t('settings.lockCloseDesc') }}</span>
          </div>
          <Toggle :model-value="prefs.local.lockClose" :label="t('settings.lockClose')" @update:model-value="prefs.patchLocal({ lockClose: $event })" />
        </div>
      </template>
      <p class="foot">{{ t('settings.lockNote') }}</p>
    </div>

    <h2 class="label head">{{ t('settings.notifications') }}</h2>
    <div class="card group" data-testid="settings-notify">
      <div v-for="r in notifyRows" :key="r.key" class="row">
        <component :is="r.icon" :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <span class="name grow">{{ t(r.label) }}</span>
        <Toggle :model-value="prefs.notify[r.key]" :label="t(r.label)" :data-testid="`notify-${r.key}`" @update:model-value="prefs.setNotify(r.key, $event)" />
      </div>
      <div class="row">
        <Fingerprint :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <div class="text">
          <span class="name">{{ t('settings.notifyContent') }}</span>
          <span class="desc">{{ t('settings.notifyContentDesc') }}</span>
        </div>
        <Toggle :model-value="prefs.notify.content" :label="t('settings.notifyContent')" data-testid="notify-content" @update:model-value="prefs.setNotify('content', $event)" />
      </div>
      <p class="foot">{{ t('settings.notifyNote') }}</p>
    </div>

    <h2 class="label head">{{ t('settings.appearance') }}</h2>
    <div class="card group">
      <label class="row">
        <Globe :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <span class="name grow">{{ t('settings.language') }}</span>
        <select class="select" :value="prefs.local.language" data-testid="language-select" @change="setLanguage">
          <option value="auto">{{ t('settings.auto') }}</option>
          <option value="vi">Tiếng Việt</option>
          <option value="en">English</option>
        </select>
      </label>
      <label class="row">
        <Moon :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <span class="name grow">{{ t('settings.theme') }}</span>
        <select class="select" :value="prefs.local.theme" data-testid="theme-select" @change="setTheme">
          <option value="auto">{{ t('settings.themeAuto') }}</option>
          <option value="light">{{ t('settings.themeLight') }}</option>
          <option value="dark">{{ t('settings.themeDark') }}</option>
        </select>
      </label>
    </div>

    <h2 class="label head">{{ t('settings.machines') }}</h2>
    <SettingsMachines />

    <h2 class="label head">{{ t('settings.data') }}</h2>
    <div class="card group">
      <button type="button" class="row btn-row danger" data-testid="wipe-all" @click="wipe">
        <Trash2 :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <span class="name grow">{{ t('debug.clear') }}</span>
      </button>
    </div>

    <h2 class="label head">{{ t('settings.about') }}</h2>
    <div class="card group" data-testid="settings-about">
      <div class="row">
        <span class="name grow">{{ t('debug.web') }}</span>
        <span class="val mono" data-testid="about-web">{{ update.info?.web ? update.info.web.slice(0, 8) : '-' }}</span>
      </div>
      <div class="row">
        <span class="name grow">{{ t('debug.proto') }}</span>
        <span class="val mono" data-testid="about-proto">{{ DEFAULT_CONFIG.proto }}</span>
      </div>
      <button type="button" class="row btn-row" data-testid="open-debug" @click="navigate({ name: 'debug' })">
        <Bug :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
        <span class="name grow">{{ t('debug.title') }}</span>
        <ChevronRight :size="18" :stroke-width="2" class="ico" aria-hidden="true" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 16px 24px;
}

.head {
  margin: 12px 0 0;
}

.group {
  overflow: hidden;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  padding: 8px 14px;
  border-top: 1px solid var(--ccm-line);
}

.row:first-child {
  border-top: 0;
}

.row.stack {
  flex-direction: column;
  align-items: stretch;
  gap: 10px;
  padding-top: 12px;
  padding-bottom: 12px;
}

.inline {
  display: flex;
  align-items: center;
  gap: 12px;
}

.btn-row {
  width: 100%;
  border-left: 0;
  border-right: 0;
  border-bottom: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.btn-row:first-child {
  border-top: 0;
}

.danger {
  color: var(--ccm-error);
}

.danger .ico,
.danger .name {
  color: var(--ccm-error);
}

.ico {
  flex: none;
  color: var(--ccm-accent-ink);
}

.text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.name {
  font-weight: 600;
}

.grow {
  flex: 1;
}

.desc {
  font-size: 0.75rem;
  color: var(--ccm-ink-muted);
}

.foot {
  margin: 0;
  padding: 10px 14px;
  border-top: 1px solid var(--ccm-line);
  font-size: 0.75rem;
  color: var(--ccm-ink-faint);
}

.fail {
  margin: 0;
  padding: 0 14px 10px;
  color: var(--ccm-error);
  font-size: 0.8125rem;
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
  transition: background-color 180ms var(--ease-out);
}

.seg-btn.on {
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  box-shadow: 0 0 0 1px var(--ccm-line);
}

.select {
  min-height: 44px;
  max-width: 50%;
  padding: 0 8px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
  text-align: right;
}

.val {
  color: var(--ccm-ink-muted);
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}
</style>
