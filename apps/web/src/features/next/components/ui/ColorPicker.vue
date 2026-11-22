<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDismiss } from '../../../../core/composables/useDismiss'
import { hexToHsv, hsvToHex, hueTrackBackground, parseHex, svBackground } from '../../lib/customTheme'

const props = withDefaults(defineProps<{ modelValue: string; name: string; swatches?: readonly string[]; recent?: readonly string[]; disabled?: boolean }>(), {
  swatches: () => [],
  recent: () => [],
  disabled: false,
})
const emit = defineEmits<{ 'update:modelValue': [hex: string]; commit: [hex: string] }>()
const { t } = useI18n()

const open = ref(false)
const button = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const area = ref<HTMLElement | null>(null)
const track = ref<HTMLElement | null>(null)
const place = ref<{ top: string; left: string }>({ top: '0px', left: '0px' })

const hsv = ref(hexToHsv(props.modelValue))
const hexText = ref(props.modelValue)
const hexBad = ref(false)
const changed = ref(false)

const current = computed(() => hsvToHex(hsv.value))
const sPct = computed(() => Math.round(hsv.value.s * 100))
const vPct = computed(() => Math.round(hsv.value.v * 100))
const hueDeg = computed(() => Math.round(hsv.value.h))
const areaStyle = computed(() => ({ background: svBackground(hsv.value.h) }))
const hueStyle = { background: hueTrackBackground() }
const areaThumb = computed(() => ({ left: `${hsv.value.s * 100}%`, top: `${(1 - hsv.value.v) * 100}%`, background: current.value }))
const hueThumb = computed(() => ({ left: `${(hsv.value.h / 360) * 100}%`, background: hsvToHex({ h: hsv.value.h, s: 1, v: 1 }) }))

watch(
  () => props.modelValue,
  (value) => {
    const hex = parseHex(value)
    if (!hex || hex === current.value) return
    hsv.value = hexToHsv(hex)
    hexText.value = hex
    hexBad.value = false
  },
)

function setHsv(next: { h?: number; s?: number; v?: number }): void {
  hsv.value = {
    h: Math.min(360, Math.max(0, next.h ?? hsv.value.h)),
    s: Math.min(1, Math.max(0, next.s ?? hsv.value.s)),
    v: Math.min(1, Math.max(0, next.v ?? hsv.value.v)),
  }
  hexText.value = current.value
  hexBad.value = false
  push()
}

function push(): void {
  if (current.value === parseHex(props.modelValue)) return
  changed.value = true
  emit('update:modelValue', current.value)
}

function useHex(hex: string): void {
  const h = parseHex(hex)
  if (!h) return
  hsv.value = hexToHsv(h)
  hexText.value = h
  hexBad.value = false
  changed.value = true
  emit('update:modelValue', h)
}

function onHexInput(e: Event): void {
  hexText.value = (e.target as HTMLInputElement).value
  const h = parseHex(hexText.value)
  hexBad.value = h === null
  if (!h) return
  hsv.value = hexToHsv(h)
  push()
}

function onHexBlur(): void {
  if (hexBad.value) {
    hexText.value = current.value
    hexBad.value = false
  }
}

function fromArea(e: PointerEvent): void {
  const r = area.value?.getBoundingClientRect()
  if (!r || r.width === 0 || r.height === 0) return
  setHsv({ s: (e.clientX - r.left) / r.width, v: 1 - (e.clientY - r.top) / r.height })
}

function fromTrack(e: PointerEvent): void {
  const r = track.value?.getBoundingClientRect()
  if (!r || r.width === 0) return
  setHsv({ h: ((e.clientX - r.left) / r.width) * 360 })
}

function onDown(e: PointerEvent, which: 'area' | 'track'): void {
  if (e.button !== 0) return
  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)
  el.focus()
  if (which === 'area') fromArea(e)
  else fromTrack(e)
}

function onMove(e: PointerEvent, which: 'area' | 'track'): void {
  const el = e.currentTarget as HTMLElement
  if (!el.hasPointerCapture(e.pointerId)) return
  if (which === 'area') fromArea(e)
  else fromTrack(e)
}

function onUp(e: PointerEvent): void {
  const el = e.currentTarget as HTMLElement
  if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId)
}

function onAreaKey(e: KeyboardEvent): void {
  const step = e.shiftKey ? 0.1 : 0.01
  const map: Record<string, { s?: number; v?: number }> = {
    ArrowLeft: { s: hsv.value.s - step },
    ArrowRight: { s: hsv.value.s + step },
    ArrowUp: { v: hsv.value.v + step },
    ArrowDown: { v: hsv.value.v - step },
    Home: { s: 0 },
    End: { s: 1 },
    PageUp: { v: hsv.value.v + 0.1 },
    PageDown: { v: hsv.value.v - 0.1 },
  }
  const next = map[e.key]
  if (!next) return
  e.preventDefault()
  setHsv(next)
}

function onHueKey(e: KeyboardEvent): void {
  const step = e.shiftKey ? 10 : 1
  const map: Record<string, number> = {
    ArrowLeft: hsv.value.h - step,
    ArrowDown: hsv.value.h - step,
    ArrowRight: hsv.value.h + step,
    ArrowUp: hsv.value.h + step,
    PageDown: hsv.value.h - 10,
    PageUp: hsv.value.h + 10,
    Home: 0,
    End: 360,
  }
  const next = map[e.key]
  if (next === undefined) return
  e.preventDefault()
  setHsv({ h: next })
}

