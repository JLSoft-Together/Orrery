import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { WorkflowStatusEvent } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import { AgentStatusWatch } from './watch'

export interface AgentStatusIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  send: (event: WorkflowStatusEvent) => void
  desk?: DeskRouter
}

const CHANNEL = 'ccm:agent-status-watch'
const KEY_MAX = 128

/** The page reports each agent pane's working folder; main answers with sanitized status pushes (`ccm:agent-status`). */
export function registerAgentStatusIpc({ ipcMain, isAppFrame, send, desk }: AgentStatusIpcDeps): () => void {
  const watch = new AgentStatusWatch((key, status) => send({ key, status }))
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: 'agentStatus' } : undefined)
  handle(CHANNEL, (event, key: unknown, dir: unknown) => {
    if (!isAppFrame(event) || typeof key !== 'string' || !key || key.length > KEY_MAX) return false
    watch.set(key, typeof dir === 'string' ? dir : null)
    return true
  })
  return () => {
    ipcMain.removeHandler(CHANNEL)
    watch.dispose()
  }
}
