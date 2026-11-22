<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, ChevronDown, ChevronUp } from 'lucide-vue-next'
import { modeOf } from '../../../core/lib/themes'
import { NEXT_PALETTE_CUSTOM, NEXT_PALETTE_RANDOM, nextLegacyCards, nextPaletteCards, type NextPreview, type NextScheme } from '@ccm/mist-tokens'
import { MAU_MONO, mauPreview } from '../../next/lib/mau'
import { useSettingsStore } from '../../../core/stores/settings'
import { useUiGenStore } from '../../next/stores/uiGen'
import NextSegmentedControl from '../../next/components/ui/SegmentedControl.vue'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import PaletteMini from './PaletteMini.vue'
import SettingRow from './SettingRow.vue'
import SettingSection from './SettingSection.vue'
import CustomThemeBuilder from './CustomThemeBuilder.vue'
import RandomPaletteBar from './RandomPaletteBar.vue'

const { t } = useI18n()
const settings = useSettingsStore()
const uiGen = useUiGenStore()

const VIEW_ALL_KEY = 'ccm.next.paletteViewAll'
const mainCards = nextPaletteCards()
const legacyCards = nextLegacyCards()

function readOpen(): boolean {
  try {
    return localStorage.getItem(VIEW_ALL_KEY) === '1'
  } catch {
    return false
  }
}

const isLegacy = (id: string): boolean => legacyCards.some((c) => c.palette.id === id)
const open = ref(readOpen() || isLegacy(uiGen.nextTheme))
const cards = computed(() => (open.value ? [...mainCards, ...legacyCards] : mainCards))

function toggleOpen(): void {
  open.value = !open.value
  try {
    localStorage.setItem(VIEW_ALL_KEY, open.value ? '1' : '0')
  } catch {
    return
  }
}

watch(
  () => uiGen.nextTheme,
  (id) => {
    if (isLegacy(id)) open.value = true
  },
)
const starsAvailable = computed(() => Boolean(uiGen.palette.stars))

const generations = computed(() => [
  { value: 'classic' as const, label: t('mist.settings.genClassic') },
  { value: 'next' as const, label: t('mist.settings.genNext') },
])
const modeValue = computed<'light' | 'dark' | 'system'>(() => (settings.themeFollow ? 'system' : modeOf(settings.theme)))
const modes = computed(() => [
  { value: 'system' as const, label: t('themeV96.system') },
  { value: 'light' as const, label: t('settings.themeLight'), disabled: !uiGen.palette.schemes.includes('light') },
  { value: 'dark' as const, label: t('settings.themeDark') },
])
const motions = computed(() => [
  { value: 'full' as const, label: t('mist.settings.motionFull') },
  { value: 'reduced' as const, label: t('mist.settings.motionReduced') },
  { value: 'off' as const, label: t('mist.settings.motionOff') },
])

const schemeFree = (id: string): boolean => id === NEXT_PALETTE_CUSTOM || id === NEXT_PALETTE_RANDOM
const cardActive = (id: string, scheme: NextScheme): boolean => uiGen.nextTheme === id && (schemeFree(id) || id !== 'mist' || uiGen.scheme === scheme)
const anyActive = computed(() => cards.value.some((x) => cardActive(x.palette.id, x.scheme)))
const cardName = (nameKey: string, scheme: NextScheme, id: string): string => (id === 'mist' ? `${t(nameKey)}, ${t(`mist.palette.${scheme}`)}` : t(nameKey))
function cardPreview(c: (typeof mainCards)[number]): NextPreview | undefined {
  if (c.palette.id === NEXT_PALETTE_CUSTOM) return uiGen.customTheme[uiGen.scheme].preview
  if (c.palette.id === NEXT_PALETTE_RANDOM) return mauPreview(uiGen.randomTheme ? uiGen.randomTheme.tokens[uiGen.scheme] : MAU_MONO[uiGen.scheme])
  return c.palette.preview[c.scheme]
}

function pick(id: string, scheme: NextScheme, schemes: readonly NextScheme[]): void {
  uiGen.setNextTheme(id)
  if (id === 'mist' && schemes.includes(scheme)) settings.setThemeMode(scheme)
}

function onKey(e: KeyboardEvent, index: number): void {
  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key]
  if (step === undefined) return
  e.preventDefault()
  const list = cards.value
  const n = list.length
  const at = (index + step + n) % n
  const target = list[at]!
  pick(target.palette.id, target.scheme, target.palette.schemes)
  ;(e.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>('[role=radio]')[at]?.focus()
}
</script>

