import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { readLook } from '../../../core/lib/appearance'
import { desktop } from '../../../core/lib/desktop'
import { fontEntry, fontStack } from '../../../core/lib/fonts'
import { applyTheme, modeOf } from '../../../core/lib/themes'
import {
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
} from '../../../core/lib/uiGeneration'
import { isNextPaletteId, NEXT_PALETTE_CUSTOM, NEXT_PALETTE_DEFAULT, NEXT_PALETTE_RANDOM, nextPalette, nextSchemeFor, type NextPaletteId, type NextScheme } from '@ccm/mist-tokens'
import {
  buildCustomTheme,
  parseCustomTheme,
  pushRecent,
  seedCustomTheme,
  serializeCustomTheme,
  type CustomThemeConfig,
} from '../lib/customTheme'
import { clampExplorerWidth, parseExplorerWidth } from '../lib/explorerPin'
import { mauSeed, mauTheme } from '../lib/mau'
import {
  applyInlineVars,
  dropRandomPin,
  freshRandomPalette,
  holdRandomPalette,
  launchRandomPalette,
  parseRandomPalette,
  pickRandomPin,
  reshuffleRandomPalette,
  serializeRandomPalette,
  type RandomPaletteState,
} from '../lib/randomPalette'
import { useSettingsStore } from '../../../core/stores/settings'

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
  const storedRandom = parseRandomPalette(read(NEXT_KEYS.random))
  const randomState = ref<RandomPaletteState | null>(nextTheme.value === NEXT_PALETTE_RANDOM ? launchRandomPalette(storedRandom, mauSeed) : storedRandom)
  const railGroup = ref<RailGroup>(isRailGroup(read(NEXT_KEYS.railGroup)) ? (read(NEXT_KEYS.railGroup) as RailGroup) : 'project')
  const explorerPinned = ref(read(NEXT_KEYS.explorerPinned) === '1')
  const explorerWidth = ref(parseExplorerWidth(read(NEXT_KEYS.explorerWidth)))
  const clearTerm = ref(read(NEXT_KEYS.clearTerm) === '1')
  const motionChoice = ref<NextMotion | null>(isNextMotion(read(NEXT_KEYS.motion)) ? (read(NEXT_KEYS.motion) as NextMotion) : null)

  const motionMedia = media('(prefers-reduced-motion: reduce)')
  const prefersReducedMotion = ref(motionMedia?.matches ?? false)
  motionMedia?.addEventListener?.('change', (e) => (prefersReducedMotion.value = e.matches))

  const isNext = computed(() => generation.value === 'next')
  const palette = computed(() => nextPalette(nextTheme.value))
  const scheme = computed<NextScheme>(() => nextSchemeFor(nextTheme.value, modeOf(settings.theme)))
  const dark = computed(() => (isNext.value ? scheme.value === 'dark' : modeOf(settings.theme) === 'dark'))
  const motion = computed<NextMotion>(() => effectiveMotion(motionChoice.value, prefersReducedMotion.value))
  const customConfig = ref<CustomThemeConfig | null>(parseCustomTheme(read(NEXT_KEYS.custom)))
  const custom = computed<CustomThemeConfig>(() => customConfig.value ?? seedCustomTheme(NEXT_PALETTE_DEFAULT))
  const customTheme = computed(() => buildCustomTheme(custom.value))
  const customScheme = computed(() => customTheme.value[scheme.value])
  const customActive = computed(() => isNext.value && nextTheme.value === NEXT_PALETTE_CUSTOM)
  const randomTheme = computed(() => (randomState.value ? mauTheme(randomState.value.seed) : null))
  const randomActive = computed(() => isNext.value && nextTheme.value === NEXT_PALETTE_RANDOM && randomTheme.value !== null)
  const inlineVars = computed<Readonly<Record<string, string>> | null>(() =>
    customActive.value ? customScheme.value.vars : randomActive.value ? randomTheme.value!.css[scheme.value] : null,
  )

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

  watch(inlineVars, (vars) => applyInlineVars(document.documentElement.style, vars, true), { flush: 'sync', immediate: true })

  watch(randomState, (value) => value && persist(NEXT_KEYS.random, serializeRandomPalette(value)), { immediate: true })

  let customTimer: ReturnType<typeof setTimeout> | null = null
  watch(customConfig, (value) => {
    if (customTimer) clearTimeout(customTimer)
    customTimer = setTimeout(() => {
      customTimer = null
      if (value) persist(NEXT_KEYS.custom, serializeCustomTheme(value, customTheme.value))
    }, 200)
  })

  watch(motion, (value) => (document.documentElement.dataset.motion = value), { flush: 'sync', immediate: true })
  document.documentElement.dataset.glass = 'reduced'

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
    if (id === NEXT_PALETTE_RANDOM && !randomState.value) randomState.value = freshRandomPalette(mauSeed)
    nextTheme.value = id
  }

  function reshuffleRandom(): void {
    randomState.value = reshuffleRandomPalette(randomState.value ?? freshRandomPalette(mauSeed), mauSeed)
    nextTheme.value = NEXT_PALETTE_RANDOM
  }

  function holdRandom(): void {
    if (randomState.value) randomState.value = holdRandomPalette(randomState.value)
  }

  function pickRandom(seed: number): void {
    if (!randomState.value) return
    randomState.value = pickRandomPin(randomState.value, seed)
    nextTheme.value = NEXT_PALETTE_RANDOM
  }

  function dropRandom(seed: number): void {
    if (randomState.value) randomState.value = dropRandomPin(randomState.value, seed)
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
    clearTerm,
    motion,
    motionChoice,
    setGeneration,
    toggleGeneration,
    setNextTheme,
    setMotion,
    custom,
    customTheme,
    customScheme,
    inlineVars,
    random: randomState,
    randomTheme,
    reshuffleRandom,
    holdRandom,
    pickRandom,
    dropRandom,
    setCustom,
    resetCustom,
    addCustomRecent,
  }
})
