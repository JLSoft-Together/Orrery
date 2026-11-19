import type { IpcMain, IpcMainInvokeEvent, MessageBoxOptions } from 'electron'
import type { AppLeftoversResult, AppUninstallResult, DiskDeleteResult, InstalledAppsResult } from '@ccm/shared'
import type { DeleteAsk } from '../diskMap/delete'
import { DiskAppsService, type DiskAppsDeps } from './service'

export const DISKAPPS_CHANNELS = ['ccm:diskapps-list', 'ccm:diskapps-uninstall', 'ccm:diskapps-leftovers', 'ccm:diskapps-delete', 'ccm:diskapps-settings'] as const

export interface DiskAppsIpcOptions extends Omit<DiskAppsDeps, 'confirm' | 'confirmDelete'> {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  confirm: (event: IpcMainInvokeEvent, opts: MessageBoxOptions) => Promise<boolean>
  confirmDelete: (event: IpcMainInvokeEvent, ask: DeleteAsk, second: boolean) => Promise<boolean>
  openSettings: () => Promise<boolean>
}

export interface DiskApps {
  service: DiskAppsService
  dispose: () => void
}

export function createDiskApps(opts: DiskAppsIpcOptions): DiskApps {
  let current: IpcMainInvokeEvent | null = null
  const service = new DiskAppsService({
    ...opts,
    confirm: (box) => (current ? opts.confirm(current, box) : Promise.resolve(false)),
    confirmDelete: (ask, second) => (current ? opts.confirmDelete(current, ask, second) : Promise.resolve(false)),
  })
  const { ipcMain, isAppFrame } = opts
  const guard = <T>(fallback: T, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<T> | T) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!isAppFrame(event)) return fallback
    try {
      return await fn(event, arg)
    } catch {
      return fallback
    }
  }
  const withEvent = async <T>(event: IpcMainInvokeEvent, fn: () => Promise<T>): Promise<T> => {
    current = event
    try {
      return await fn()
    } finally {
      current = null
    }
  }
  const emptyList: InstalledAppsResult = { apps: [], removed: [], ms: 0, ok: false, storeApps: 'none' }
  const nothing: DiskDeleteResult = { done: [], failed: [], freed: 0, permanent: false, cancelled: true }
  ipcMain.handle('ccm:diskapps-list', guard<InstalledAppsResult>(emptyList, () => service.list()))
  ipcMain.handle('ccm:diskapps-uninstall', guard<AppUninstallResult>({ ok: false, error: 'failed' }, (e, raw) => withEvent(e, () => service.uninstall((raw as { id?: unknown } | null)?.id))))
  ipcMain.handle('ccm:diskapps-leftovers', guard<AppLeftoversResult | null>(null, (_e, raw) => service.scanLeftovers((raw as { id?: unknown } | null)?.id)))
  ipcMain.handle('ccm:diskapps-delete', guard<DiskDeleteResult>(nothing, (e, raw) => withEvent(e, () => service.deleteLeftovers(raw))))
  ipcMain.handle('ccm:diskapps-settings', guard<boolean>(false, () => (opts.platform === 'win32' ? opts.openSettings() : false)))
  return {
    service,
    dispose: () => {
      for (const c of DISKAPPS_CHANNELS) ipcMain.removeHandler(c)
    },
  }
}
