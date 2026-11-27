import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { hasPushPhone, sendPaneToPhone, useRemoteStore, type SendPaneResult } from '@features/remote'
import { useToastsStore } from '../../../core/stores/toasts'

export function phoneToastKey(res: SendPaneResult): string {
  if (res.ok) return res.sent > 0 ? 'workspace.paneHead.phone.sent' : 'workspace.paneHead.phone.none'
  return `workspace.paneHead.phone.${res.error === 'invalid' ? 'unavailable' : res.error}`
}

export function usePanePhone(paneKey: () => string) {
  const { t } = useI18n()
  const remote = useRemoteStore()
  const toasts = useToastsStore()
  const available = computed(() => hasPushPhone(remote.devices))

  async function send(): Promise<void> {
    const res = await sendPaneToPhone(paneKey())
    const sent = res.ok ? res.sent : 0
    toasts.push({
      id: 'pane-phone',
      kind: res.ok && sent > 0 ? 'info' : 'warning',
      text: t(phoneToastKey(res), { n: sent }, sent),
      timeoutMs: 4000,
    })
  }

  return { available, send }
}
