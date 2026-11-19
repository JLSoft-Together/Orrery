import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { readLook } from '../lib/appearance'
import { desktop } from '../lib/desktop'
import { fontEntry, fontStack } from '../lib/fonts'
import { applyTheme, modeOf } from '../lib/themes'
import {
  effectiveGlass,
  effectiveMotion,
  isNextMotion,
  isRailGroup,
  NEXT_KEYS,
  parseNextTheme,
  startUi,
  UI_GENERATION_KEY,
  UI_MIGRATION,
  UI_MIGRATION_KEY,
  type NextMotion,
  type RailGroup,
  type UiGeneration,
} from '../lib/uiGeneration'
import { isNextPaletteId, NEXT_PALETTE_CUSTOM, NEXT_PALETTE_DEFAULT, nextPalette, nextSchemeFor, type NextPaletteId, type NextScheme } from '@ccm/mist-tokens'
import {
  applyCustomVars,
  buildCustomTheme,
  parseCustomTheme,
  pushRecent,
  seedCustomTheme,
  serializeCustomTheme,
  type CustomThemeConfig,
} from '../lib/next/customTheme'
import { clampExplorerWidth, parseExplorerWidth } from '../lib/next/explorerPin'
import { useSettingsStore } from './settings'

function read(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function persist(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    return
  }
}

function media(query: string): MediaQueryList | null {
  return typeof matchMedia === 'function' ? matchMedia(query) : null
}

function initialGeneration(): UiGeneration {
  const start = startUi(read(UI_GENERATION_KEY), read(UI_MIGRATION_KEY))
  if (start.migrate) {
    persist(UI_GENERATION_KEY, start.ui)
    persist(UI_MIGRATION_KEY, UI_MIGRATION)
  }
  return start.ui
}

export const NEXT_UI_FONT = 'manrope'
export const NEXT_CODE_FONT = 'jetbrains-mono'

