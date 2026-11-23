import { onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, CircleX } from 'lucide-vue-next'
import type { GhCiDoneEvent } from '@ccm/shared'
import { onGhEvent, openExternal } from '../../../core/lib/preview'
import { useSettingsStore } from '../../../core/stores/settings'
import { useToastsStore } from '../../../core/stores/toasts'
import { useCiStore } from '../stores/ci'
import { toastParts } from '../lib/prView'

const GITHUB = 'https://github.com/'

let owners = 0
let stop: (() => void) | null = null

export function useCiEvents(): void {
  const { t } = useI18n()
  const toasts = useToastsStore()
  const settings = useSettingsStore()
  const ci = useCiStore()

  function show(ev: GhCiDoneEvent): void {
    const parts = toastParts(ev)
    if (!parts) return
    const failed = ev.failed.slice(0, 3).join(', ')
    const title = t(parts.title)
    const body = t(parts.body, { branch: ev.branch, failed })
    const url = ev.url.startsWith(GITHUB) ? ev.url : null
    toasts.push({
      id: `ci-done-${ev.root}-${ev.branch}`,
      kind: 'info',
      title,
      text: body,
      icon: parts.kind === 'ok' ? CircleCheck : parts.kind === 'bad' ? CircleX : CircleAlert,
      timeoutMs: parts.kind === 'bad' ? 12_000 : 8_000,
      attrs: { 'data-ci-toast': ev.state },
      actions: url ? [{ label: t('v111pr.toast.open'), run: () => void openExternal(url), primary: true }] : undefined,
    })
    if (!settings.notifyEnabled || typeof Notification === 'undefined' || Notification.permission !== 'granted') return
    if (document.visibilityState === 'visible' && document.hasFocus()) return
    try {
      const n = new Notification(title, { body, tag: `ci|${ev.root}|${ev.branch}`, icon: '/favicon.svg' })
      n.onclick = () => {
        window.focus()
        if (url) void openExternal(url)
        n.close()
      }
    } catch {
      return
    }
  }

  if (owners++ === 0) {
    void ci.loadWatches()
    stop = onGhEvent((ev) => {
      if (ev.timedOut) ci.clear(ev.root)
      else ci.set(ev.root, ev.branch, ev.state)
      show(ev)
    })
  }
  onBeforeUnmount(() => {
    if (--owners > 0) return
    owners = 0
    stop?.()
    stop = null
  })
}
