import { useTerminalsStore } from '../stores/terminals'
import { useTurnEditsStore } from '../stores/turnEdits'
import { useUiStore } from '../stores/ui'
import { isTerm, useWorkspaceStore, workCwd } from '../stores/workspace'

export function useTurnChanges() {
  const workspace = useWorkspaceStore()
  const terminals = useTerminalsStore()
  const turns = useTurnEditsStore()
  const ui = useUiStore()

  function cwdOf(terminalId: string): string | undefined {
    const pane = workspace.panes.find((p) => isTerm(p) && p.terminalId === terminalId)
    return (pane ? workCwd(pane) : undefined) ?? terminals.byId[terminalId]?.cwd
  }

  const has = (terminalId: string | undefined): boolean => !!terminalId && turns.countOf(terminalId) > 0

  function open(terminalId: string, cwd = cwdOf(terminalId)): boolean {
    if (!cwd) return false
    workspace.openChanges(cwd, has(terminalId) ? { session: terminalId } : {})
    ui.setView('workspace')
    return true
  }

  return { has, open, cwdOf }
}
