import { onScopeDispose } from 'vue'
import { useI18n } from 'vue-i18n'
import { editorKeepsKey, inEditor } from '../../files/lib/fileEditor'
import { isMac } from '../../../core/lib/platform'
import { folderName } from '../../rail/lib/sidebarRows'
import { workspaceAction } from '../../workspace/lib/workspaceKeys'
import { useInboxStore } from '../stores/inbox'
import { useMenuQueueStore } from '../stores/menuQueue'
import { paneProjectOf, useWorkspaceStore } from '../../workspace/stores/workspace'
import { useMenuToast } from './useMenuToast'
import { usePaneInfo } from '../../workspace/composables/usePaneInfo'

export function inboxKeyAction(e: KeyboardEvent, mac: boolean): 'toggle' | 'next' | null {
  const a = workspaceAction(e, mac, false)
  return a?.t === 'attention' ? 'toggle' : a?.t === 'asked' ? 'next' : null
}

export function useInboxGlobal(): void {
  const { t } = useI18n()
  const inbox = useInboxStore()
  const queue = useMenuQueueStore()
  const workspace = useWorkspaceStore()
  const info = usePaneInfo()
  const paneOf = (k: string) => workspace.panes.find((p) => p.key === k)

  useMenuToast(() =>
    queue.entries.map((entry) => {
      const pane = paneOf(entry.paneKey)
      const tab = workspace.tabOf(entry.paneKey)
      const project = pane ? paneProjectOf(tab, pane, paneOf) : undefined
      return {
        paneKey: entry.paneKey,
        menu: entry.menu,
        question: entry.question,
        title: entry.question || t(`menuQueue.kind.${entry.menu.kind}`),
        place: [project ? folderName(project) : '', tab ? info.tabTitle(tab) : ''].filter(Boolean).join(', '),
      }
    }),
  )

  function onKey(e: KeyboardEvent): void {
    if (e.defaultPrevented) return
    const action = inboxKeyAction(e, isMac)
    if (!action) return
    if (inEditor(e.target) && editorKeepsKey(e, isMac)) return
    e.preventDefault()
    e.stopPropagation()
    if (action === 'toggle') inbox.toggle(document.querySelector<HTMLElement>('[data-inbox-chip]'))
    else inbox.goNext()
  }

  window.addEventListener('keydown', onKey, true)
  onScopeDispose(() => window.removeEventListener('keydown', onKey, true))
}
