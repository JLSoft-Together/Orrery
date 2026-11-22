<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, ChevronDown, ChevronUp } from 'lucide-vue-next'
import { withViewTransition } from '../../lib/motion'
import { THEMES, modeOf, type ThemeId, type ThemeMeta } from '../../lib/themes'
import { useSettingsStore } from '../../stores/settings'
import SegmentedControl from './SegmentedControl.vue'

const { t } = useI18n()
const settings = useSettingsStore()
const root = ref<HTMLElement | null>(null)

const VIEW_ALL_KEY = 'ccm.themeViewAll'
const full = THEMES.filter((th) => !th.legacy)
const legacy = THEMES.filter((th) => th.legacy)
const STARS: [number, number][] = [
  [10, 12],
  [28, 40],
  [46, 18],
  [64, 52],
  [82, 14],
  [90, 44],
  [18, 62],
]

const shown = computed(() => (settings.themeFollow ? settings.themeBase : settings.theme))

function readOpen(): boolean {
  try {
    return localStorage.getItem(VIEW_ALL_KEY) === '1'
  } catch {
    return false
  }
}

const isLegacy = (id: ThemeId): boolean => legacy.some((th) => th.id === id)
const open = ref(readOpen() || isLegacy(shown.value))
const ordered = computed(() => (open.value ? [...full, ...legacy] : full))

function toggleOpen(): void {
  open.value = !open.value
  try {
    localStorage.setItem(VIEW_ALL_KEY, open.value ? '1' : '0')
  } catch {
    return
  }
}

watch(shown, (id) => {
  if (isLegacy(id)) open.value = true
})
const modeValue = computed<'light' | 'dark' | 'system'>(() => (settings.themeFollow ? 'system' : modeOf(settings.theme)))
const modes = computed(() => [
  { value: 'light' as const, label: t('settings.themeLight') },
  { value: 'dark' as const, label: t('settings.themeDark') },
  { value: 'system' as const, label: t('themeV96.system') },
])

const name = (th: ThemeMeta): string => (th.nameKey ? t(th.nameKey) : th.name)
const modeLabel = (th: ThemeMeta): string => t(th.mode === 'light' ? 'settings.themeLight' : 'settings.themeDark')
const label = (th: ThemeMeta): string => `${name(th)}, ${modeLabel(th)}${th.isDefault ? `, ${t('settings.themeDefault')}` : ''}`

function pick(id: ThemeId): void {
  if (id !== shown.value) withViewTransition(() => settings.setTheme(id))
}

function setMode(mode: 'light' | 'dark' | 'system'): void {
  if (mode !== modeValue.value) withViewTransition(() => settings.setThemeMode(mode))
}

function onKey(e: KeyboardEvent, index: number): void {
  const list = ordered.value
  const n = list.length
  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key]
  const target = e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : step === undefined ? -1 : (index + step + n) % n
  if (target < 0) return
  e.preventDefault()
  pick(list[target]!.id)
  root.value?.querySelectorAll<HTMLElement>('[role=radio][data-theme-option]')[target]?.focus()
}
</script>

