import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { HooksListResult, HookTestResult, HookToggleResult } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import { checkDir } from '../extensions/ipc'
import type { HooksManager } from './manager'

export interface HooksIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  manager: HooksManager
  desk?: DeskRouter
}

const CHANNELS = ['ccm:hooks-list', 'ccm:hooks-toggle', 'ccm:hooks-test']

const str = (p: unknown, key: string): string => {
  const v = p && typeof p === 'object' ? (p as Record<string, unknown>)[key] : undefined
  return typeof v === 'string' ? v : ''
}

export function claudeHooksHosts(manager: HooksManager): HostHandlers {
  return {
    'claudeHooks.confirmWrite': (p) =>
      manager.deps.confirmWrite({ file: str(p, 'file'), action: str(p, 'action') === 'enable' ? 'enable' : 'disable', event: str(p, 'event'), diff: str(p, 'diff') }),
    'claudeHooks.confirmTest': (p) => manager.deps.confirmTest({ event: str(p, 'event'), text: str(p, 'text'), cwd: str(p, 'cwd') }),
  }
}

export function registerHooksIpc({ ipcMain, isAppFrame, manager, desk }: HooksIpcDeps): () => void {
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: 'claudeHooks', hosts: () => claudeHooksHosts(manager) } : undefined)
  handle('ccm:hooks-list', async (event, rawDir: unknown): Promise<HooksListResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'unavailable' }
    const dir = await checkDir(rawDir)
    if (dir === 'bad') return { ok: false, error: 'invalid' }
    return manager.list(dir)
  })
  handle('ccm:hooks-toggle', async (event, req: unknown): Promise<HookToggleResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'invalid' }
    const r = (req ?? {}) as { id?: unknown; dir?: unknown; on?: unknown }
    const dir = await checkDir(r.dir)
    if (dir === 'bad') return { ok: false, error: 'invalid' }
    return manager.toggle(r.id, dir, r.on)
  })
  handle('ccm:hooks-test', async (event, req: unknown): Promise<HookTestResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'invalid' }
    const r = (req ?? {}) as { id?: unknown; dir?: unknown }
    const dir = await checkDir(r.dir)
    if (dir === 'bad') return { ok: false, error: 'invalid' }
    return manager.test(r.id, dir)
  })
  return () => {
    for (const ch of CHANNELS) ipcMain.removeHandler(ch)
  }
}
