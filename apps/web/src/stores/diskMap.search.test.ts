import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { DiskChildren, DiskDeleteResult, DiskEntry, DiskSearchResult, DiskStatus } from '../lib/diskMap'

const api = vi.hoisted(() => ({
  status: vi.fn(),
  volumes: vi.fn(),
  children: vi.fn(),
  search: vi.fn(),
  del: vi.fn(),
  history: vi.fn(),
}))

vi.mock('../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMap')>()),
  hasDiskMap: true,
  diskMapStatus: () => api.status(),
  diskMapVolumes: () => api.volumes(),
  diskMapChildren: (...a: unknown[]) => api.children(...a),
  diskMapSearch: (...a: unknown[]) => api.search(...a),
  diskMapDelete: (...a: unknown[]) => api.del(...a),
  diskMapHistory: () => api.history(),
  onDiskMapProgress: () => () => {},
  onDiskMapDone: () => () => {},
}))

import { useDiskMapStore } from './diskMap'

const entry = (name: string, size: number): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir: false, mtime: 1 })
const found = (items: DiskEntry[], total = items.length): DiskSearchResult => ({ total, ms: 3, items })
const status = (): DiskStatus => ({ scanning: false, paused: false, indexed: true, roots: ['C:\\r'] })
const listing = (p: string): DiskChildren => ({ path: p, size: 10, total: 1, items: [entry('a', 10)] })

beforeEach(() => {
  vi.useFakeTimers()
  setActivePinia(createPinia())
  localStorage.clear()
  for (const f of [api.status, api.volumes, api.children, api.search, api.del, api.history]) f.mockReset()
  api.status.mockResolvedValue(status())
  api.volumes.mockResolvedValue([])
  api.history.mockResolvedValue([])
  api.children.mockImplementation(async (p: string) => listing(p))
  api.search.mockResolvedValue(found([entry('x', 5)]))
})

afterEach(() => vi.useRealTimers())

