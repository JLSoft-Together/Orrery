import type { ToastHealth } from '@ccm/shared'
import { hideToast, showToast, toastHealth, toastTag, type ToastContent } from '../../collector/src/toast'

export interface ToastDrawer {
  show(content: ToastContent): Promise<boolean>
  hide(tag: string): Promise<boolean>
  health(): Promise<ToastHealth>
}

export const windowsToasts: ToastDrawer = { show: showToast, hide: hideToast, health: toastHealth }

const MAX_TEXT = 4096

const text = (v: unknown, max = MAX_TEXT): v is string => typeof v === 'string' && v.length <= max

export function parseFullToast(data: unknown): ToastContent | null {
  if (typeof data !== 'object' || data === null) return null
  const m = data as Record<string, unknown>
  if (m.type !== 'toast.show' || !text(m.tag, 200) || !m.tag || typeof m.persistent !== 'boolean') return null
  if (!text(m.title) || !text(m.body) || !text(m.url) || !text(m.open, 200) || !text(m.dismiss, 200)) return null
  if (m.snoozeUrl !== undefined && !text(m.snoozeUrl)) return null
  if (m.snoozeLabel !== undefined && !text(m.snoozeLabel, 200)) return null
  const snooze = typeof m.snoozeUrl === 'string' && typeof m.snoozeLabel === 'string' ? { snoozeUrl: m.snoozeUrl, snoozeLabel: m.snoozeLabel } : {}
  return { tag: m.tag, title: m.title, body: m.body, url: m.url, open: m.open, dismiss: m.dismiss, persistent: m.persistent, ...snooze }
}

export interface ServeToasts {
  refreshHealth(): void
  handle(data: unknown): boolean
}

export function createServeToasts(post: (message: object) => void, drawer: ToastDrawer = windowsToasts): ServeToasts {
  const testTag = toastTag('test')
  let queue: Promise<unknown> = Promise.resolve()
  const run = (job: () => Promise<unknown>): void => {
    queue = queue.then(job, job).catch(() => undefined)
  }
  const refreshHealth = (): void => run(async () => post({ type: 'toast.health', health: await drawer.health() }))
  return {
    refreshHealth,
    handle(data: unknown): boolean {
      const m = data as { type?: unknown; tag?: unknown } | null
      if (m?.type === 'toast.hide' && text(m.tag, 200) && m.tag) {
        const tag = m.tag
        run(() => drawer.hide(tag))
        return true
      }
      const content = parseFullToast(data)
      if (!content) return false
      run(() => drawer.show(content))
      if (content.tag === testTag) refreshHealth()
      return true
    },
  }
}
