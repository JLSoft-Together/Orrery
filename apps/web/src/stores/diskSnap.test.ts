import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { DiskCompare, DiskDiffRow, DiskSchedule, DiskSnapshotMeta } from '../lib/diskSnap'
import type { DiskEntry } from '../lib/diskMap'

const api = vi.hoisted(() => ({
  list: vi.fn(),
  compare: vi.fn(),
  sget: vi.fn(),
  sset: vi.fn(),
  newCb: null as null | ((m: unknown) => void),
  schedCb: null as null | ((s: unknown) => void),
  offs: 0,
}))

vi.mock('../lib/diskSnap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskSnap')>()),
  hasDiskSnap: true,
  hasDiskSchedule: true,
  diskSnapList: () => api.list(),
  diskSnapCompare: (...a: unknown[]) => api.compare(...a),
  diskScheduleGet: () => api.sget(),
  diskScheduleSet: (...a: unknown[]) => api.sset(...a),
  onDiskSnapNew: (cb: (m: unknown) => void) => {
    api.newCb = cb
    return () => void api.offs++
  },
  onDiskSchedule: (cb: (s: unknown) => void) => {
    api.schedCb = cb
    return () => void api.offs++
  },
}))

import { useDiskSnapStore } from './diskSnap'

const meta = (id: string, at = 1): DiskSnapshotMeta => ({ id, at, bytes: 100, roots: ['C:\\r'] })
const row = (path: string, before: number, after: number): DiskDiffRow => ({ path, name: path.split('\\').pop()!, before, after, delta: after - before, state: 'changed' })
const cmp = (id: string, path: string, rows: DiskDiffRow[], covered = true, top: DiskDiffRow[] = []): DiskCompare => ({ base: meta(id), path, covered, rows, top })
const entry = (name: string, size: number, dir = true): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir, mtime: 1 })
const sched = (over: Partial<DiskSchedule> = {}): DiskSchedule => ({ freq: 'daily', telegram: true, last: null, running: false, paused: false, ...over })

beforeEach(() => {
  setActivePinia(createPinia())
  api.list.mockReset().mockResolvedValue([meta('s2', 2), meta('s1', 1)])
  api.compare.mockReset().mockImplementation(async (id: string, path: string) => cmp(id, path, [row('C:\\r\\a', 10, 60), row('C:\\r\\b', 50, 20)], true, [row('C:\\r\\a', 10, 60)]))
  api.sget.mockReset().mockResolvedValue(sched())
  api.sset.mockReset().mockImplementation(async (patch: object) => sched(patch))
  api.newCb = null
  api.schedCb = null
  api.offs = 0
})

describe('diskSnap store', () => {
  it('init loads snapshots and the schedule', async () => {
    const s = useDiskSnapStore()
    await s.init()
    expect(s.snapshots.map((x) => x.id)).toEqual(['s2', 's1'])
    expect(s.schedule?.freq).toBe('daily')
    expect(s.active).toBe(false)
  })

  it('select loads the comparison of the folder and exposes rows', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.select('s1', 'C:\\r')
    expect(api.compare).toHaveBeenCalledWith('s1', 'C:\\r')
    expect(s.active).toBe(true)
    expect(s.rowOf('C:\\r\\a')?.delta).toBe(50)
    expect(s.rowOf('c:\\R\\B')?.delta).toBe(-30)
    expect(s.rowOf('C:\\r\\none')).toBeUndefined()
    expect(s.top).toHaveLength(1)
    expect(s.covered).toBe(true)
  })

  it('selecting none clears everything', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.select('s1', 'C:\\r')
    s.sortDelta = true
    s.showTop = true
    await s.select('', 'C:\\r')
    expect(s.active).toBe(false)
    expect(s.baseId).toBe('')
    expect(s.sortDelta).toBe(false)
    expect(s.showTop).toBe(false)
    expect(s.rowOf('C:\\r\\a')).toBeUndefined()
  })

  it('orders by change only while active and sorting', async () => {
    const s = useDiskSnapStore()
    await s.init()
    const items = [entry('b', 20), entry('a', 60), entry('c', 5)]
    expect(s.order(items)).toBe(items)
    await s.select('s1', 'C:\\r')
    expect(s.order(items)).toBe(items)
    s.toggleSort()
    expect(s.order(items).map((i) => i.name)).toEqual(['a', 'c', 'b'])
  })

  it('sync loads a new folder once and ignores the same one', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.sync('C:\\r')
    expect(api.compare).not.toHaveBeenCalled()
    await s.select('s1', 'C:\\r')
    await s.sync('C:\\r')
    expect(api.compare).toHaveBeenCalledTimes(1)
    await s.sync('C:\\r\\a')
    expect(api.compare).toHaveBeenLastCalledWith('s1', 'C:\\r\\a')
  })

  it('drops a stale answer when the folder changed meanwhile', async () => {
    const s = useDiskSnapStore()
    await s.init()
    let release: (c: DiskCompare) => void = () => undefined
    api.compare.mockImplementationOnce(() => new Promise<DiskCompare>((r) => (release = r)))
    const slow = s.select('s1', 'C:\\r')
    api.compare.mockImplementationOnce(async () => cmp('s1', 'C:\\r\\a', [row('C:\\r\\a\\x', 1, 9)]))
    await s.sync('C:\\r\\a')
    release(cmp('s1', 'C:\\r', [row('C:\\r\\old', 1, 2)]))
    await slow
    expect(s.rowOf('C:\\r\\a\\x')?.delta).toBe(8)
    expect(s.rowOf('C:\\r\\old')).toBeUndefined()
    expect(s.loading).toBe(false)
  })

  it('a failed comparison shows as failed and not active', async () => {
    const s = useDiskSnapStore()
    await s.init()
    api.compare.mockResolvedValueOnce(null)
    await s.select('s1', 'C:\\r')
    expect(s.failed).toBe(true)
    expect(s.active).toBe(false)
  })

  it('a new snapshot refreshes the list and the open comparison', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.select('s1', 'C:\\r\\a')
    api.list.mockResolvedValue([meta('s3', 3), meta('s2', 2), meta('s1', 1)])
    api.newCb?.(meta('s3', 3))
    await vi.waitFor(() => expect(s.snapshots).toHaveLength(3))
    await vi.waitFor(() => expect(api.compare).toHaveBeenCalledTimes(2))
    expect(api.compare).toHaveBeenLastCalledWith('s1', 'C:\\r\\a')
  })

  it('clears when the chosen snapshot was rotated away', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.select('s1', 'C:\\r')
    api.list.mockResolvedValue([meta('s3', 3), meta('s2', 2)])
    await s.loadList()
    expect(s.baseId).toBe('')
    expect(s.active).toBe(false)
  })

  it('schedule changes go to main and events update the state', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.setSchedule({ freq: 'weekly' })
    expect(api.sset).toHaveBeenCalledWith({ freq: 'weekly' })
    expect(s.schedule?.freq).toBe('weekly')
    api.schedCb?.(sched({ freq: 'off', running: true }))
    expect(s.schedule).toMatchObject({ freq: 'off', running: true })
  })

  it('dispose unsubscribes and init subscribes only once', async () => {
    const s = useDiskSnapStore()
    await s.init()
    await s.init()
    s.dispose()
    expect(api.offs).toBe(2)
  })
})