function measure(): void {
  const r = button.value?.getBoundingClientRect()
  if (!r) return
  const width = panel.value?.offsetWidth ?? 264
  const height = panel.value?.offsetHeight ?? 360
  const below = r.bottom + 8
  const top = below + height > window.innerHeight - 12 && r.top - 8 - height > 12 ? r.top - 8 - height : below
  const left = Math.min(Math.max(12, r.right - width), Math.max(12, window.innerWidth - width - 12))
  place.value = { top: `${Math.round(top)}px`, left: `${Math.round(left)}px` }
}

async function toggle(): Promise<void> {
  if (props.disabled) return
  if (open.value) return close(true)
  hsv.value = hexToHsv(props.modelValue)
  hexText.value = parseHex(props.modelValue) ?? props.modelValue
  hexBad.value = false
  changed.value = false
  measure()
  open.value = true
  await nextTick()
  measure()
  area.value?.focus()
}

function close(focus = false): void {
  if (!open.value) return
  open.value = false
  if (changed.value) emit('commit', current.value)
  changed.value = false
  if (focus) button.value?.focus()
}

useDismiss({ active: open, inside: () => [button.value, panel.value], dismiss: () => close() })

function onPanelKey(e: KeyboardEvent): void {
  if (e.key !== 'Escape') return
  e.preventDefault()
  e.stopPropagation()
  close(true)
}

watch(open, (on) => {
  if (on) {
    window.addEventListener('resize', measure)
  } else {
    window.removeEventListener('resize', measure)
  }
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', measure)
})
</script>

<template>
  <button
    ref="button"
    type="button"
    class="mist-cp-swatch"
    :style="{ background: modelValue }"
    :disabled="disabled"
    :aria-expanded="open"
    aria-haspopup="dialog"
    :aria-label="t('mist.custom.picker.open', { name, hex: modelValue })"
    :title="modelValue"
    data-color-picker-open
    @click="toggle"
  />
  <Teleport to="body">
    <Transition name="ccm-next-pop">
      <div
        v-if="open"
        ref="panel"
        class="mist-cp-pop glass-pop"
        :style="place"
        role="dialog"
        :aria-label="t('mist.custom.picker.dialog', { name })"
        data-color-picker
        @keydown="onPanelKey"
      >
        <div
          ref="area"
          class="mist-cp-area"
          :style="areaStyle"
          role="slider"
          tabindex="0"
          :aria-label="t('mist.custom.picker.area')"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-valuenow="sPct"
          :aria-valuetext="t('mist.custom.picker.areaValue', { s: sPct, v: vPct })"
          data-color-area
          @pointerdown="onDown($event, 'area')"
          @pointermove="onMove($event, 'area')"
          @pointerup="onUp"
          @pointercancel="onUp"
          @keydown="onAreaKey"
        >
          <span class="mist-cp-thumb" :style="areaThumb" aria-hidden="true" />
        </div>
        <div
          ref="track"
          class="mist-cp-hue"
          :style="hueStyle"
          role="slider"
          tabindex="0"
          :aria-label="t('mist.custom.picker.hue')"
          aria-valuemin="0"
          aria-valuemax="360"
          :aria-valuenow="hueDeg"
          :aria-valuetext="t('mist.custom.picker.hueValue', { h: hueDeg })"
          data-color-hue
          @pointerdown="onDown($event, 'track')"
          @pointermove="onMove($event, 'track')"
          @pointerup="onUp"
          @pointercancel="onUp"
          @keydown="onHueKey"
        >
          <span class="mist-cp-thumb mist-cp-thumb-hue" :style="hueThumb" aria-hidden="true" />
        </div>
        <label class="mist-cp-hexrow">
          <span class="mist-cp-label">{{ t('mist.custom.picker.hex') }}</span>
          <span class="mist-cp-dot" :style="{ background: current }" aria-hidden="true" />
          <input
            class="mist-cp-hex"
            :class="hexBad ? 'mist-cp-hex-bad' : ''"
            :value="hexText"
            spellcheck="false"
            autocomplete="off"
            maxlength="7"
            :aria-invalid="hexBad"
            :aria-describedby="hexBad ? 'mist-cp-hex-err' : undefined"
            data-color-hex
            @input="onHexInput"
            @blur="onHexBlur"
            @keydown.enter.prevent="onHexBlur"
          />
        </label>
        <p v-if="hexBad" id="mist-cp-hex-err" class="mist-cp-err" role="alert">{{ t('mist.custom.picker.hexInvalid') }}</p>
        <div v-if="swatches.length" class="mist-cp-group" role="group" :aria-label="t('mist.custom.picker.swatches')">
          <span class="mist-cp-label">{{ t('mist.custom.picker.swatches') }}</span>
          <div class="mist-cp-row">
            <button
              v-for="hex in swatches"
              :key="`s${hex}`"
              type="button"
              class="mist-cp-chip"
              :style="{ background: hex }"
              :aria-label="t('mist.custom.picker.swatch', { hex })"
              :aria-pressed="hex === current"
              :title="hex"
              data-color-swatch
              @click="useHex(hex)"
            />
          </div>
        </div>
        <div v-if="recent.length" class="mist-cp-group" role="group" :aria-label="t('mist.custom.picker.recent')">
          <span class="mist-cp-label">{{ t('mist.custom.picker.recent') }}</span>
          <div class="mist-cp-row">
            <button
              v-for="hex in recent"
              :key="`r${hex}`"
              type="button"
              class="mist-cp-chip"
              :style="{ background: hex }"
              :aria-label="t('mist.custom.picker.swatch', { hex })"
              :aria-pressed="hex === current"
              :title="hex"
              data-color-recent
              @click="useHex(hex)"
            />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
