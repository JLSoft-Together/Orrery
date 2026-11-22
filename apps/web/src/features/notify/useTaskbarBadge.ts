import { watch } from 'vue'
import { inQuietHours } from '@ccm/shared'
import { desktop } from '../../core/lib/desktop'
import { useConnectionStore } from '../../app/stores/connection'
import { useExtrasStore } from '../../core/stores/extras'
import { useInboxStore } from '@features/click-menu'
import { useSettingsStore } from '../../core/stores/settings'
import { badgeCount, hasFresh, needIds, shouldFlash } from './badge'

export function useTaskbarBadge(): void {
  if (typeof desktop?.taskbarBadge !== 'function') return
  const inbox = useInboxStore()
  const settings = useSettingsStore()
  const connection = useConnectionStore()
  const extras = useExtrasStore()
  let prev = new Set<string>()
  let primed = false
  let sent = -1

  function send(count: number, flash: boolean): void {
    if (count === sent && !flash) return
    sent = count
    void desktop?.taskbarBadge?.(count, flash)?.catch(() => undefined)
  }

  watch(
    () => [needIds(inbox.items).join('|'), settings.taskbarBadge, connection.hasData] as const,
    ([, enabled, hasData]) => {
      if (!hasData) return
      if (!enabled) {
        send(0, false)
        prev = new Set()
        primed = false
        return
      }
      const ids = needIds(inbox.items)
      const fresh = primed && hasFresh(prev, ids)
      prev = new Set(ids)
      primed = true
      const focused = document.visibilityState === 'visible' && document.hasFocus()
      const quiet = inQuietHours(extras.notify?.quiet, new Date())
      send(badgeCount(inbox.items), shouldFlash({ enabled, fresh, focused, quiet }))
    },
    { immediate: true },
  )
}