<template>
  <div class="flex min-w-0 flex-col gap-3">
    <SegmentedControl :model-value="modeValue" :options="modes" :label="t('themeV96.mode')" data-setting="theme-mode" @update:model-value="setMode" />

    <div ref="root" role="radiogroup" :aria-label="t('settings.theme')" class="flex min-w-0 flex-col gap-3">
      <div class="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-2" role="presentation">
        <button
          v-for="th in full"
          :key="th.id"
          type="button"
          role="radio"
          :aria-checked="shown === th.id"
          :aria-label="label(th)"
          :tabindex="shown === th.id ? 0 : -1"
          :data-theme-option="th.id"
          class="group flex min-w-0 cursor-pointer flex-col gap-2 rounded-card bg-surface p-2 text-left text-xs ring-1 ring-line transition-colors duration-(--ccm-dur-fast) outline-none hover:bg-fill focus-visible:ring-2 focus-visible:ring-focus aria-checked:ring-2 aria-checked:ring-accent"
          @click="pick(th.id)"
          @keydown="onKey($event, ordered.indexOf(th))"
        >
          <span class="relative block h-20 w-full overflow-hidden rounded-inner shadow-control" :style="{ background: th.swatch[0] }" aria-hidden="true">
            <span class="absolute inset-x-1 top-1 flex h-2.5 items-center justify-end rounded-[2px] px-1" :style="{ background: th.swatch[1] }">
              <span class="size-1.5 rounded-full" :style="{ background: th.swatch[2] }" />
            </span>
            <span class="absolute top-5 bottom-5 left-1 w-2 rounded-[2px]" :style="{ background: th.swatch[1] }" />
            <span class="absolute top-5 bottom-5 left-4 w-[calc(50%-1.25rem)] overflow-hidden rounded-[2px]" :style="{ background: th.swatch[3] }">
              <span class="block h-1" :style="{ background: th.agents![0] }" />
            </span>
            <span class="absolute top-5 right-1 bottom-5 left-[calc(50%+0.25rem)] overflow-hidden rounded-[2px]" :style="{ background: th.swatch[3] }">
              <span class="block h-1" :style="{ background: th.agents![1] }" />
            </span>
            <span class="absolute inset-x-1 bottom-1 h-2.5 rounded-[2px]" :style="{ background: th.swatch[1] }" />
            <template v-if="th.stars">
              <span v-for="(s, i) in STARS" :key="i" class="absolute size-0.5 rounded-full bg-white" :style="{ left: `${s[0]}%`, top: `${s[1]}%`, opacity: i % 2 ? 0.5 : 0.85 }" />
            </template>
          </span>
          <span class="flex min-w-0 items-center gap-1.5">
            <span class="min-w-0 flex-1 truncate text-sm font-semibold">{{ name(th) }}</span>
            <span class="shrink-0 text-2xs text-ink-faint">{{ modeLabel(th) }}</span>
            <span v-if="th.isDefault" class="shrink-0 rounded-full bg-accent-soft px-1.5 text-2xs text-accent-ink" data-theme-default>{{ t('settings.themeDefault') }}</span>
            <Check v-if="shown === th.id" :size="14" class="shrink-0 text-accent-ink" aria-hidden="true" />
          </span>
          <span class="text-2xs leading-snug text-ink-muted">{{ t(th.descKey!) }}</span>
          <span class="flex flex-wrap gap-x-2.5 gap-y-1" :aria-label="t('themeV96.agents')">
            <span v-for="(n, i) in ['Claude', 'Codex', 'Shell']" :key="n" class="inline-flex items-center gap-1 text-2xs text-ink-muted">
              <span class="size-2 rounded-full" :style="{ background: th.agents![i] }" aria-hidden="true" />{{ n }}
            </span>
          </span>
        </button>
      </div>

      <div v-if="open" role="presentation" data-theme-legacy>
        <p class="ccm-label mb-1.5 text-2xs font-semibold text-ink-faint" aria-hidden="true">{{ t('themeV96.legacyThemes') }}</p>
        <div class="flex flex-wrap gap-1.5" role="presentation">
          <button
            v-for="th in legacy"
            :key="th.id"
            type="button"
            role="radio"
            :aria-checked="shown === th.id"
            :aria-label="label(th)"
            :tabindex="shown === th.id ? 0 : -1"
            :data-theme-option="th.id"
            class="inline-flex min-w-0 cursor-pointer items-center gap-1.5 rounded-control px-2 py-1 text-xs ring-1 ring-line transition-colors duration-(--ccm-dur-fast) outline-none hover:bg-fill focus-visible:ring-2 focus-visible:ring-focus aria-checked:bg-accent-soft aria-checked:text-accent-ink aria-checked:ring-2 aria-checked:ring-accent"
            @click="pick(th.id)"
            @keydown="onKey($event, ordered.indexOf(th))"
          >
            <span class="relative inline-flex h-4 w-6 shrink-0 overflow-hidden rounded-[3px] shadow-control" :style="{ background: th.swatch[0] }" aria-hidden="true">
              <span class="absolute top-0.5 left-0.5 h-1.5 w-3 rounded-[1px]" :style="{ background: th.swatch[1] }" />
              <span class="absolute right-0.5 bottom-0.5 size-1.5 rounded-full" :style="{ background: th.swatch[2] }" />
            </span>
            <span class="truncate font-medium">{{ name(th) }}</span>
            <span v-if="th.pair" class="text-2xs text-ink-faint">{{ modeLabel(th) }}</span>
          </button>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2 border-t border-dashed border-line pt-2.5">
      <button
        type="button"
        class="inline-flex cursor-pointer items-center gap-1 rounded-control px-2.5 py-1 text-xs font-semibold ring-1 ring-line outline-none hover:bg-fill focus-visible:ring-2 focus-visible:ring-focus"
        :aria-expanded="open"
        data-theme-view-all
        @click="toggleOpen"
      >
        {{ open ? t('themeV96.viewLess') : t('themeV96.viewAll') }}
        <component :is="open ? ChevronUp : ChevronDown" :size="14" aria-hidden="true" />
      </button>
      <span v-if="!open" class="text-xs text-ink-muted">{{ t('themeV96.moreCount', { n: legacy.length }) }}</span>
    </div>
  </div>
</template>
