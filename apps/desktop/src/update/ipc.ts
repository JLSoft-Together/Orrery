import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { UpdateManager } from './manager'

export interface UpdateIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  manager: UpdateManager
  desk?: DeskRouter
}

const CHANNELS = [
  'ccm:update-state',
  'ccm:update-check',
  'ccm:update-download',
  'ccm:update-cancel',
  'ccm:update-install',
  'ccm:update-skip',
  'ccm:update-auto',
  'ccm:update-page',
  'ccm:update-reveal',
]

/** The page only names actions; URLs, files and versions all come from the manager's own state. */
export function registerUpdateIpc(deps: UpdateIpcDeps): () => void {
  const { ipcMain, isAppFrame, manager } = deps
  const handle = deskHandle(ipcMain, isAppFrame, deps.desk ? { router: deps.desk, module: 'update' } : undefined)
  const gate =
    <A extends unknown[]>(fn: (...args: A) => unknown) =>
    (event: IpcMainInvokeEvent, ...args: A): unknown =>
      isAppFrame(event) ? fn(...args) : null

  handle('ccm:update-state', gate(() => manager.state()))
  handle('ccm:update-check', gate(() => manager.check()))
  handle('ccm:update-download', gate(() => manager.download()))
  handle('ccm:update-cancel', gate(() => manager.cancel()))
  handle('ccm:update-install', gate(() => manager.install()))
  handle('ccm:update-skip', gate((version: unknown) => manager.skip(version)))
  handle('ccm:update-auto', gate((on: unknown) => (typeof on === 'boolean' ? manager.setAutoCheck(on) : manager.state())))
  handle('ccm:update-page', gate(() => manager.openPage()))
  handle('ccm:update-reveal', gate(() => manager.revealFile()))

  return () => {
    for (const ch of CHANNELS) ipcMain.removeHandler(ch)
  }
}
