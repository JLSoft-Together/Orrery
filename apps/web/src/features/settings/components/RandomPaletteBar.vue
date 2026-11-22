<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Pin, Shuffle, X } from 'lucide-vue-next'
import { mauTheme } from '../../next/lib/mau'
import { useUiGenStore } from '../../next/stores/uiGen'

const { t } = useI18n()
const uiGen = useUiGenStore()

const state = computed(() => uiGen.random)
const held = computed(() => state.value?.held === true)
const pins = computed(() =>
  (state.value?.pins ?? []).map((seed, i) => {
    const tokens = mauTheme(seed).tokens[uiGen.scheme]
    return { seed, n: i + 1, canvas: tokens.canvas!, accent: tokens.accent!, on: state.value?.seed === seed }
  }),
)
</script>

<template>
  <div class="mist-ctb mt-3 flex flex-col gap-2 p-3" data-random-palette>
    <div class="flex flex-wrap items-center gap-2">
      <button type="button" class="mist-ctb-btn" data-random-reshuffle @click="uiGen.reshuffleRandom()">
        <Shuffle :size="14" aria-hidden="true" />
        {{ t('mist.random.reshuffle') }}
      </button>
      <button type="button" class="mist-ctb-btn" :disabled="held" :aria-pressed="held" data-random-keep @click="uiGen.holdRandom()">
        <Pin :size="14" aria-hidden="true" />
        {{ held ? t('mist.random.kept') : t('mist.random.keep') }}
      </button>
      <span class="min-w-0 flex-1 text-2xs text-ink-muted" aria-live="polite">{{ held ? t('mist.random.held') : t('mist.random.rolling') }}</span>
    </div>
    <div v-if="pins.length" class="flex flex-wrap items-center gap-1.5" role="group" :aria-label="t('mist.random.pins')" data-random-pins>
      <span class="text-2xs text-ink-muted">{{ t('mist.random.pins') }}</span>
      <span v-for="p in pins" :key="p.seed" class="inline-flex items-center">
        <button
          type="button"
          class="mist-ctb-seed"
          :aria-pressed="p.on"
          :aria-label="t('mist.random.pin', { n: p.n })"
          :data-random-pin="p.seed"
          @click="uiGen.pickRandom(p.seed)"
        >
          <span class="mist-cp-dot" :style="{ background: p.canvas }" aria-hidden="true" />
          <span class="mist-cp-dot" :style="{ background: p.accent }" aria-hidden="true" />
          {{ p.n }}
        </button>
        <button type="button" class="mist-ctb-seed ml-0.5 px-1.5" :aria-label="t('mist.random.drop', { n: p.n })" :data-random-drop="p.seed" @click="uiGen.dropRandom(p.seed)">
          <X :size="12" aria-hidden="true" />
        </button>
      </span>
    </div>
  </div>
</template>
