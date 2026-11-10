import path from 'node:path'
import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { parseBuildClean, parseBuildRoots, type BuildCleanResult, type BuildProject } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import type { BuildCleanDeps, BuildCleanService } from './service'

export type BuildCleanDeskDeps = Pick<BuildCleanDeps, 'running' | 'daemons' | 'trash' | 'confirm' | 'texts'>

export interface BuildCleanIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  service: BuildCleanService
  desk?: { router: DeskRouter; deps: BuildCleanDeskDeps }
}

const CHANNELS = ['ccm:build-scan', 'ccm:build-clean', 'ccm:build-cancel']

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)
const num = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) ? v : 0)

export function buildCleanHosts(d: BuildCleanDeskDeps): HostHandlers {
  return {
    'buildClean.running': (p) => {
      const root = field(p, 'root')
      return typeof root === 'string' ? d.running(root) : []
    },
    'buildClean.daemons': () => d.daemons(),
    'buildClean.trash': async (p) => {
      const file = field(p, 'path')
      if (typeof file !== 'string' || !path.isAbsolute(file)) return false
      try {
        await d.trash(file)
        return true
      } catch {
        return false
      }
    },
    'buildClean.confirm': (p) => {
      const t = d.texts()
      const raw = field(p, 'lines')
      const lines = Array.isArray(raw) ? raw.filter((l): l is string => typeof l === 'string') : []
      const trash = field(p, 'trash') === true
      return d.confirm({
        message: t.title(num(field(p, 'dirs')), String(field(p, 'size') ?? '')),
        detail: t.detail(lines, num(field(p, 'more')), field(p, 'reinstall') === true, trash),
        buttons: [trash ? t.confirmTrash : t.confirm, t.cancel],
      })
    },
  }
}

export function registerBuildCleanIpc({ ipcMain, isAppFrame, service, desk }: BuildCleanIpcDeps): () => void {
  const hosts = desk ? buildCleanHosts(desk.deps) : {}
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk.router, module: 'buildClean', hosts: () => hosts } : undefined)
  handle('ccm:build-scan', (event, raw: unknown): Promise<BuildProject[] | null> | null => {
    const roots = isAppFrame(event) ? parseBuildRoots(raw) : null
    return roots ? service.scan(roots) : null
  })
  handle('ccm:build-clean', (event, raw: unknown): Promise<BuildCleanResult> | BuildCleanResult => {
    const req = isAppFrame(event) ? parseBuildClean(raw) : null
    return req ? service.clean(req) : { ok: false, error: 'invalid' }
  })
  handle('ccm:build-cancel', (event): boolean => (isAppFrame(event) ? service.cancel() : false))
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}
