import { useI18n } from 'vue-i18n'
import { isUnder, relPath, type Clash } from '../lib/clash'
import { UNDO_MS } from '../lib/next/railLive'
import { useAgentTouchesStore } from '../stores/agentTouches'
import { useTerminalsStore } from '../stores/terminals'
import { useToastsStore } from '../stores/toasts'
import { useUiStore } from '../stores/ui'
import { isTerm, useWorkspaceStore, workCwd } from '../stores/workspace'

export const clashToastId = (fileKey: string): string => `clash-${fileKey}`

export function useClashActions() {
  const { t } = useI18n()
  const workspace = useWorkspaceStore()
  const terminals = useTerminalsStore()
  const touches = useAgentTouchesStore()
  const toasts = useToastsStore()
  const ui = useUiStore()

  function cwdOf(terminalId: string): string | undefined {
    const pane = workspace.panes.find((p) => isTerm(p) && p.terminalId === terminalId)
    return (pane ? workCwd(pane) : undefined) ?? terminals.byId[terminalId]?.cwd
  }

  function rootFor(c: Clash, prefer?: string): string | undefined {
    const ids = [...(prefer ? [prefer] : []), ...c.peers.map((p) => p.terminalId)]
    for (const id of ids) {
      const cwd = cwdOf(id)
      if (cwd && isUnder(c.path, cwd)) return cwd
    }
    return undefined
  }

  function fileName(c: Clash, prefer?: string): string {
    return relPath(c.path, rootFor(c, prefer))
  }

  function viewDiff(c: Clash, prefer?: string): void {
    const root = rootFor(c, prefer)
    if (root) workspace.openChanges(root, { select: c.path })
    else workspace.openFile(c.path)
    ui.setView('workspace')
  }

  function ignore(c: Clash, prefer?: string): void {
    const file = fileName(c, prefer)
    touches.ignoreFile(c.fileKey)
    toasts.remove(clashToastId(c.fileKey))
    const id = `clash-undo-${c.fileKey}`
    toasts.push({
      id,
      kind: 'undo',
      text: t('v104.clash.ignored', { file }),
      timeoutMs: UNDO_MS,
      attrs: { 'data-clash-undo-toast': c.fileKey },
      actions: [
        {
          label: t('v104.clash.undo'),
          primary: true,
          attrs: { 'data-clash-undo': '' },
          run: () => {
            touches.unignoreFile(c.fileKey)
            toasts.remove(id)
          },
        },
      ],
    })
  }

  return { viewDiff, ignore, fileName }
}
