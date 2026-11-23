import { watch } from 'vue'
import { desktop } from '../../core/lib/desktop'
import { useConnectionStore } from '../../app/stores/connection'
import { useInboxStore } from '@features/click-menu'
import { useTerminalsStore } from '../../core/stores/terminals'
import { useProjectNotifyStore } from './stores/projectNotify'
import { trayPayload } from './trayWaiting'

type TrayBridge = { trayWaiting?: (payload: unknown) => Promise<unknown> }

export function useTrayWaiting(): void {
  const bridge = desktop as unknown as TrayBridge | undefined
  if (typeof bridge?.trayWaiting !== 'function') return
  const inbox = useInboxStore()
  const terminals = useTerminalsStore()
  const projects = useProjectNotifyStore()
  const connection = useConnectionStore()
  let sent = ''

  watch(
    () => {
      if (!connection.hasData) return null
      return JSON.stringify(trayPayload(inbox.items, terminals.byId, (cwd) => projects.levelOf(cwd) !== 'off'))
    },
    (json) => {
      if (json === null || json === sent) return
      sent = json
      void bridge.trayWaiting?.(JSON.parse(json))?.catch(() => undefined)
    },
    { immediate: true },
  )
}
