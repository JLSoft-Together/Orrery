import { computed, nextTick, watch, type Component, type ComputedRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { BookText, Plus } from 'lucide-vue-next'
import { useUiStore } from '../../../core/stores/ui'
import { workCwd } from '../../workspace/stores/workspace'
import { paletteRows } from '../lib/palettePrompts'
import { paletteSeed, PROMPT_PREFIX } from '../lib/paletteSeed'
import { useSavedPromptsStore } from '../stores/savedPrompts'
import { usePromptInsert } from './useInsertPrompt'

export interface PalettePromptItem {
  id: string
  group: 'prompts'
  bucket?: undefined
  label: string
  exact: string
  hay: string
  sub?: string
  icon: Component
  run: () => void
}

const fold = (s: string): string =>
  s
    .toLowerCase()
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .replace(/đ/g, 'd')

const entry = (id: string, label: string, sub: string | undefined, icon: Component, run: () => void): PalettePromptItem => ({
  id,
  group: 'prompts',
  label,
  exact: label.toLowerCase(),
  hay: fold(label),
  sub,
  icon,
  run,
})

export function usePalettePrompts(query: () => string | null, setQuery?: (q: string) => void): ComputedRef<PalettePromptItem[]> {
  const { t } = useI18n()
  const store = useSavedPromptsStore()
  const ui = useUiStore()
  const inserter = usePromptInsert()

  watch(
    () => ui.paletteOpen,
    async (open) => {
      if (!open || !paletteSeed.value) return
      paletteSeed.value = false
      await nextTick()
      await nextTick()
      if (ui.paletteOpen) setQuery?.(PROMPT_PREFIX)
    },
  )

  watch(
    () => query() !== null,
    (on) => {
      if (on && !store.loaded) store.start()
    },
    { immediate: true },
  )

  const openSettings = (): void => {
    ui.settingsPage = 'workspace'
    ui.settingsOpen = true
  }

  return computed(() => {
    const text = query()
    if (text === null) return []
    const pane = inserter.focusedPane()
    const rows = paletteRows(store.items, text, pane ? workCwd(pane) : undefined)
    const out = rows.map((r) => entry(`pr:${r.prompt.id}`, r.prompt.title, r.hint, BookText, () => void inserter.insert(r.prompt)))
    if (!text) out.push(entry('pr:new', t('savedPrompts.palette.manage'), t('savedPrompts.palette.manageSub'), Plus, openSettings))
    return out
  })
}
