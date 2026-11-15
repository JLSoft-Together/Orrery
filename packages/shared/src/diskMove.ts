export type MoveKind = 'gradle' | 'pub' | 'avd' | 'sdk' | 'npm' | 'docker' | 'picked' | 'clean'

export type MoveBlock = 'link' | 'missing' | 'notDir' | 'guard' | 'self' | 'locked' | 'pending' | 'moved'

export interface MoveCandidate {
  id: string
  path: string
  kind: MoveKind
  size?: number
  guide?: boolean
  block?: MoveBlock
  holder?: string
}

export type MoveVolumeReason = 'same' | 'network' | 'space' | 'missing'

export interface MoveVolume {
  mount: string
  label: string
  free: number
  total: number
  ok: boolean
  reason?: MoveVolumeReason
}

export interface MovePlan {
  id: string
  candidate: string
  source: string
  target: string
  bytes: number
  files: number
  volume: string
  need: number
  free: number
}

export type MovePhase = 'check' | 'copy' | 'verify' | 'switch' | 'cleanup' | 'rollback' | 'done'

export interface MoveProgress {
  id: string
  phase: MovePhase
  bytes: number
  total: number
  files: number
}

export interface DiskMoveRecord {
  id: string
  source: string
  target: string
  at: number
  bytes: number
  files: number
  state: 'moved' | 'back'
  leftover?: string
}

export type MoveStage = 'copying' | 'renamed' | 'linked'

export interface MovePending {
  id: string
  kind: 'move' | 'back'
  source: string
  target: string
  stage: MoveStage
}

export interface MoveState {
  moves: DiskMoveRecord[]
  pending: MovePending[]
  running: string | null
}

export interface MoveResult {
  ok: boolean
  error?: string
  holder?: string
  rolledBack?: boolean
  cancelled?: boolean
  leftover?: string
  record?: DiskMoveRecord
}

export interface MovePlanResult {
  plan?: MovePlan
  error?: string
  holder?: string
}

export const MOVE_SUFFIX = '.orrery-moving'
export const MOVE_BACK_SUFFIX = '.orrery-back'
export const MOVE_DIR = 'OrreryMoved'
export const MOVE_SPARE = 0.1

const ID = /^[a-z0-9-]{1,64}$/
const idOk = (v: unknown): v is string => typeof v === 'string' && ID.test(v)
const textOk = (v: unknown, max = 4096): v is string => typeof v === 'string' && v.length > 0 && v.length <= max && !/[\u0000-\u001f]/.test(v)
const obj = (raw: unknown): Record<string, unknown> | null => (raw && typeof raw === 'object' && !Array.isArray(raw) ? (raw as Record<string, unknown>) : null)

export function parseMovePlan(raw: unknown): { candidate: string; volume: string } | null {
  const o = obj(raw)
  return o && idOk(o.candidate) && textOk(o.volume, 1024) ? { candidate: o.candidate, volume: o.volume } : null
}

export function parseMoveId(raw: unknown, key: 'plan' | 'move' | 'candidate'): string | null {
  const o = obj(raw)
  const v = o?.[key]
  return idOk(v) ? v : null
}

export function parseMoveRecover(raw: unknown): { pending: string; action: 'restore' | 'finish' } | null {
  const o = obj(raw)
  if (!o || !idOk(o.pending) || (o.action !== 'restore' && o.action !== 'finish')) return null
  return { pending: o.pending, action: o.action }
}

export function parseMoveFromClean(raw: unknown): { path: string; open: boolean } | null {
  const o = obj(raw)
  return o && textOk(o.path) ? { path: o.path, open: o.open === true } : null
}

export function moveNeed(bytes: number): number {
  return Math.ceil(Math.max(0, bytes) * (1 + MOVE_SPARE))
}
