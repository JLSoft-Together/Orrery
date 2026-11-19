import { onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ActionResult } from './viewerActions'
import { ViewerError } from './viewerApi'

export interface Notice {
  kind: 'ok' | 'wait' | 'error'
  text: string
}

export function useAction() {
  const { t, te } = useI18n()
  const busy = ref(false)
  const notice = ref<Notice | null>(null)
  let timer: ReturnType<typeof setTimeout> | null = null

  function show(kind: Notice['kind'], text: string): void {
    notice.value = { kind, text }
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => (notice.value = null), 4500)
  }

  async function perform(job: () => Promise<ActionResult>, success?: string): Promise<ActionResult | null> {
    if (busy.value) return null
    busy.value = true
    try {
      const result = await job()
      if (result.kind === 'pending') show('wait', t('viewer.waitConfirm'))
      else if (result.kind === 'cancelled') show('error', t('err.cancelled'))
      else if (success) show('ok', success)
      return result
    } catch (error) {
      const code = error instanceof ViewerError ? error.code : 'internal'
      show('error', t(te(`err.${code}`) ? `err.${code}` : 'err.failed'))
      return null
    } finally {
      busy.value = false
    }
  }

  onBeforeUnmount(() => {
    if (timer) clearTimeout(timer)
  })

  return { busy, notice, perform, show }
}
