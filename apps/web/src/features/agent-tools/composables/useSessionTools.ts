import type { TerminalSession } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { toolsEntryVisible } from '../lib/sessionTools'
import { isCodex, isDeepseek } from '../../../core/lib/provider'
import { useUiStore } from '../../../core/stores/ui'
import { useWorkspaceStore } from '../../workspace/stores/workspace'

export function useSessionTools() {
  const ui = useUiStore()
  const workspace = useWorkspaceStore()

  const available = (term: TerminalSession): boolean =>
    toolsEntryVisible({ hasApi: !!desktop?.extProject, hasCwd: !!term.cwd, claude: !isCodex(term) && !isDeepseek(term), ended: term.status === 'stale' })

  function open(term: TerminalSession): void {
    if (!term.cwd) return
    const pane = workspace.panes.find((p) => p.terminalId === term.id)
    ui.sessionTools = { dir: term.cwd, ...(pane ? { paneKey: pane.key } : {}), ...(term.claudeSessionId ? { sessionId: term.claudeSessionId } : {}) }
  }

  return { available, open }
}