describe('disk map search', () => {
  it('debounces typing into a single query', async () => {
    const s = useDiskMapStore()
    s.setMode('search')
    expect(api.search).not.toHaveBeenCalled()
    s.patchFilters({ text: 'a' })
    await vi.advanceTimersByTimeAsync(60)
    s.patchFilters({ text: 'ab' })
    await vi.advanceTimersByTimeAsync(60)
    s.patchFilters({ text: 'abc' })
    expect(api.search).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(130)
    expect(api.search).toHaveBeenCalledTimes(1)
    expect(api.search.mock.calls[0]![0]).toMatchObject({ query: 'abc', sort: 'size', limit: 500 })
    expect(s.results?.items.map((i) => i.name)).toEqual(['x'])
    expect(s.searching).toBe(false)
  })

  it('does not query while nothing is typed or picked', async () => {
    const s = useDiskMapStore()
    s.setMode('search')
    await vi.advanceTimersByTimeAsync(300)
    expect(api.search).not.toHaveBeenCalled()
    expect(s.results).toBeNull()
    s.patchFilters({ text: 'a' })
    await vi.advanceTimersByTimeAsync(130)
    s.patchFilters({ text: '' })
    await vi.advanceTimersByTimeAsync(130)
    expect(s.results).toBeNull()
  })

  it('drops a slow older answer', async () => {
    const s = useDiskMapStore()
    let first!: (r: DiskSearchResult) => void
    api.search.mockImplementationOnce(() => new Promise<DiskSearchResult>((r) => (first = r)))
    s.setMode('search')
    s.patchFilters({ text: 'old' })
    await vi.advanceTimersByTimeAsync(130)
    expect(api.search).toHaveBeenCalledTimes(1)
    api.search.mockResolvedValueOnce(found([entry('new', 9)]))
    s.patchFilters({ text: 'new' })
    await vi.advanceTimersByTimeAsync(130)
    expect(s.results?.items[0]?.name).toBe('new')
    first(found([entry('old', 1)]))
    await vi.advanceTimersByTimeAsync(0)
    expect(s.results?.items[0]?.name).toBe('new')
    expect(s.searching).toBe(false)
  })

  it('big files preset queries right away with min 100 MB', async () => {
    const s = useDiskMapStore()
    s.setMode('big')
    await vi.advanceTimersByTimeAsync(0)
    expect(api.search.mock.calls[0]![0]).toMatchObject({ only: 'file', sort: 'size', minSize: 100 * 1024 ** 2 })
    s.patchFilters({ minSize: 1024 ** 3 })
    await vi.advanceTimersByTimeAsync(130)
    expect(api.search.mock.calls[1]![0]).toMatchObject({ minSize: 1024 ** 3 })
  })

  it('old files preset uses N months and re-runs on change', async () => {
    const s = useDiskMapStore()
    s.setMode('old')
    await vi.advanceTimersByTimeAsync(0)
    const q1 = api.search.mock.calls[0]![0] as { before: number }
    s.setOldMonths(3)
    await vi.advanceTimersByTimeAsync(0)
    const q2 = api.search.mock.calls[1]![0] as { before: number }
    expect(q2.before).toBeGreaterThan(q1.before)
    expect(s.oldMonths).toBe(3)
  })

  it('scoped search sends the open folder as under', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.setMode('search')
    s.patchFilters({ text: 'a', scoped: true })
    await vi.advanceTimersByTimeAsync(130)
    expect(api.search.mock.calls[0]![0]).toMatchObject({ under: 'C:\\r' })
  })

  it('reports a failed search and clears it on the next good one', async () => {
    const s = useDiskMapStore()
    api.search.mockResolvedValueOnce(null)
    s.setMode('search')
    s.patchFilters({ text: 'a' })
    await vi.advanceTimersByTimeAsync(130)
    expect(s.searchFailed).toBe(true)
    expect(s.results).toBeNull()
    s.patchFilters({ text: 'ab' })
    await vi.advanceTimersByTimeAsync(130)
    expect(s.searchFailed).toBe(false)
    expect(s.results?.total).toBe(1)
  })

  it('keeps total beyond the top N and selection only for rows still listed', async () => {
    const s = useDiskMapStore()
    api.search.mockResolvedValueOnce(found([entry('a', 3), entry('b', 2)], 9000))
    s.setMode('search')
    s.patchFilters({ text: 'a' })
    await vi.advanceTimersByTimeAsync(130)
    expect(s.results?.total).toBe(9000)
    expect(s.listed).toHaveLength(2)
    s.toggle('C:\\r\\a', false)
    s.toggle('C:\\r\\b', false)
    expect(s.selectedSize).toBe(5)
    api.search.mockResolvedValueOnce(found([entry('b', 2)]))
    s.patchFilters({ text: 'b' })
    await vi.advanceTimersByTimeAsync(130)
    expect([...s.selected]).toEqual(['C:\\r\\b'])
  })

  it('range select and select all work on the result list', async () => {
    const s = useDiskMapStore()
    api.search.mockResolvedValueOnce(found([entry('a', 1), entry('b', 1), entry('c', 1), entry('d', 1)]))
    s.setMode('search')
    s.patchFilters({ text: 'z' })
    await vi.advanceTimersByTimeAsync(130)
    s.toggle('C:\\r\\a', false)
    s.toggle('C:\\r\\c', true)
    expect(s.selected.size).toBe(3)
    s.selectAll()
    expect(s.selected.size).toBe(4)
  })

  it('delete from results removes then re-searches', async () => {
    const s = useDiskMapStore()
    await s.init()
    api.search.mockResolvedValueOnce(found([entry('a', 3), entry('b', 2)]))
    s.setMode('search')
    s.patchFilters({ text: 'q' })
    await vi.advanceTimersByTimeAsync(130)
    s.selectOnly('C:\\r\\a')
    api.search.mockResolvedValueOnce(found([entry('b', 2)]))
    const res: DiskDeleteResult = { done: [{ path: 'C:\\r\\a', size: 3 }], failed: [], freed: 3, permanent: false }
    api.del.mockResolvedValue(res)
    await s.removeSelected(false)
    expect(api.del).toHaveBeenCalledWith(['C:\\r\\a'], false)
    expect(s.results?.items.map((i) => i.name)).toEqual(['b'])
    expect(s.selected.size).toBe(0)
  })

  it('leaving search cancels the pending query and clears results', async () => {
    const s = useDiskMapStore()
    s.setMode('search')
    s.patchFilters({ text: 'a' })
    s.setMode('browse')
    await vi.advanceTimersByTimeAsync(300)
    expect(api.search).not.toHaveBeenCalled()
    expect(s.results).toBeNull()
    expect(s.mode).toBe('browse')
  })

  it('reset filters runs again with defaults', async () => {
    const s = useDiskMapStore()
    s.setMode('search')
    s.patchFilters({ text: 'a', kind: 'video' })
    await vi.advanceTimersByTimeAsync(130)
    s.resetFilters()
    await vi.advanceTimersByTimeAsync(0)
    expect(s.filters.text).toBe('')
    expect(s.results).toBeNull()
  })
})
