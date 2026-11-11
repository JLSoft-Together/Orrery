import { onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { CRASH_PROMPT_DELAY_MS, createUiErrorGate, diffCrashes, readSeen, writeSeen } from '../lib/bugReport'
import { desktop } from '../lib/desktop'
import { onUncaughtError } from '../lib/errorLog'
import { useSettingsStore } from '../stores/settings'
import { useToastsStore } from '../stores/toasts'
import { useUiStore } from '../stores/ui'

const TOAST_ID = 'bug-report'
const TOAST_MS = 12_000

function storage(): Storage | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

export function scrollToBugReport(): void {
  const started = Date.now()
  const tick = (): void => {
    const el = document.querySelector<HTMLElement>('[data-bug-report]')
    if (el) {
      el.scrollIntoView({ block: 'center' })
      return
    }
    if (Date.now() - started < 1500) window.requestAnimationFrame(tick)
  }
  window.requestAnimationFrame(tick)
}

export function useBugReportPrompts(): void {
  const { t } = useI18n()
  const toasts = useToastsStore()
  const ui = useUiStore()
  const settings = useSettingsStore()
  const uiErrorGate = createUiErrorGate(() => Date.now())
  let timer: number | undefined
  let offUncaught: (() => void) | null = null

  function show(text: string): void {
    toasts.push({
      id: TOAST_ID,
      kind: 'warning',
      title: t('bugReport.toast.title'),
      text,
      timeoutMs: TOAST_MS,
      attrs: { 'data-bug-report-toast': '' },
      actions: [
        {
          label: t('bugReport.toast.action'),
          primary: true,
          attrs: { 'data-bug-report-open': '' },
          run: () => {
            toasts.dismiss(TOAST_ID, 'close')
            ui.settingsPage = 'about'
            ui.settingsOpen = true
            scrollToBugReport()
          },
        },
      ],
    })
  }

  async function checkPreviousCrash(): Promise<void> {
    const preview = desktop?.bugReportPreview
    if (!preview) return
    const res = await preview().catch(() => null)
    if (!res?.ok) return
    const store = storage()
    const diff = diffCrashes(res.items, readSeen(store))
    writeSeen(store, diff.next)
    if (diff.fresh.length && settings.bugReportNudge) show(t('bugReport.toast.crash'))
  }

  onMounted(() => {
    timer = window.setTimeout(() => void checkPreviousCrash(), CRASH_PROMPT_DELAY_MS)
    offUncaught = onUncaughtError((e) => {
      if (!desktop?.bugReportSave || !settings.bugReportNudge) return
      if (uiErrorGate(e.message)) show(t('bugReport.toast.ui'))
    })
  })
  onBeforeUnmount(() => {
    window.clearTimeout(timer)
    offUncaught?.()
  })
}
