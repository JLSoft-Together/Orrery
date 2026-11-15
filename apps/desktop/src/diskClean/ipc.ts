import type { IpcMain, IpcMainInvokeEvent, MessageBoxOptions } from 'electron'
import { parseCleanDelete, parseCleanScan, type CleanBinResult, type CleanCategory, type CleanScanResult, type DiskDeleteResult } from '@ccm/shared'
import type { DeleteAsk } from '../diskMap/delete'
import { binBoxOptions, oneTapBoxOptions, oneTapChoice } from './dialog'
import { DiskCleanService, type DiskCleanDeps } from './service'

export const DISKCLEAN_CHANNELS = ['ccm:diskclean-categories', 'ccm:diskclean-scan', 'ccm:diskclean-cancel', 'ccm:diskclean-delete', 'ccm:diskclean-empty-bin'] as const
export const DISKCLEAN_PROGRESS = 'ccm:diskclean-progress'

export interface DiskCleanIpcOptions extends Omit<DiskCleanDeps, 'confirm' | 'emit' | 'askOneTap' | 'askBin'> {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  broadcast: (channel: string, payload: unknown) => void
  confirm: (event: IpcMainInvokeEvent, ask: DeleteAsk, second: boolean) => Promise<boolean>
  autoConfirm?: boolean
  ask?: (event: IpcMainInvokeEvent, opts: MessageBoxOptions) => Promise<number>
  locale?: () => string
}

export interface DiskClean {
  service: DiskCleanService
  dispose: () => void
}

export function createDiskClean(opts: DiskCleanIpcOptions): DiskClean {
  let current: IpcMainInvokeEvent | null = null
  const service = new DiskCleanService({
    ...opts,
    emit: (p) => opts.broadcast(DISKCLEAN_PROGRESS, p),
    confirm: (ask, second) => (opts.autoConfirm ? Promise.resolve(true) : current ? opts.confirm(current, ask, second) : Promise.resolve(false)),
    askOneTap: async (ask) => {
      if (opts.autoConfirm) return 'clean'
      if (!current || !opts.ask) return 'cancel'
      return oneTapChoice(await opts.ask(current, oneTapBoxOptions(ask, opts.locale?.() ?? 'en', opts.platform === 'darwin')), ask)
    },
    askBin: async (ask) => {
      if (opts.autoConfirm) return true
      if (!current || !opts.ask) return false
      return (await opts.ask(current, binBoxOptions(ask, opts.locale?.() ?? 'en', opts.platform === 'darwin'))) === 0
    },
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
  const nothing: DiskDeleteResult = { done: [], failed: [], freed: 0, permanent: false, cancelled: true }
  ipcMain.handle('ccm:diskclean-categories', guard<CleanCategory[]>([], () => service.categories()))
  ipcMain.handle(
    'ccm:diskclean-scan',
    guard<CleanScanResult>({ items: [] }, (_e, raw) => {
      const req = parseCleanScan(raw)
      return req ? service.scan(req) : { items: [] }
    }),
  )
  ipcMain.handle('ccm:diskclean-cancel', guard(false, () => service.cancel()))
  ipcMain.handle(
    'ccm:diskclean-delete',
    guard<DiskDeleteResult>(nothing, async (event, raw) => {
      const req = parseCleanDelete(raw)
      if (!req) return nothing
      current = event
      try {
        return await service.remove(req)
      } finally {
        current = null
      }
    }),
  )
  ipcMain.handle(
    'ccm:diskclean-empty-bin',
    guard<CleanBinResult>({ done: false, freed: 0, cancelled: true }, async (event) => {
      current = event
      try {
        return await service.emptyBin()
      } finally {
        current = null
      }
    }),
  )
  return {
    service,
    dispose: () => {
      for (const c of DISKCLEAN_CHANNELS) ipcMain.removeHandler(c)
    },
  }
}
