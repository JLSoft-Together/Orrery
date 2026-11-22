import type { DiskMoveRecord, MoveCandidate, MovePending, MovePlan, MovePlanResult, MoveProgress, MoveResult, MoveState, MoveVolume } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'

export type { DiskMoveRecord, MoveCandidate, MovePending, MovePlan, MovePlanResult, MoveProgress, MoveResult, MoveState, MoveVolume }

interface MoveBridge {
  diskMoveList?: () => Promise<unknown>
  diskMovePick?: () => Promise<unknown>
  diskMoveFromClean?: (req: unknown) => Promise<unknown>
  diskMoveVolumes?: (req: unknown) => Promise<unknown>
  diskMovePlan?: (req: unknown) => Promise<unknown>
  diskMoveRun?: (req: unknown) => Promise<unknown>
  diskMoveBack?: (req: unknown) => Promise<unknown>
  diskMoveCancel?: () => Promise<unknown>
  diskMoveState?: () => Promise<unknown>
  diskMoveRecover?: (req: unknown) => Promise<unknown>
  onDiskMoveProgress?: (cb: (p: unknown) => void) => () => void
}
const bridge = desktop as unknown as MoveBridge | undefined

export const hasDiskMove = typeof bridge?.diskMoveList === 'function' && typeof bridge?.diskMoveRun === 'function'

const call = async <T>(fn: (() => Promise<unknown>) | undefined, fallback: T): Promise<T> => {
  if (!fn) return fallback
  try {
    const v = await fn()
    return (v ?? fallback) as T
  } catch {
    return fallback
  }
}

const bad: MoveResult = { ok: false, error: 'invalid' }
export const diskMoveList = (): Promise<MoveCandidate[]> => call(bridge?.diskMoveList && (() => bridge.diskMoveList!()), [])
export const diskMovePick = (): Promise<MoveCandidate | null> => call<MoveCandidate | null>(bridge?.diskMovePick && (() => bridge.diskMovePick!()), null)
export const diskMoveFromClean = (path: string, open = false): Promise<MoveCandidate | null> =>
  call<MoveCandidate | null>(bridge?.diskMoveFromClean && (() => bridge.diskMoveFromClean!(open ? { path, open: true } : { path })), null)
export const diskMoveVolumes = (candidate: string): Promise<MoveVolume[]> => call(bridge?.diskMoveVolumes && (() => bridge.diskMoveVolumes!({ candidate })), [])
export const diskMovePlan = (candidate: string, volume: string): Promise<MovePlanResult> => call<MovePlanResult>(bridge?.diskMovePlan && (() => bridge.diskMovePlan!({ candidate, volume })), { error: 'invalid' })
export const diskMoveRun = (plan: string): Promise<MoveResult> => call(bridge?.diskMoveRun && (() => bridge.diskMoveRun!({ plan })), bad)
export const diskMoveBack = (move: string): Promise<MoveResult> => call(bridge?.diskMoveBack && (() => bridge.diskMoveBack!({ move })), bad)
export const diskMoveCancel = (): Promise<boolean> => call(bridge?.diskMoveCancel && (() => bridge.diskMoveCancel!()), false)
export const diskMoveState = (): Promise<MoveState> => call<MoveState>(bridge?.diskMoveState && (() => bridge.diskMoveState!()), { moves: [], pending: [], running: null })
export const diskMoveRecover = (pending: string, action: 'restore' | 'finish'): Promise<MoveResult> => call(bridge?.diskMoveRecover && (() => bridge.diskMoveRecover!({ pending, action })), bad)
export const onDiskMoveProgress = (cb: (p: MoveProgress) => void): (() => void) => bridge?.onDiskMoveProgress?.((p) => cb(p as MoveProgress)) ?? (() => {})

export function percent(p: Pick<MoveProgress, 'bytes' | 'total'> | null): number {
  if (!p || p.total <= 0) return 0
  return Math.max(0, Math.min(100, Math.round((p.bytes / p.total) * 100)))
}

export function errorKey(error: string | undefined): string {
  if (!error) return 'failed'
  if (error.startsWith('verify:')) return 'verify'
  const known = ['busy', 'invalid', 'cancelled', 'locked', 'space', 'same', 'network', 'missing', 'link', 'notDir', 'guard', 'self', 'pending', 'moved', 'target', 'targetMissing', 'notLink', 'leftover', 'linkCheck', 'restore', 'ENOSPC', 'EBUSY', 'EPERM', 'EACCES']
  return known.includes(error) ? error : 'failed'
}
