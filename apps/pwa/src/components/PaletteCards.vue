<script setup lang="ts">
import { NEXT_PALETTES, type NextPalette, type NextScheme } from '@ccm/mist-tokens'
import { Check } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ modelValue: string; scheme: NextScheme }>()
const emit = defineEmits<{ 'update:modelValue': [id: string] }>()
const { t } = useI18n()

function view(p: NextPalette): Record<string, string> {
  const pv = p.preview[props.scheme] ?? p.preview[p.schemes[0]!]!
  return { '--pv-canvas': pv.canvas, '--pv-a': pv.glowA, '--pv-b': pv.glowB, '--pv-glass': pv.glass, '--pv-ink': pv.ink, '--pv-accent': pv.accent, '--pv-well': pv.well }
}

function onKey(event: KeyboardEvent, index: number): void {
  const step = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 0
  if (!step) return
  event.preventDefault()
  const next = NEXT_PALETTES[(index + step + NEXT_PALETTES.length) % NEXT_PALETTES.length]!
  emit('update:modelValue', next.id)
  const group = (event.currentTarget as HTMLElement).parentElement
  group?.querySelectorAll<HTMLElement>('[role="radio"]')[NEXT_PALETTES.indexOf(next)]?.focus()
}
</script>

<template>
  <div class="grid" role="radiogroup" :aria-label="t('settings.palette')" data-testid="palette-cards">
    <button
      v-for="(p, i) in NEXT_PALETTES"
      :key="p.id"
      type="button"
      role="radio"
      class="pal"
      :class="{ on: modelValue === p.id }"
      :aria-checked="modelValue === p.id"
      :tabindex="modelValue === p.id ? 0 : -1"
      :data-testid="`palette-${p.id}`"
      @click="emit('update:modelValue', p.id)"
      @keydown="onKey($event, i)"
    >
      <span class="swatch" :style="view(p)" aria-hidden="true">
        <span class="glass" />
        <span class="well" />
        <span class="dot" />
      </span>
      <span class="name">{{ t(`settings.paletteNames.${p.id}`) }}</span>
      <span class="desc">{{ p.schemes.length === 1 ? t('settings.paletteDarkOnly') : t(`settings.paletteDesc.${p.id}`) }}</span>
      <span class="check" aria-hidden="true"><Check :size="14" :stroke-width="3" /></span>
    </button>
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  padding: 12px 14px;
}

.pal {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-height: 44px;
  padding: 8px;
  border: 1px solid var(--ccm-line);
  border-radius: var(--radius-control);
  background: var(--ccm-fill);
  color: var(--ccm-ink);
  text-align: left;
  transition:
    border-color var(--ccm-dur-base) var(--ccm-ease-standard),
    background-color var(--ccm-dur-base) var(--ccm-ease-standard),
    transform var(--ccm-dur-press) var(--ccm-ease-standard);
}

.pal:active {
  transform: scale(var(--ccm-motion-scale, 0.98));
}

.pal.on {
  border-color: var(--ccm-accent);
  background: var(--ccm-accent-soft);
  box-shadow: 0 0 0 1px var(--ccm-accent);
}

.swatch {
  position: relative;
  display: block;
  height: 56px;
  margin-bottom: 6px;
  overflow: hidden;
  border-radius: var(--radius-inner);
  background:
    radial-gradient(80% 90% at 0 0, var(--pv-a), transparent 70%),
    radial-gradient(80% 90% at 100% 0, var(--pv-b), transparent 70%),
    var(--pv-canvas);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.glass {
  position: absolute;
  left: 8px;
  right: 8px;
  top: 8px;
  height: 22px;
  border-radius: 7px;
  background: var(--pv-glass);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.well {
  position: absolute;
  left: 8px;
  bottom: 8px;
  width: 38%;
  height: 10px;
  border-radius: 5px;
  background: var(--pv-well);
}

.dot {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--pv-accent);
}

.name {
  font-size: 0.875rem;
  font-weight: 700;
}

.desc {
  color: var(--ccm-ink-muted);
  font-size: 0.75rem;
  line-height: 1.3;
}

.check {
  position: absolute;
  top: 14px;
  right: 14px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--ccm-accent);
  color: var(--ccm-on-accent);
  opacity: 0;
  transform: scale(0.6);
  transition:
    opacity var(--ccm-dur-base) var(--ccm-ease-enter),
    transform var(--ccm-dur-base) var(--ccm-ease-enter);
}

.pal.on .check {
  opacity: 1;
  transform: scale(1);
}
</style>
