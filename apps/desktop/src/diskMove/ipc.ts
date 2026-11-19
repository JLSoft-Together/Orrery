import { AsyncLocalStorage } from 'node:async_hooks'
import type { IpcMain, IpcMainInvokeEvent, MessageBoxOptions } from 'electron'
import { parseMoveFromClean, parseMoveId, parseMovePlan, parseMoveRecover, type MoveCandidate, type MovePlanResult, type MoveResult, type MoveState, type MoveVolume } from '@ccm/shared'
import { moveBoxOptions } from './dialog'
import { DiskMoveService, type DiskMoveDeps } from './service'

export const DISKMOVE_CHANNELS = [
  'ccm:diskmove-list',
  'ccm:diskmove-pick',
  'ccm:diskmove-from-clean',
  'ccm:diskmove-volumes',
  'ccm:diskmove-plan',
  'ccm:diskmove-run',
  'ccm:diskmove-back',
  'ccm:diskmove-cancel',
  'ccm:diskmove-state',
  'ccm:diskmove-recover',
] as const
export const DISKMOVE_PROGRESS = 'ccm:diskmove-progress'

export interface DiskMoveIpcOptions extends Omit<DiskMoveDeps, 'confirm' | 'emit' | 'pick'> {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  broadcast: (channel: string, payload: unknown) => void
  ask: (event: IpcMainInvokeEvent, opts: MessageBoxOptions) => Promise<number>
  pickFolder?: (event: IpcMainInvokeEvent) => Promise<string | null>
  autoConfirm?: boolean
  locale?: () => string
}

export interface DiskMove {
  service: DiskMoveService
  dispose: () => void
}

export function createDiskMove(opts: DiskMoveIpcOptions): DiskMove {
  const scope = new AsyncLocalStorage<IpcMainInvokeEvent>()
  const service = new DiskMoveService({
    ...opts,
    emit: (p) => opts.broadcast(DISKMOVE_PROGRESS, p),
    confirm: async (ask) => {
      if (opts.autoConfirm) return true
      const current = scope.getStore()
      if (!current) return false
      return (await opts.ask(current, moveBoxOptions(ask, opts.locale?.() ?? 'en'))) === 0
    },
    pick: async () => {
      const current = scope.getStore()
      return current && opts.pickFolder ? opts.pickFolder(current) : null
    },
  })
  const { ipcMain, isAppFrame } = opts
  const guard = <T>(fallback: T, fn: (arg: unknown) => Promise<T> | T) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!isAppFrame(event)) return fallback
    try {
      return await scope.run(event, () => fn(arg))
    } catch {
      return fallback
    }
  }
  const bad: MoveResult = { ok: false, error: 'invalid' }
  ipcMain.handle('ccm:diskmove-list', guard<MoveCandidate[]>([], () => service.list()))
  ipcMain.handle('ccm:diskmove-pick', guard<MoveCandidate | null>(null, () => service.pick()))
  ipcMain.handle(
    'ccm:diskmove-from-clean',
    guard<MoveCandidate | null>(null, (raw) => {
      const req = parseMoveFromClean(raw)
      return req ? service.fromClean(req.path, req.open) : null
    }),
  )
  ipcMain.handle(
    'ccm:diskmove-volumes',
    guard<MoveVolume[]>([], (raw) => {
      const id = parseMoveId(raw, 'candidate')
      return id ? service.volumes(id) : []
    }),
  )
  ipcMain.handle(
    'ccm:diskmove-plan',
    guard<MovePlanResult>({ error: 'invalid' }, (raw) => {
      const req = parseMovePlan(raw)
      return req ? service.plan(req.candidate, req.volume) : { error: 'invalid' }
    }),
  )
  ipcMain.handle(
    'ccm:diskmove-run',
    guard<MoveResult>(bad, (raw) => {
      const id = parseMoveId(raw, 'plan')
      return id ? service.run(id) : bad
    }),
  )
  ipcMain.handle(
    'ccm:diskmove-back',
    guard<MoveResult>(bad, (raw) => {
      const id = parseMoveId(raw, 'move')
      return id ? service.back(id) : bad
    }),
  )
  ipcMain.handle('ccm:diskmove-cancel', guard(false, () => service.cancel()))
  ipcMain.handle('ccm:diskmove-state', guard<MoveState>({ moves: [], pending: [], running: null }, () => service.state()))
  ipcMain.handle(
    'ccm:diskmove-recover',
    guard<MoveResult>(bad, (raw) => {
      const req = parseMoveRecover(raw)
      return req ? service.recover(req.pending, req.action) : bad
    }),
  )
  return {
    service,
    dispose: () => {
      for (const c of DISKMOVE_CHANNELS) ipcMain.removeHandler(c)
    },
  }
}
