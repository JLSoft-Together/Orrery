<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Copy, Minus, Square, X } from 'lucide-vue-next'
import { desktop } from '../../lib/desktop'

const { t } = useI18n()
const maximized = ref(false)
let off: (() => void) | undefined

onMounted(() => {
  void desktop?.windowControl?.('state').then((v) => (maximized.value = v === true))
  off = desktop?.onWindowMaximized?.((v) => (maximized.value = v))
})
onBeforeUnmount(() => off?.())

const act = (action: 'minimize' | 'toggle-maximize' | 'close'): void => void desktop?.windowControl?.(action)
</script>

<template>
  <div v-if="desktop?.windowControl" class="ccm-winctl flex shrink-0 items-center gap-0.5" role="group" :aria-label="t('mist.window.label')" data-window-controls>
    <button type="button" class="ccm-winctl-btn" :title="t('mist.window.minimize')" :aria-label="t('mist.window.minimize')" data-window="minimize" @click="act('minimize')">
      <Minus :size="14" :stroke-width="1.8" aria-hidden="true" />
    </button>
    <button
      type="button"
      class="ccm-winctl-btn"
      :title="maximized ? t('mist.window.restore') : t('mist.window.maximize')"
      :aria-label="maximized ? t('mist.window.restore') : t('mist.window.maximize')"
      data-window="maximize"
      @click="act('toggle-maximize')"
    >
      <Copy v-if="maximized" :size="12" :stroke-width="1.8" class="-scale-x-100" aria-hidden="true" />
      <Square v-else :size="12" :stroke-width="1.8" aria-hidden="true" />
    </button>
    <button type="button" class="ccm-winctl-btn ccm-winctl-close" :title="t('mist.window.close')" :aria-label="t('mist.window.close')" data-window="close" @click="act('close')">
      <X :size="15" :stroke-width="1.8" aria-hidden="true" />
    </button>
  </div>
</template>

<style>
.ccm-winctl {
  -webkit-app-region: no-drag;
  app-region: no-drag;
}
.ccm-winctl-btn {
  display: inline-grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-inner, 7px);
  color: var(--ccm-ink-muted);
  cursor: pointer;
  transition:
    background-color var(--ccm-dur-fast, 120ms) ease,
    color var(--ccm-dur-fast, 120ms) ease;
}
.ccm-winctl-btn:hover {
  background-color: var(--ccm-fill);
  color: var(--ccm-ink);
}
.ccm-winctl-btn:focus-visible {
  outline: 2px solid var(--ccm-focus);
  outline-offset: 1px;
}
.ccm-winctl-close:hover {
  background-color: var(--ccm-error);
  color: var(--ccm-on-accent);
}
</style>
