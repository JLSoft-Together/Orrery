import { DISK_FREQS, isDiskFreq } from '@ccm/shared'
import type { DiskCompare, DiskDiffRow, DiskEntry, DiskFreq, DiskSchedule, DiskSchedulePatch, DiskSnapshotMeta } from '@ccm/shared'
import { desktop } from './desktop'

export { DISK_FREQS, isDiskFreq }
export type { DiskCompare, DiskDiffRow, DiskFreq, DiskSchedule, DiskSchedulePatch, DiskSnapshotMeta }

interface SnapBridge {
  diskMapSnapList?: () => Promise<DiskSnapshotMeta[]>
  diskMapSnapCompare?: (req: { id: string; path?: string }) => Promise<DiskCompare | null>
  diskMapScheduleGet?: () => Promise<DiskSchedule>
  diskMapScheduleSet?: (patch: DiskSchedulePatch) => Promise<DiskSchedule>
  onDiskMapSchedule?: (cb: (s: DiskSchedule) => void) => () => void
  onDiskMapSnapNew?: (cb: (m: DiskSnapshotMeta) => void) => () => void
}

const bridge = desktop as unknown as SnapBridge | undefined

export const hasDiskSnap = typeof bridge?.diskMapSnapList === 'function' && typeof bridge?.diskMapSnapCompare === 'function'
export const hasDiskSchedule = typeof bridge?.diskMapScheduleGet === 'function' && typeof bridge?.diskMapScheduleSet === 'function'

async function call<T>(fn: (() => Promise<T>) | undefined, fallback: T): Promise<T> {
  if (!fn) return fallback
  try {
    return await fn()
  } catch {
    return fallback
  }
}

export const diskSnapList = (): Promise<DiskSnapshotMeta[]> => call(bridge?.diskMapSnapList && (async () => cleanMetas(await bridge.diskMapSnapList!())), [])
export const diskSnapCompare = (id: string, path: string): Promise<DiskCompare | null> =>
  call<DiskCompare | null>(bridge?.diskMapSnapCompare && (async () => cleanCompare(await bridge.diskMapSnapCompare!(path ? { id, path } : { id }))), null)
export const diskScheduleGet = (): Promise<DiskSchedule | null> => call<DiskSchedule | null>(bridge?.diskMapScheduleGet && (async () => cleanSchedule(await bridge.diskMapScheduleGet!())), null)
export const diskScheduleSet = (patch: DiskSchedulePatch): Promise<DiskSchedule | null> =>
  call<DiskSchedule | null>(bridge?.diskMapScheduleSet && (async () => cleanSchedule(await bridge.diskMapScheduleSet!(patch))), null)
export const onDiskSchedule = (cb: (s: DiskSchedule) => void): (() => void) => bridge?.onDiskMapSchedule?.((s) => {
  const c = cleanSchedule(s)
  if (c) cb(c)
}) ?? (() => {})
export const onDiskSnapNew = (cb: (m: DiskSnapshotMeta) => void): (() => void) => bridge?.onDiskMapSnapNew?.(cb) ?? (() => {})

const num = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) ? v : 0)

export function cleanMetas(raw: unknown): DiskSnapshotMeta[] {
  if (!Array.isArray(raw)) return []
  return raw.flatMap((m): DiskSnapshotMeta[] => {
    const o = m && typeof m === 'object' ? (m as Record<string, unknown>) : {}
    if (typeof o.id !== 'string' || !o.id || typeof o.at !== 'number') return []
    return [{ id: o.id, at: o.at, bytes: num(o.bytes), roots: Array.isArray(o.roots) ? o.roots.filter((r): r is string => typeof r === 'string') : [] }]
  })
}

function cleanRow(raw: unknown): DiskDiffRow | null {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : null
  if (!o || typeof o.path !== 'string') return null
  const state = o.state === 'new' || o.state === 'gone' || o.state === 'changed' || o.state === 'same' ? o.state : 'same'
  const before = num(o.before)
  const after = num(o.after)
  return { path: o.path, name: typeof o.name === 'string' ? o.name : o.path, before, after, delta: after - before, state }
}

export function cleanCompare(raw: unknown): DiskCompare | null {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : null
  if (!o) return null
  const base = cleanMetas([o.base])[0]
  if (!base) return null
  const rows = (list: unknown): DiskDiffRow[] => (Array.isArray(list) ? list.map(cleanRow).filter((r): r is DiskDiffRow => r !== null) : [])
  return { base, path: typeof o.path === 'string' ? o.path : '', covered: o.covered === true, rows: rows(o.rows), top: rows(o.top) }
}

export function cleanSchedule(raw: unknown): DiskSchedule | null {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : null
  if (!o || !isDiskFreq(o.freq)) return null
  const last = o.last && typeof o.last === 'object' ? (o.last as Record<string, unknown>) : null
  return {
    freq: o.freq,
    telegram: o.telegram === true,
    last: last && typeof last.at === 'number' ? { at: last.at, ok: last.ok === true } : null,
    running: o.running === true,
    paused: o.paused === true,
  }
}

export function sizeSign(delta: number): 'up' | 'down' | 'same' {
  return delta > 0 ? 'up' : delta < 0 ? 'down' : 'same'
}

export interface DiffIndex {
  exact: Map<string, DiskDiffRow>
  folded: Map<string, DiskDiffRow>
}

export function indexRows(rows: readonly DiskDiffRow[]): DiffIndex {
  const exact = new Map<string, DiskDiffRow>()
  const folded = new Map<string, DiskDiffRow>()
  for (const r of rows) {
    exact.set(r.path, r)
    folded.set(r.path.toLowerCase(), r)
  }
  return { exact, folded }
}

export function findRow(index: DiffIndex, path: string): DiskDiffRow | undefined {
  return index.exact.get(path) ?? index.folded.get(path.toLowerCase())
}

export function sortByDelta(items: readonly DiskEntry[], index: DiffIndex): DiskEntry[] {
  const delta = (e: DiskEntry): number => (e.dir ? (findRow(index, e.path)?.delta ?? 0) : Number.NEGATIVE_INFINITY)
  return [...items].sort((a, b) => delta(b) - delta(a) || b.size - a.size)
}

export function snapshotLabel(m: DiskSnapshotMeta, locale: string): string {
  return new Date(m.at).toLocaleString(locale, { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}

export function whenLabel(at: number, locale: string): string {
  return new Date(at).toLocaleString(locale, { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}
