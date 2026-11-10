import path from 'node:path'
import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { isAwakeMode, type AwakeState, type CaptureShotResult, type CaptureStatus } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import type { KeepAwake } from '../power/awake'
import { cleanDir, isInside, parsePatch, parseShotRequest, SHOT_SUB } from './files'
import type { CaptureFolder } from './folder'

export interface CaptureIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  folder: CaptureFolder
  /** Native folder dialog; null = cancelled. */
  pickFolder: (event: IpcMainInvokeEvent, near: string | null) => Promise<string | null>
  openFolder: (dir: string) => Promise<boolean>
  awake: KeepAwake
  setAwakeMode: (mode: Parameters<KeepAwake['setMode']>[0]) => void
  missing?: (dir: string) => void
  held?: () => string[]
  desk?: DeskRouter
}

const CHANNELS = ['ccm:capture-get', 'ccm:capture-pick', 'ccm:capture-set', 'ccm:capture-shot', 'ccm:capture-open', 'ccm:awake-state', 'ccm:awake-mode']

const str = (p: unknown, key: string): string => {
  const v = p && typeof p === 'object' ? (p as Record<string, unknown>)[key] : undefined
  return typeof v === 'string' ? v : ''
}

export function captureHosts(deps: Pick<CaptureIpcDeps, 'pickFolder' | 'openFolder' | 'missing' | 'held'>, event: IpcMainInvokeEvent): HostHandlers {
  return {
    'capture.pick': (p) => deps.pickFolder(event, str(p, 'near') || null),
    'capture.open': (p) => deps.openFolder(str(p, 'dir')),
    'capture.missing': (p) => deps.missing?.(str(p, 'dir')) ?? null,
    'capture.held': () => deps.held?.() ?? [],
  }
}

function synced(ipcMain: IpcMain, folder: CaptureFolder, desk: DeskRouter | undefined): IpcMain {
  if (!desk) return ipcMain
  return {
    handle: (channel: string, fn: (event: IpcMainInvokeEvent, ...args: unknown[]) => unknown) =>
      ipcMain.handle(channel, async (event, ...args) => {
        try {
          return await fn(event, ...args)
        } finally {
          if (desk.mode('capture') === 'rust') folder.reload()
        }
      }),
  } as unknown as IpcMain
}

export function registerCaptureIpc(deps: CaptureIpcDeps): () => void {
  const { ipcMain, isAppFrame, folder, pickFolder, openFolder, awake, setAwakeMode, desk } = deps
  const handle = deskHandle(synced(ipcMain, folder, desk), isAppFrame, desk ? { router: desk, module: 'capture', hosts: (event) => captureHosts(deps, event) } : undefined)
  const power = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: 'power' } : undefined)
  handle('ccm:capture-get', (event): CaptureStatus | null => (isAppFrame(event) ? folder.status() : null))
  handle('ccm:capture-pick', async (event): Promise<CaptureStatus | null> => {
    if (!isAppFrame(event)) return null
    const dir = cleanDir(await pickFolder(event, folder.config().dir))
    return dir ? folder.set({ dir }) : folder.status()
  })
  handle('ccm:capture-set', (event, raw: unknown): CaptureStatus | null => {
    const patch = isAppFrame(event) ? parsePatch(raw) : null
    return patch ? folder.set(patch) : null
  })
  handle('ccm:capture-shot', async (event, raw: unknown): Promise<CaptureShotResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'invalid' }
    const req = parseShotRequest(raw)
    if (!req) return { ok: false, error: 'invalid' }
    const dir = folder.active()
    if (!dir) return { ok: false, error: 'no-dir' }
    const file = await folder.writeShot(req.png, req.device, req.frame)
    return file && isInside(path.join(dir, SHOT_SUB), file) ? { ok: true, path: file } : { ok: false, error: 'failed' }
  })
  handle('ccm:capture-open', async (event): Promise<boolean> => {
    const dir = isAppFrame(event) ? folder.active() : null
    return dir ? openFolder(dir) : false
  })
  power('ccm:awake-state', (event): AwakeState | null => (isAppFrame(event) ? awake.state() : null))
  power('ccm:awake-mode', (event, mode: unknown): AwakeState | null => {
    if (!isAppFrame(event) || !isAwakeMode(mode)) return null
    setAwakeMode(mode)
    return awake.state()
  })
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}