export const useUiGenStore = defineStore('uiGen', () => {
  const settings = useSettingsStore()
  const generation = ref<UiGeneration>(initialGeneration())
  const nextTheme = ref<NextPaletteId>(parseNextTheme(read(NEXT_KEYS.theme)))
  const railGroup = ref<RailGroup>(isRailGroup(read(NEXT_KEYS.railGroup)) ? (read(NEXT_KEYS.railGroup) as RailGroup) : 'project')
  const explorerPinned = ref(read(NEXT_KEYS.explorerPinned) === '1')
  const explorerWidth = ref(parseExplorerWidth(read(NEXT_KEYS.explorerWidth)))
  const reduceGlass = ref(read(NEXT_KEYS.reduceGlass) === '1')
  const clearTerm = ref(read(NEXT_KEYS.clearTerm) === '1')
  const motionChoice = ref<NextMotion | null>(isNextMotion(read(NEXT_KEYS.motion)) ? (read(NEXT_KEYS.motion) as NextMotion) : null)

  const motionMedia = media('(prefers-reduced-motion: reduce)')
  const glassMedia = media('(prefers-reduced-transparency: reduce)')
  const prefersReducedMotion = ref(motionMedia?.matches ?? false)
  const prefersReducedTransparency = ref(glassMedia?.matches ?? false)
  motionMedia?.addEventListener?.('change', (e) => (prefersReducedMotion.value = e.matches))
  glassMedia?.addEventListener?.('change', (e) => (prefersReducedTransparency.value = e.matches))

  const isNext = computed(() => generation.value === 'next')
  const palette = computed(() => nextPalette(nextTheme.value))
  const scheme = computed<NextScheme>(() => nextSchemeFor(nextTheme.value, modeOf(settings.theme)))
  const dark = computed(() => (isNext.value ? scheme.value === 'dark' : modeOf(settings.theme) === 'dark'))
  const motion = computed<NextMotion>(() => effectiveMotion(motionChoice.value, prefersReducedMotion.value))
  const glass = computed(() => effectiveGlass(reduceGlass.value, prefersReducedTransparency.value))
  const customConfig = ref<CustomThemeConfig | null>(parseCustomTheme(read(NEXT_KEYS.custom)))
  const custom = computed<CustomThemeConfig>(() => customConfig.value ?? seedCustomTheme(NEXT_PALETTE_DEFAULT))
  const customTheme = computed(() => buildCustomTheme(custom.value))
  const customScheme = computed(() => customTheme.value[scheme.value])
  const customActive = computed(() => isNext.value && nextTheme.value === NEXT_PALETTE_CUSTOM)

  watch(
    [generation, nextTheme, scheme, () => settings.theme],
    () => {
      const root = document.documentElement
      root.dataset.ui = generation.value
      if (isNext.value) {
        root.removeAttribute('data-theme')
        root.dataset.nextTheme = nextTheme.value
        root.classList.toggle('dark', scheme.value === 'dark')
      } else {
        delete root.dataset.nextTheme
        applyTheme(root, settings.theme)
      }
      const look = desktop?.setAppearance ? readLook() : null
      if (look) void desktop!.setAppearance!(look)
    },
    { flush: 'sync', immediate: true },
  )

  watch(
    [customActive, customScheme, glass],
    ([active, result, g]) => applyCustomVars(document.documentElement.style, active ? result.vars : null, g === 'reduced'),
    { flush: 'sync', immediate: true },
  )

  let customTimer: ReturnType<typeof setTimeout> | null = null
  watch(customConfig, (value) => {
    if (customTimer) clearTimeout(customTimer)
    customTimer = setTimeout(() => {
      customTimer = null
      if (value) persist(NEXT_KEYS.custom, serializeCustomTheme(value, customTheme.value))
    }, 200)
  })

  watch(motion, (value) => (document.documentElement.dataset.motion = value), { flush: 'sync', immediate: true })
  watch(
    glass,
    (value) => {
      if (value === 'reduced') document.documentElement.dataset.glass = 'reduced'
      else delete document.documentElement.dataset.glass
    },
    { flush: 'sync', immediate: true },
  )

  watch(
    [generation, () => settings.uiFont, () => settings.codeFont, () => settings.codeFontSize],
    ([gen, ui, code]) => {
      const style = document.documentElement.style
      const next = gen === 'next'
      style.setProperty('--font-sans', fontStack(fontEntry('ui', next && ui === 'system' ? NEXT_UI_FONT : ui)))
      style.setProperty('--font-mono', fontStack(fontEntry('code', next && code === 'system' ? NEXT_CODE_FONT : code)))
    },
    { immediate: true },
  )

  watch(generation, (value) => persist(UI_GENERATION_KEY, value))
  watch(nextTheme, (value) => persist(NEXT_KEYS.theme, value))
  watch(railGroup, (value) => persist(NEXT_KEYS.railGroup, value))
  watch(explorerPinned, (value) => persist(NEXT_KEYS.explorerPinned, value ? '1' : '0'))
  watch(explorerWidth, (value) => persist(NEXT_KEYS.explorerWidth, String(value)))
  watch(reduceGlass, (value) => persist(NEXT_KEYS.reduceGlass, value ? '1' : '0'))
  watch(clearTerm, (value) => persist(NEXT_KEYS.clearTerm, value ? '1' : '0'))

  function setExplorerWidth(value: number, room?: number): void {
    explorerWidth.value = clampExplorerWidth(value, room)
  }

  function setGeneration(value: UiGeneration): void {
    generation.value = value
  }

  function toggleGeneration(): void {
    generation.value = isNext.value ? 'classic' : 'next'
  }

  function setNextTheme(id: string): void {
    if (!isNextPaletteId(id)) return
    if (id === NEXT_PALETTE_CUSTOM && !customConfig.value) customConfig.value = seedCustomTheme(nextTheme.value)
    nextTheme.value = id
  }

  function setCustom(patch: Partial<Omit<CustomThemeConfig, 'v'>>): void {
    customConfig.value = { ...custom.value, ...patch }
  }

  function resetCustom(seed?: string): void {
    customConfig.value = seedCustomTheme(seed ?? custom.value.seed, custom.value.recent)
  }

  function addCustomRecent(hex: string): void {
    const recent = pushRecent(custom.value.recent, hex)
    if (recent.join() !== custom.value.recent.join()) setCustom({ recent })
  }

  function setMotion(value: NextMotion): void {
    motionChoice.value = value
    persist(NEXT_KEYS.motion, value)
  }

  return {
    generation,
    isNext,
    nextTheme,
    palette,
    scheme,
    dark,
    railGroup,
    explorerPinned,
    explorerWidth,
    setExplorerWidth,
    reduceGlass,
    clearTerm,
    motion,
    motionChoice,
    glass,
    setGeneration,
    toggleGeneration,
    setNextTheme,
    setMotion,
    custom,
    customTheme,
    customScheme,
    setCustom,
    resetCustom,
    addCustomRecent,
  }
})
