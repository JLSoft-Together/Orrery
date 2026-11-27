import { imageData } from '../../../core/lib/imageData'
import { useUiStore } from '../../../core/stores/ui'
import { useAttachmentsStore } from '../../agent-tools/stores/attachments'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { handoffPaneFor } from '../lib/runUi'
import { shotFileName } from '../lib/multiRunUi'

export type ShotsHandoff = 'ok' | 'no-agent' | 'failed'

export function useShotsHandoff(root: () => string) {
  const workspace = useWorkspaceStore()
  const attachments = useAttachmentsStore()
  const ui = useUiStore()

  async function attach(shots: readonly { png: string; device: string; variant: string }[]): Promise<ShotsHandoff> {
    const pane = handoffPaneFor(workspace.panes, root())
    if (!pane) return 'no-agent'
    const items = []
    for (const s of shots) {
      const blob = await fetch(s.png)
        .then((r) => r.blob())
        .catch(() => null)
      const img = blob ? await imageData(blob) : null
      if (!img) return 'failed'
      items.push({ kind: 'image' as const, name: shotFileName(s.device, s.variant), source: 'paste' as const, ...img })
    }
    if (!items.length) return 'failed'
    attachments.add(pane.key, items)
    workspace.activate(pane.key)
    if (ui.view !== 'workspace') ui.setView('workspace')
    return 'ok'
  }

  return { attach }
}
