import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { BookText } from 'lucide-vue-next'
import { useUiStore } from '../../../core/stores/ui'
import { paletteSeed } from '../lib/paletteSeed'

export const PROMPTS_MENU_ID = 'prompts'

export function usePromptsMenu() {
  const { t } = useI18n()
  const ui = useUiStore()
  const entry = computed(() => ({ id: PROMPTS_MENU_ID, label: t('savedPrompts.title'), icon: BookText }))

  function pick(id: string): boolean {
    if (id !== PROMPTS_MENU_ID) return false
    paletteSeed.value = true
    ui.paletteOpen = true
    return true
  }

  return { entry, pick }
}
