import { useI18n } from 'vue-i18n'
import { useToastsStore } from '../../../core/stores/toasts'
import { clipToastKey, sendClipboardToPhone } from '../lib/remoteClip'

export function useClipToPhone() {
  const { t } = useI18n()
  const toasts = useToastsStore()

  async function send(): Promise<void> {
    const res = await sendClipboardToPhone()
    toasts.push({
      id: 'clip-phone',
      kind: res.ok && res.sent > 0 ? 'info' : 'warning',
      text: t(clipToastKey(res)),
      timeoutMs: 4000,
    })
  }

  return { send }
}
