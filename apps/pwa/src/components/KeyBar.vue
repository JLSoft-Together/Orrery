<script setup lang="ts">
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, CornerDownLeft } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import type { KeyId } from '../terminal/keys'
import type { ModState } from '../terminal/modifiers'

defineProps<{ ctrl: ModState; alt: ModState; disabled?: boolean }>()
const emit = defineEmits<{ key: [key: KeyId]; ctrl: []; alt: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="keys" role="toolbar" :aria-label="t('term.keys')" data-testid="key-bar">
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-esc" @click="emit('key', 'esc')">Esc</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-tab" @click="emit('key', 'tab')">Tab</button>
    <button type="button" class="key mono" :class="`mod-${ctrl}`" :disabled="disabled" :aria-pressed="ctrl !== 'off'" :data-mod="ctrl" data-testid="key-ctrl" @click="emit('ctrl')">Ctrl</button>
    <button type="button" class="key mono" :class="`mod-${alt}`" :disabled="disabled" :aria-pressed="alt !== 'off'" :data-mod="alt" data-testid="key-alt" @click="emit('alt')">Alt</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-shiftTab" @click="emit('key', 'shiftTab')">⇧Tab</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-ctrlC" @click="emit('key', 'ctrlC')">^C</button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.up')" data-testid="key-up" @click="emit('key', 'up')"><ArrowUp :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.down')" data-testid="key-down" @click="emit('key', 'down')"><ArrowDown :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.left')" data-testid="key-left" @click="emit('key', 'left')"><ArrowLeft :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.right')" data-testid="key-right" @click="emit('key', 'right')"><ArrowRight :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.enter')" data-testid="key-enter" @click="emit('key', 'enter')"><CornerDownLeft :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
  </div>
</template>

<style scoped>
.keys {
  display: flex;
  gap: 8px;
  padding: 6px 12px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
  flex: none;
}

.keys::-webkit-scrollbar {
  display: none;
}

.key {
  display: grid;
  place-items: center;
  flex: none;
  min-width: 44px;
  min-height: 40px;
  padding: 0 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 0.875rem;
  font-weight: 600;
  transition:
    background 150ms var(--ease-out),
    border-color 150ms var(--ease-out),
    transform 90ms var(--ease-out);
}

.key:active {
  transform: scale(0.94);
}

.key:disabled {
  color: var(--ccm-ink-faint);
}

.mod-once {
  color: var(--ccm-accent-ink);
  border-color: var(--ccm-accent);
  border-width: 1.5px;
  background: color-mix(in srgb, var(--ccm-accent) 14%, var(--ccm-control));
}

.mod-locked {
  color: var(--ccm-on-accent);
  background: var(--ccm-accent);
  border-color: var(--ccm-accent);
}
</style>
