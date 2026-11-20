<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, CornerDownLeft } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import type { KeyId } from '../terminal/keys'
import type { ModState } from '../terminal/modifiers'

defineProps<{ ctrl: ModState; alt: ModState; disabled?: boolean }>()
const emit = defineEmits<{ key: [key: KeyId]; ctrl: []; alt: [] }>()
const { t } = useI18n()

const LONG_PRESS_MS = 500
const ROW_HOLD_MS = 8000
const ROW_KEYS: Array<{ id: KeyId; label: string }> = [
  { id: 'ctrlC', label: 'C' },
  { id: 'ctrlD', label: 'D' },
  { id: 'ctrlL', label: 'L' },
  { id: 'ctrlR', label: 'R' },
]

const strip = ref<HTMLElement | null>(null)
const more = ref({ left: false, right: false })
const pages = ref(1)
const page = ref(0)
const row = ref(false)
let pressTimer: ReturnType<typeof setTimeout> | null = null
let rowTimer: ReturnType<typeof setTimeout> | null = null
let longPressed = false
let observer: ResizeObserver | null = null
let frame = 0

const dots = computed(() => Array.from({ length: pages.value }, (_, i) => i))

function measure(): void {
  const el = strip.value
  if (!el) return
  const span = el.scrollWidth - el.clientWidth
  more.value = { left: el.scrollLeft > 4, right: span - el.scrollLeft > 4 }
  const count = span > 4 ? Math.ceil(el.scrollWidth / el.clientWidth) : 1
  pages.value = count
  page.value = span > 4 ? Math.min(count - 1, Math.round((el.scrollLeft / span) * (count - 1))) : 0
}

function closeRow(): void {
  row.value = false
  if (rowTimer) clearTimeout(rowTimer)
  rowTimer = null
}

function openRow(): void {
  row.value = true
  if (rowTimer) clearTimeout(rowTimer)
  rowTimer = setTimeout(closeRow, ROW_HOLD_MS)
}

function pressStart(): void {
  longPressed = false
  if (pressTimer) clearTimeout(pressTimer)
  pressTimer = setTimeout(() => {
    pressTimer = null
    longPressed = true
    openRow()
  }, LONG_PRESS_MS)
}

function pressEnd(): void {
  if (pressTimer) clearTimeout(pressTimer)
  pressTimer = null
}

function ctrlClick(): void {
  if (longPressed) {
    longPressed = false
    return
  }
  if (row.value) closeRow()
  emit('ctrl')
}

function rowKey(id: KeyId): void {
  closeRow()
  emit('key', id)
}

onMounted(() => {
  measure()
  if (strip.value && typeof ResizeObserver !== 'undefined') {
    observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(measure)
    })
    observer.observe(strip.value)
  }
  void nextTick(measure)
})

onBeforeUnmount(() => {
  observer?.disconnect()
  if (pressTimer) clearTimeout(pressTimer)
  if (rowTimer) clearTimeout(rowTimer)
})
</script>

<template>
  <div class="bar" data-testid="key-bar-wrap">
  <div v-if="row" class="keys row" role="toolbar" :aria-label="t('term.ctrlRow')" data-testid="key-row">
    <button v-for="k in ROW_KEYS" :key="k.id" type="button" class="key mono" :disabled="disabled" :aria-label="`Ctrl+${k.label}`" :data-testid="`key-row-${k.id}`" @click="rowKey(k.id)">^{{ k.label }}</button>
  </div>
  <div ref="strip" class="keys" :class="{ 'fade-left': more.left, 'fade-right': more.right }" role="toolbar" :aria-label="t('term.keys')" data-testid="key-bar" @scroll.passive="measure">
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-esc" @click="emit('key', 'esc')">Esc</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-tab" @click="emit('key', 'tab')">Tab</button>
    <button type="button" class="key mono" :class="`mod-${ctrl}`" :disabled="disabled" :aria-pressed="ctrl !== 'off'" :data-mod="ctrl" data-testid="key-ctrl" @pointerdown="pressStart" @pointerup="pressEnd" @pointerleave="pressEnd" @pointercancel="pressEnd" @contextmenu.prevent @click="ctrlClick">Ctrl</button>
    <button type="button" class="key mono" :class="`mod-${alt}`" :disabled="disabled" :aria-pressed="alt !== 'off'" :data-mod="alt" data-testid="key-alt" @click="emit('alt')">Alt</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-shiftTab" @click="emit('key', 'shiftTab')">⇧Tab</button>
    <button type="button" class="key mono" :disabled="disabled" data-testid="key-ctrlC" @click="emit('key', 'ctrlC')">^C</button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.up')" data-testid="key-up" @click="emit('key', 'up')"><ArrowUp :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.down')" data-testid="key-down" @click="emit('key', 'down')"><ArrowDown :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.left')" data-testid="key-left" @click="emit('key', 'left')"><ArrowLeft :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.right')" data-testid="key-right" @click="emit('key', 'right')"><ArrowRight :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
    <button type="button" class="key" :disabled="disabled" :aria-label="t('term.enter')" data-testid="key-enter" @click="emit('key', 'enter')"><CornerDownLeft :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
  </div>
  <div v-if="pages > 1" class="dots" role="presentation" data-testid="key-dots">
    <span v-for="i in dots" :key="i" class="dot" :class="{ on: i === page }" :title="t('term.keysPage', { n: i + 1, total: pages })" />
  </div>
  </div>
</template>

<style scoped>
.bar {
  flex: none;
  min-width: 0;
}

.keys {
  -webkit-touch-callout: none;
  user-select: none;
  display: flex;
  gap: 8px;
  padding: 6px 12px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
  flex: none;
}

.keys.fade-right {
  -webkit-mask-image: linear-gradient(to right, #000 calc(100% - 28px), transparent);
  mask-image: linear-gradient(to right, #000 calc(100% - 28px), transparent);
}

.keys.fade-left {
  -webkit-mask-image: linear-gradient(to left, #000 calc(100% - 28px), transparent);
  mask-image: linear-gradient(to left, #000 calc(100% - 28px), transparent);
}

.keys.fade-left.fade-right {
  -webkit-mask-image: linear-gradient(to right, transparent, #000 28px, #000 calc(100% - 28px), transparent);
  mask-image: linear-gradient(to right, transparent, #000 28px, #000 calc(100% - 28px), transparent);
}

.keys.row {
  padding-bottom: 0;
}

.dots {
  display: flex;
  justify-content: center;
  gap: 6px;
  padding: 0 0 4px;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ccm-line-strong);
  transition: background 150ms var(--ease-out);
}

.dot.on {
  background: var(--ccm-accent);
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