<template>
  <div data-settings-mist-appearance>
    <SettingSection id="uiGeneration" :title="t('mist.settings.generation')">
      <SettingRow :label="t('mist.settings.generation')" :info="t('mist.settings.generationHint')" data-setting-row="uiGeneration">
        <NextSegmentedControl
          :model-value="uiGen.generation"
          :options="generations"
          :label="t('mist.settings.generation')"
          data-setting="ui-generation"
          @update:model-value="uiGen.setGeneration"
        />
      </SettingRow>
    </SettingSection>

    <SettingSection id="theme" :title="t('mist.settings.palette')">
      <div class="px-3 py-3" data-setting-row="theme">
        <div class="grid grid-cols-[repeat(auto-fill,minmax(10.5rem,1fr))] gap-3" role="radiogroup" :aria-label="t('mist.settings.palette')" data-palette-grid>
          <template v-for="(c, i) in cards" :key="c.id">
            <p v-if="i === mainCards.length" class="ccm-label col-span-full text-2xs font-semibold text-ink-faint" aria-hidden="true" data-palette-legacy>{{ t('themeV96.legacyPalettes') }}</p>
            <button
              type="button"
              role="radio"
              class="mist-pal group flex cursor-pointer flex-col gap-2 p-2 text-left"
              :aria-checked="cardActive(c.palette.id, c.scheme)"
              :tabindex="cardActive(c.palette.id, c.scheme) || (i === 0 && !anyActive) ? 0 : -1"
              :data-palette-card="c.id"
              @click="pick(c.palette.id, c.scheme, c.palette.schemes)"
              @keydown="onKey($event, i)"
            >
              <PaletteMini v-if="cardPreview(c)" :preview="cardPreview(c)!" :stars="c.palette.stars" />
              <span class="flex items-center gap-1.5 px-0.5">
                <span class="min-w-0 flex-1 truncate text-sm font-medium">{{ cardName(c.palette.nameKey, c.scheme, c.palette.id) }}</span>
                <Check v-if="cardActive(c.palette.id, c.scheme)" :size="14" class="shrink-0 text-accent-ink" aria-hidden="true" />
              </span>
              <span class="line-clamp-2 px-0.5 text-2xs text-ink-muted">{{ t(c.palette.descKey) }}</span>
            </button>
          </template>
        </div>
        <div class="mt-3 flex items-center gap-2 border-t border-dashed border-line pt-2.5">
          <button
            type="button"
            class="inline-flex cursor-pointer items-center gap-1 rounded-control px-2.5 py-1 text-xs font-semibold ring-1 ring-line outline-none hover:bg-fill focus-visible:ring-2 focus-visible:ring-focus"
            :aria-expanded="open"
            data-palette-view-all
            @click="toggleOpen"
          >
            {{ open ? t('themeV96.viewLess') : t('themeV96.viewAll') }}
            <component :is="open ? ChevronUp : ChevronDown" :size="14" aria-hidden="true" />
          </button>
          <span v-if="!open" class="text-xs text-ink-muted">{{ t('themeV96.moreCount', { n: legacyCards.length }) }}</span>
        </div>
        <CustomThemeBuilder v-if="uiGen.nextTheme === NEXT_PALETTE_CUSTOM" class="mt-3" />
        <RandomPaletteBar v-if="uiGen.nextTheme === NEXT_PALETTE_RANDOM" />
      </div>
      <SettingRow :label="t('mist.settings.mode')" :info="t('mist.settings.modeHint')" data-setting-row="mode">
        <NextSegmentedControl
          :model-value="modeValue"
          :options="modes"
          :label="t('mist.settings.mode')"
          data-setting="theme-mode"
          @update:model-value="settings.setThemeMode"
        />
      </SettingRow>
    </SettingSection>

    <SettingSection id="glass" :title="t('mist.settings.surface')">
      <ToggleSwitch
        v-model="uiGen.clearTerm"
        :label="t('mist.settings.clearTerm')"
        :info="t('mist.settings.clearTermHint')"
        class="px-3"
        data-setting="clear-term"
        data-setting-row="clearTerm"
      />
      <SettingRow :label="t('mist.settings.motion')" :info="t('mist.settings.motionHint')" data-setting-row="motion">
        <NextSegmentedControl
          :model-value="uiGen.motion"
          :options="motions"
          :label="t('mist.settings.motion')"
          data-setting="motion"
          @update:model-value="uiGen.setMotion"
        />
      </SettingRow>
      <ToggleSwitch
        v-model="settings.stars"
        :label="t('mist.settings.stars')"
        :hint="starsAvailable ? t('themeV95.starsHint') : t('mist.settings.starsOnly')"
        :disabled="!starsAvailable"
        class="px-3"
        data-setting-row="stars"
      />
    </SettingSection>

  </div>
</template>
