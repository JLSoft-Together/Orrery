import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { OrganizeJunkResult, OrganizePlan, OrganizeRunResult, OrganizeState, OrganizeUndoResult } from '@ccm/shared'
import { OrganizeService, type OrganizeDeps } from './service'

export const ORGANIZE_CHANNELS = [
  'ccm:organize-state', 'ccm:organize-settings', 'ccm:organize-add-source', 'ccm:organize-remove-source',
  'ccm:organize-plan', 'ccm:organize-run', 'ccm:organize-undo', 'ccm:organize-junk',
] as const
export const ORGANIZE_CHANGED = 'ccm:organize-changed'

export interface OrganizeOptions extends Omit<OrganizeDeps, 'broadcast'> {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  pickFolder: (event: IpcMainInvokeEvent) => Promise<string | null>
  broadcast: (channel: string, payload: unknown) => void
}

export interface Organize {
  service: OrganizeService
  dispose: () => void
}

export function e2eStableMs(env: NodeJS.ProcessEnv, packaged: boolean): number | undefined {
  if (packaged) return undefined
  const n = Number(env.CCM_E2E_ORGANIZE_STABLE_MS)
  return Number.isFinite(n) && n >= 0 && env.CCM_E2E_ORGANIZE_STABLE_MS ? n : undefined
}

export function e2ePath(env: NodeJS.ProcessEnv, packaged: boolean, name: 'DOWNLOADS' | 'DESKTOP' | 'PICK'): string | null {
  if (packaged) return null
  return env[`CCM_E2E_ORGANIZE_${name}`] || null
}

export function createOrganize(opts: OrganizeOptions): Organize {
  const service = new OrganizeService({ ...opts, broadcast: (payload) => opts.broadcast(ORGANIZE_CHANGED, payload) })
  const { ipcMain, isAppFrame } = opts
  const guard = <T>(fallback: () => T, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<T>) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!isAppFrame(event)) return fallback()
    try {
      return await fn(event, arg)
    } catch {
      return fallback()
    }
  }
  const noState = (): OrganizeState => ({ settings: { sources: [], source: '', mode: 'type', kinds: [], rules: [], auto: false }, sources: [], runs: [], watching: [], stableMs: service.stableMs })
  ipcMain.handle('ccm:organize-state', guard(noState, () => service.state()))
  ipcMain.handle('ccm:organize-settings', guard(noState, (_e, v) => service.patch(v)))
  ipcMain.handle(
    'ccm:organize-add-source',
    guard(
      () => ({ state: noState(), reason: 'failed' }),
      async (e, kind) => {
        const p = kind === 'downloads' ? opts.downloads : kind === 'desktop' ? opts.desktop : kind === 'pick' ? await opts.pickFolder(e) : null
        return service.addSource(p || null)
      },
    ),
  )
  ipcMain.handle('ccm:organize-remove-source', guard(noState, (_e, p) => service.removeSource(p)))
  ipcMain.handle('ccm:organize-plan', guard<OrganizePlan>(() => ({ id: '', source: '', mode: 'type', items: [], skipped: [], error: 'failed' }), (_e, v) => service.plan(v)))
  ipcMain.handle('ccm:organize-run', guard<OrganizeRunResult>(() => ({ run: null, moved: 0, bytes: 0, failed: [], error: 'failed' }), (_e, v) => service.run(v)))
  ipcMain.handle('ccm:organize-undo', guard<OrganizeUndoResult>(() => ({ restored: 0, conflicts: [], dirs: 0, error: 'failed' }), (_e, v) => service.undo(v)))
  ipcMain.handle('ccm:organize-junk', guard<OrganizeJunkResult>(() => ({ indexed: false, items: [] }), () => service.junk()))
  void service.start().catch(() => undefined)
  return {
    service,
    dispose: () => {
      for (const c of ORGANIZE_CHANNELS) ipcMain.removeHandler(c)
      service.dispose()
    },
  }
}
