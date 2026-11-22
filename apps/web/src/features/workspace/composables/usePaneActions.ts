import { hasTelegram } from '@features/telegram'
import { runPaneAction, telegramMenuState, telegramTopicLink, type PaneActionDeps, type PaneActionHooks, type PaneActionSubject } from '../lib/paneActions'
import { useTelegramStore } from '@features/telegram'
import { useUiStore } from '../../../core/stores/ui'
import { useWorkspaceStore, type Pane } from '../stores/workspace'
import { useCompact } from '../../agent-tools/composables/useCompact'
import { openTelegramGlyph } from '@features/telegram'

/** Runner for the pane actions of the tab and rail context menus (lib/paneActions). */
export function usePaneActions() {
  const workspace = useWorkspaceStore()
  const ui = useUiStore()
  const telegram = useTelegramStore()
  const compactFlow = useCompact()

  const deps: PaneActionDeps = {
    compact: (key) => void compactFlow.start(key),
    compactCancel: (key) => compactFlow.cancel(key),
    telegramLink: (ctx) => (ui.telegramLink = ctx),
    telegramOpen: (pane, name) => openTelegramGlyph(pane, name),
    focus: (key) => workspace.activate(key),
    monitor: (terminalId) => {
      ui.setView('monitor')
      ui.focusTerminal(terminalId)
    },
  }

  const run = (id: string, subject: PaneActionSubject, hooks: PaneActionHooks = {}): boolean => runPaneAction(id, subject, hooks, deps)

  const telegramOf = (pane: Pane | undefined): 'linked' | 'unlinked' | undefined =>
    hasTelegram ? telegramMenuState(pane, telegram.state ?? undefined) : undefined

  const telegramOpenOf = (pane: Pane | undefined): boolean => hasTelegram && !!telegramTopicLink(pane, telegram.state ?? undefined)

  return { run, telegramOf, telegramOpenOf }
}
