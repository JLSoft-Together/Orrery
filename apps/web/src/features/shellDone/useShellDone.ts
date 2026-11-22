import { useI18n } from 'vue-i18n'
import type { PtyServerMessage } from '@ccm/shared'
import { useConnectionStore } from '../../app/stores/connection'
import { useSettingsStore } from '../../core/stores/settings'
import { useToastsStore } from '../../core/stores/toasts'
import { useUiStore } from '../../core/stores/ui'
import { useWorkspaceStore } from '../workspace/stores/workspace'
import { elapsedParts, shellDoneAction, shownCommand } from './shellDone'

export type CmdMessage = Extract<PtyServerMessage, { t: 'cmd' }>

export interface ShellDoneHost {
  paneKey: () => string
  paneTitle: () => string
  visible: () => boolean
}

const TOAST_MS = 10_000

export function useShellDone(host: ShellDoneHost) {
  const { t } = useI18n()
  const settings = useSettingsStore()
  const workspace = useWorkspaceStore()
  const connection = useConnectionStore()
  const toasts = useToastsStore()
  const ui = useUiStore()

  function elapsed(ms: number): string {
    const { h, m, s } = elapsedParts(ms)
    if (h) return t('shellDone.hm', { h, m })
    if (m) return t('shellDone.ms', { m, s })
    return t('shellDone.s', { s })
  }

  function title(code: number, ms: number, command: string): string {
    const head = t(code === 0 ? 'shellDone.okTitle' : 'shellDone.failTitle', { code, time: elapsed(ms) })
    const shown = shownCommand(command)
    return shown ? `${head}: ${shown}` : head
  }

  function onCmd(msg: CmdMessage): void {
    if (msg.phase !== 'end') return
    const key = host.paneKey()
    workspace.update(key, { shellLast: { code: msg.code, ms: msg.ms, command: msg.command } })
    const windowFocused = document.visibilityState === 'visible' && document.hasFocus()
    const action = shellDoneAction({
      ms: msg.ms,
      enabled: settings.shellIntegration && settings.shellDoneNotify,
      seconds: settings.shellDoneSeconds,
      paneSeen: host.visible() && workspace.activeKey === key,
      windowFocused,
    })
    if (action === 'none') return
    workspace.update(key, { attention: 'done' })
    if (action === 'os') {
      connection.shellDone({ paneKey: key, name: host.paneTitle(), command: msg.command, code: msg.code, ms: msg.ms })
      return
    }
    const id = `shell-done-${key}`
    toasts.push({
      id,
      kind: msg.code === 0 ? 'info' : 'warning',
      text: title(msg.code, msg.ms, msg.command),
      meta: host.paneTitle(),
      timeoutMs: TOAST_MS,
      attrs: { 'data-shell-done-toast': key, 'data-shell-done-exit': String(msg.code) },
      actions: [
        {
          label: t('shellDone.open'),
          attrs: { 'data-shell-done-open': '' },
          run: () => {
            toasts.remove(id)
            if (ui.view !== 'workspace') ui.setView('workspace')
            workspace.activate(key)
          },
        },
      ],
    })
  }

  return { onCmd, title, elapsed }
}
