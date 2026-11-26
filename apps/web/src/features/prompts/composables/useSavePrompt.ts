import { useI18n } from 'vue-i18n'
import { useToastsStore } from '../../../core/stores/toasts'
import { autoTitle, MAX_BODY, MAX_TITLE, normalizeBody, SCOPE_ALL } from '../lib/promptModel'
import { useSavedPromptsStore } from '../stores/savedPrompts'

export type SaveTextOutcome = 'saved' | 'exists' | 'failed'

export function usePromptSave() {
  const { t } = useI18n()
  const store = useSavedPromptsStore()
  const toasts = useToastsStore()

  const say = (id: string, text: string): void => {
    toasts.push({ id: `saved-prompt-${id}`, kind: 'info', text, timeoutMs: 4000 })
  }

  async function saveText(raw: string): Promise<SaveTextOutcome> {
    const body = normalizeBody(raw).slice(0, MAX_BODY)
    if (!store.loaded) await store.load()
    if (store.items.some((p) => p.body === body)) {
      say('exists', t('savedPrompts.history.exists'))
      return 'exists'
    }
    const res = await store.save({ title: autoTitle(body).slice(0, MAX_TITLE) || t('savedPrompts.title'), body, scope: SCOPE_ALL })
    say(res.ok ? 'saved' : 'failed', res.ok ? t('savedPrompts.history.saved') : t(`savedPrompts.error.${res.error}`))
    return res.ok ? 'saved' : 'failed'
  }

  return { saveText, supported: store.supported }
}
