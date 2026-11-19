import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { DiskDeleteResult, DiskDupesDone, DiskDupesProgress } from '../lib/diskMap'

const api = vi.hoisted(() => ({
  dupes: vi.fn(),
  cancel: vi.fn(),
  del: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  done: null as ((d: DiskDupesDone | null) => void) | null,
  progress: null as ((p: DiskDupesProgress) => void) | null,
}))

vi.mock('../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMap')>()),
  hasDiskMap: true,
  hasDiskDupes: true,
  diskMapStatus: async () => ({ scanning: false, paused: false, indexed: true, roots: ['C:\\'] }),
  diskMapVolumes: async () => [],
  diskMapHistory: async () => [],
  diskMapChildren: async () => null,
  diskMapDupes: (...a: unknown[]) => api.dupes(...a),
  diskMapDupesCancel: () => api.cancel(),
  diskMapDelete: (...a: unknown[]) => api.del(...a),
  diskMapPause: () => api.pause(),
  diskMapResume: () => api.resume(),
  onDiskMapProgress: () => () => {},
  onDiskMapDone: () => () => {},
  onDiskMapDupesProgress: (cb: (p: DiskDupesProgress) => void) => {
    api.progress = cb
    return () => {}
  },
  onDiskMapDupesDone: (cb: (d: DiskDupesDone | null) => void) => {
    api.done = cb
    return () => {}
  },
}))

import { dupeError, useDiskDupesStore } from './diskDupes'

const skipped = { stale: 0, cloud: 0, locked: 0, changed: 0, gone: 0 }
const done = (groups: DiskDupesDone['groups'], extra: Partial<DiskDupesDone> = {}): DiskDupesDone => ({
  ok: true,
  groups,
  totalGroups: groups.length,
  wasted: groups.reduce((s, g) => s + g.size * (g.count - 1), 0),
  candidates: 10,
  hashedFiles: 6,
  hashedBytes: 600,
  skipped,
  ms: 1234,
  ...extra,
})
const group = (hash: string, size: number, files: [string, number][], count = files.length) => ({ size, hash, count, files: files.map(([path, mtime]) => ({ path, mtime })) })

const A = group('a', 300, [['C:/new/a.bin', 30], ['C:/old/a.bin', 10], ['C:/mid/a.bin', 20]])
const B = group('b', 50, [['C:/x/b.txt', 5], ['C:/y/b.txt', 5]])

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  for (const f of [api.dupes, api.cancel, api.del, api.pause, api.resume]) f.mockReset()
  api.done = null
  api.progress = null
  api.dupes.mockResolvedValue({ started: true })
})

async function loaded() {
  const s = useDiskDupesStore()
  s.init()
  await s.start()
  api.done!(done([A, B]))
  return s
}

describe('start and finish', () => {
  it('sends the min size and level and fills the list with default ticks', async () => {
    const s = useDiskDupesStore()
    s.init()
    s.minMb = 10
    expect(await s.start()).toBe(true)
    expect(api.dupes).toHaveBeenCalledWith({ minSize: 10 * 1024 * 1024, level: 'balanced' })
    expect(s.running).toBe(true)
    api.progress!({ stage: 'quick', candidates: 1, files: 4, filesDone: 2, bytes: 10, bytesRead: 5, groups: 0, skipped: 0, current: 'C:/x', elapsedMs: 10, paused: false })
    expect(s.progress?.stage).toBe('quick')
    api.done!(done([A, B]))
    expect(s.running).toBe(false)
    expect(s.progress).toBeNull()
    expect(s.groups).toHaveLength(2)
    expect([...s.ticked].sort()).toEqual(['C:/mid/a.bin', 'C:/new/a.bin', 'C:/y/b.txt'].sort())
    expect(s.wasted).toBe(300 * 2 + 50)
    expect(s.plan).toEqual({ files: 3, bytes: 300 * 2 + 50 })
  })

  it('passes the chosen folder as the search scope', async () => {
    const s = useDiskDupesStore()
    s.scope = 'D:\\'
    await s.start()
    expect(api.dupes).toHaveBeenCalledWith(expect.objectContaining({ under: ['D:\\'] }))
  })

  it('maps start failures to a short error and stops running', async () => {
    api.dupes.mockResolvedValue({ started: false, error: 'busy' })
    const s = useDiskDupesStore()
    expect(await s.start()).toBe(false)
    expect(s.error).toBe('busy')
    expect(s.running).toBe(false)
    expect(dupeError('not-indexed')).toBe('not-indexed')
    expect(dupeError('weird')).toBe('failed')
  })

  it('keeps the old result when the run is cancelled and shows an error when it fails', async () => {
    const s = await loaded()
    await s.start()
    api.done!({ ...done([]), ok: false, cancelled: true })
    expect(s.groups).toHaveLength(2)
    expect(s.running).toBe(false)
    await s.start()
    api.done!({ ...done([]), ok: false, error: 'boom' })
    expect(s.error).toBe('failed')
    expect(s.detail).toBe('boom')
    expect(s.groups).toHaveLength(2)
  })

  it('cancels and pauses through the bridge only while running', async () => {
    const s = useDiskDupesStore()
    await s.cancel()
    expect(api.cancel).not.toHaveBeenCalled()
    await s.start()
    api.pause.mockResolvedValue(true)
    api.resume.mockResolvedValue(true)
    await s.togglePause()
    expect(s.paused).toBe(true)
    await s.togglePause()
    expect(s.paused).toBe(false)
    await s.cancel()
    expect(api.cancel).toHaveBeenCalledTimes(1)
  })
})

