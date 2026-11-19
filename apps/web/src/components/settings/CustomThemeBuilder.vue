<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RotateCcw, SunMedium, Palette } from 'lucide-vue-next'
import { autoGlowColors, effectiveTint, paletteAccents } from '../../lib/next/customTheme'
import { NEXT_PALETTES } from '@ccm/mist-tokens'
import { useUiGenStore } from '../../stores/uiGen'
import NextChip from '../next/ui/Chip.vue'
import ColorPicker from '../next/ui/ColorPicker.vue'
import ToggleSwitch from '../ToggleSwitch.vue'

const { t } = useI18n()
const uiGen = useUiGenStore()

const swatches = paletteAccents()
const seedOpen = ref(false)

const cfg = computed(() => uiGen.custom)
const autoGlows = computed(() => autoGlowColors(cfg.value.accent))
const glowA = computed(() => (!cfg.value.autoGlow && cfg.value.glowA) || autoGlows.value.a)
const glowB = computed(() => (!cfg.value.autoGlow && cfg.value.glowB) || autoGlows.value.b)
const tint = computed(() => effectiveTint(cfg.value))
const adjusted = computed(() => uiGen.customScheme.adjusted)

const seeds = computed(() =>
  NEXT_PALETTES.map((p) => ({ id: p.id, name: t(p.nameKey), accent: p.preview.light?.accent ?? p.preview.dark?.accent ?? '' })),
)

function setAuto(on: boolean): void {
  if (on) uiGen.setCustom({ autoGlow: true })
  else uiGen.setCustom({ autoGlow: false, glowA: glowA.value, glowB: glowB.value })
}

function setClarity(e: Event): void {
  uiGen.setCustom({ clarity: Number((e.target as HTMLInputElement).value) })
}

function seedFrom(id: string): void {
  uiGen.resetCustom(id)
  seedOpen.value = false
}
</script>

<template>
  <div class="mist-ctb flex flex-col gap-1 p-3" role="group" :aria-label="t('mist.custom.title')" data-custom-builder>
    <div class="flex flex-wrap items-center gap-2 px-0.5">
      <h4 class="min-w-0 flex-1 text-sm font-semibold">{{ t('mist.custom.title') }}</h4>
      <NextChip v-if="adjusted" tone="warning" :label="t('mist.custom.adjusted')" data-custom-adjusted>
        <template #icon><SunMedium :size="12" aria-hidden="true" /></template>
      </NextChip>
    </div>
    <p class="px-0.5 text-2xs text-ink-muted">{{ t('mist.custom.hint') }}</p>

    <div class="mist-ctb-row" data-custom-row="accent">
      <div class="min-w-0 flex-1">
        <p class="text-sm">{{ t('mist.custom.accent') }}</p>
        <p class="text-2xs text-ink-faint">{{ t('mist.custom.accentHint') }}</p>
      </div>
      <span class="mist-ctb-hex">{{ cfg.accent }}</span>
      <ColorPicker
        :model-value="cfg.accent"
        :name="t('mist.custom.accent')"
        :swatches="swatches"
        :recent="cfg.recent"
        @update:model-value="uiGen.setCustom({ accent: $event })"
        @commit="uiGen.addCustomRecent"
      />
    </div>

    <ToggleSwitch :model-value="cfg.autoGlow" :label="t('mist.custom.autoGlow')" :hint="t('mist.custom.autoGlowHint')" class="px-0.5" data-custom-autoglow @update:model-value="setAuto" />

    <div class="mist-ctb-row" data-custom-row="glowA">
      <div class="min-w-0 flex-1">
        <p class="text-sm">{{ t('mist.custom.glowA') }}</p>
        <p class="text-2xs text-ink-faint">{{ t('mist.custom.glowHint') }}</p>
      </div>
      <span class="mist-ctb-hex">{{ glowA }}</span>
      <ColorPicker
        :model-value="glowA"
        :name="t('mist.custom.glowA')"
        :swatches="swatches"
        :recent="cfg.recent"
        :disabled="cfg.autoGlow"
        @update:model-value="uiGen.setCustom({ glowA: $event })"
        @commit="uiGen.addCustomRecent"
      />
    </div>

    <div class="mist-ctb-row" data-custom-row="glowB">
      <div class="min-w-0 flex-1">
        <p class="text-sm">{{ t('mist.custom.glowB') }}</p>
        <p class="text-2xs text-ink-faint">{{ t('mist.custom.glowHint') }}</p>
      </div>
      <span class="mist-ctb-hex">{{ glowB }}</span>
      <ColorPicker
        :model-value="glowB"
        :name="t('mist.custom.glowB')"
        :swatches="swatches"
        :recent="cfg.recent"
        :disabled="cfg.autoGlow"
        @update:model-value="uiGen.setCustom({ glowB: $event })"
        @commit="uiGen.addCustomRecent"
      />
    </div>

    <div class="mist-ctb-row" data-custom-row="tint">
      <div class="min-w-0 flex-1">
        <p class="text-sm">{{ t('mist.custom.tint') }}</p>
        <p class="text-2xs text-ink-faint">{{ t('mist.custom.tintHint') }}</p>
      </div>
      <span class="mist-ctb-hex">{{ tint }}</span>
      <ColorPicker
        :model-value="tint"
        :name="t('mist.custom.tint')"
        :swatches="swatches"
        :recent="cfg.recent"
        @update:model-value="uiGen.setCustom({ tint: $event })"
        @commit="uiGen.addCustomRecent"
      />
    </div>

    <label class="mist-ctb-row" data-custom-row="clarity">
      <span class="min-w-0 flex-1 text-sm">{{ t('mist.custom.clarity') }}</span>
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        class="mist-ctb-range"
        :value="cfg.clarity"
        :aria-valuetext="t('mist.custom.clarityValue', { n: cfg.clarity })"
        data-custom-clarity
        @input="setClarity"
      />
      <span class="mist-ctb-hex w-10 text-right">{{ t('mist.custom.clarityValue', { n: cfg.clarity }) }}</span>
    </label>

    <div class="flex flex-wrap items-center gap-2 px-0.5 pt-2">
      <button type="button" class="mist-ctb-btn" :aria-expanded="seedOpen" aria-controls="mist-ctb-seeds" data-custom-seed @click="seedOpen = !seedOpen">
        <Palette :size="14" aria-hidden="true" />
        {{ t('mist.custom.seed') }}
      </button>
      <button type="button" class="mist-ctb-btn" data-custom-reset @click="uiGen.resetCustom()">
        <RotateCcw :size="14" aria-hidden="true" />
        {{ t('mist.custom.reset') }}
      </button>
    </div>
    <div v-if="seedOpen" id="mist-ctb-seeds" class="flex flex-wrap gap-1.5 px-0.5 pt-1" role="group" :aria-label="t('mist.custom.seedTitle')">
      <button v-for="s in seeds" :key="s.id" type="button" class="mist-ctb-seed" :data-custom-seed-item="s.id" @click="seedFrom(s.id)">
        <span class="mist-cp-dot" :style="{ background: s.accent }" aria-hidden="true" />
        {{ s.name }}
      </button>
    </div>
  </div>
</template>