describe('ticks', () => {
  it('lets the user move the keeper but never tick every copy', async () => {
    const s = await loaded()
    s.setKeep(A, 'C:/mid/a.bin')
    expect([...s.ticked].filter((p) => p.includes('a.bin')).sort()).toEqual(['C:/new/a.bin', 'C:/old/a.bin'])
    expect(s.toggleFile(A, 'C:/mid/a.bin')).toBe(false)
    expect(s.toggleFile(A, 'C:/old/a.bin')).toBe(true)
    expect(s.ticked.has('C:/old/a.bin')).toBe(false)
    expect(s.plan.bytes).toBe(300 + 50)
  })

  it('resets and clears the picks', async () => {
    const s = await loaded()
    s.clearTicks()
    expect(s.plan).toEqual({ files: 0, bytes: 0 })
    s.resetTicks()
    expect(s.plan.files).toBe(3)
  })

  it('only counts groups that pass the filters', async () => {
    const s = await loaded()
    s.patchFilters({ minSize: 100 })
    expect(s.shown).toEqual([A])
    expect(s.plan).toEqual({ files: 2, bytes: 600 })
    s.patchFilters({ minSize: 0, folder: 'x/' })
    expect(s.shown).toEqual([B])
  })
})

describe('remove', () => {
  const res = (paths: string[], extra: Partial<DiskDeleteResult> = {}): DiskDeleteResult => ({ done: paths.map((path) => ({ path, size: 1 })), failed: [], freed: 123, permanent: false, ...extra })

  it('deletes only ticked copies of the visible groups and prunes the list', async () => {
    const s = await loaded()
    s.patchFilters({ minSize: 100 })
    api.del.mockImplementation(async (paths: string[]) => res(paths))
    await s.remove(false)
    expect(api.del).toHaveBeenCalledTimes(1)
    const [paths, permanent] = api.del.mock.calls[0]!
    expect([...paths].sort()).toEqual(['C:/mid/a.bin', 'C:/new/a.bin'])
    expect(permanent).toBe(false)
    s.patchFilters({ minSize: 0 })
    expect(s.groups.map((g) => g.hash)).toEqual(['b'])
    expect(s.report?.done).toBe(2)
    expect(s.report?.freed).toBe(123)
    expect(s.plan.files).toBe(1)
  })

  it('keeps copies that failed to delete and still ticked', async () => {
    const s = await loaded()
    api.del.mockImplementation(async (paths: string[]) => res(paths.filter((p) => p !== 'C:/y/b.txt'), { failed: [{ path: 'C:/y/b.txt', reason: 'locked' }] }))
    await s.remove(true)
    expect(api.del.mock.calls[0]![1]).toBe(true)
    expect(s.groups.map((g) => g.hash)).toEqual(['b'])
    expect(s.ticked.has('C:/y/b.txt')).toBe(true)
    expect(s.report?.failed).toHaveLength(1)
  })

  it('does nothing when the user cancels the native confirm', async () => {
    const s = await loaded()
    api.del.mockResolvedValue({ done: [], failed: [], freed: 0, permanent: false, cancelled: true })
    await s.remove(false)
    expect(s.groups).toHaveLength(2)
    expect(s.report).toBeNull()
    expect(s.plan.files).toBe(3)
  })

  it('reports an unavailable bridge and does not call twice at once', async () => {
    const s = await loaded()
    api.del.mockResolvedValue(null)
    await s.remove(false)
    expect(s.report?.failed).toHaveLength(3)
    expect(s.groups).toHaveLength(2)
    s.clearTicks()
    await s.remove(false)
    expect(api.del).toHaveBeenCalledTimes(1)
  })
})
