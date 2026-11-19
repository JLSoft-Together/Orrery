import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { DiskChildren, DiskDeleteResult, DiskEntry, DiskProgress, DiskStatus } from '../lib/diskMap'

const api = vi.hoisted(() => ({
  progress: null as null | ((p: DiskProgress) => void),
  done: null as null | (() => void),
  status: vi.fn(),
  volumes: vi.fn(),
  scan: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  cancel: vi.fn(),
  children: vi.fn(),
  del: vi.fn(),
  history: vi.fn(),
  reveal: vi.fn(),
}))

vi.mock('../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMap')>()),
  hasDiskMap: true,
  diskMapStatus: () => api.status(),
  diskMapVolumes: () => api.volumes(),
  diskMapScan: (...a: unknown[]) => api.scan(...a),
  diskMapPause: () => api.pause(),
  diskMapResume: () => api.resume(),
  diskMapCancel: () => api.cancel(),
  diskMapChildren: (...a: unknown[]) => api.children(...a),
  diskMapDelete: (...a: unknown[]) => api.del(...a),
  diskMapHistory: () => api.history(),
  diskMapReveal: (p: string) => api.reveal(p),
  onDiskMapProgress: (cb: (p: DiskProgress) => void) => {
    api.progress = cb
    return () => (api.progress = null)
  },
  onDiskMapDone: (cb: () => void) => {
    api.done = cb
    return () => (api.done = null)
  },
}))

import { useDiskMapStore } from './diskMap'

const entry = (name: string, size: number, dir = false): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir, mtime: 1 })
const listing = (path: string, items: DiskEntry[]): DiskChildren => ({ path, size: items.reduce((s, i) => s + i.size, 0), total: items.length, items })
const status = (over: Partial<DiskStatus> = {}): DiskStatus => ({ scanning: false, paused: false, indexed: true, roots: ['C:\\r'], ...over })

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  for (const f of [api.status, api.volumes, api.scan, api.pause, api.resume, api.cancel, api.children, api.del, api.history, api.reveal]) f.mockReset()
  api.progress = null
  api.done = null
  api.status.mockResolvedValue(status())
  api.volumes.mockResolvedValue([{ mount: 'C:\\r', label: '', fs: 'NTFS', kind: 'fixed', network: false, total: 100, free: 40 }])
  api.history.mockResolvedValue([])
  api.children.mockImplementation(async (p: string) => listing(p, [entry('a', 300, true), entry('b', 200), entry('c', 100)]))
})

describe('disk map store', () => {
  it('opens the first root when an index exists', async () => {
    const s = useDiskMapStore()
    await s.init()
    expect(s.path).toBe('C:\\r')
    expect(s.items.map((i) => i.name)).toEqual(['a', 'b', 'c'])
    expect(s.roots).toEqual(['C:\\r'])
    expect(s.canUp).toBe(false)
  })

  it('stays empty without index and handles an empty tree', async () => {
    api.status.mockResolvedValue(status({ indexed: false, roots: [] }))
    const s = useDiskMapStore()
    await s.init()
    expect(s.path).toBe('')
    expect(s.items).toEqual([])
    api.children.mockResolvedValue(null)
    await s.open('C:\\r\\none')
    expect(s.items).toEqual([])
    expect(s.loading).toBe(false)
  })

  it('walks scan, progress and done', async () => {
    api.status.mockResolvedValue(status({ indexed: false, roots: [] }))
    api.scan.mockResolvedValue(true)
    const s = useDiskMapStore()
    await s.init()
    s.toggleRoot('C:\\r')
    api.status.mockResolvedValue(status({ scanning: true, indexed: false }))
    expect(await s.scan()).toBe(true)
    expect(api.scan).toHaveBeenCalledWith(['C:\\r'], 'balanced')
    expect(s.scanning).toBe(true)
    api.progress?.({ files: 10, bytes: 1000, current: 'C:\\r\\x', elapsedMs: 500 })
    expect(s.progress?.files).toBe(10)
    api.status.mockResolvedValue(status())
    api.done?.()
    await vi.waitFor(() => expect(s.path).toBe('C:\\r'))
    expect(s.progress).toBeNull()
    expect(s.scanning).toBe(false)
    expect(s.items).toHaveLength(3)
  })

  it('does not scan without a root or while scanning, and resets progress when the bridge refuses', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.roots = []
    expect(await s.scan()).toBe(false)
    s.roots = ['C:\\r']
    api.scan.mockResolvedValue(false)
    expect(await s.scan()).toBe(false)
    expect(s.progress).toBeNull()
  })

  it('pauses, resumes and cancels through status refresh', async () => {
    const s = useDiskMapStore()
    await s.init()
    api.pause.mockResolvedValue(true)
    api.status.mockResolvedValue(status({ scanning: true, paused: true }))
    await s.pause()
    expect(s.paused).toBe(true)
    api.resume.mockResolvedValue(true)
    api.status.mockResolvedValue(status({ scanning: true }))
    await s.resume()
    expect(s.paused).toBe(false)
    api.cancel.mockResolvedValue(true)
    api.status.mockResolvedValue(status())
    await s.cancel()
    expect(s.scanning).toBe(false)
  })

  it('selects with toggle and shift range, and clears on navigation', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggle('C:\\r\\a')
    s.toggle('C:\\r\\c', true)
    expect([...s.selected].sort()).toEqual(['C:\\r\\a', 'C:\\r\\b', 'C:\\r\\c'])
    expect(s.selectedSize).toBe(600)
    s.toggle('C:\\r\\b')
    expect(s.selected.has('C:\\r\\b')).toBe(false)
    await s.open('C:\\r\\a')
    expect(s.selected.size).toBe(0)
    expect(s.canUp).toBe(true)
  })

  it('goes up only inside the indexed roots', async () => {
    const s = useDiskMapStore()
    await s.init()
    await s.open('C:\\r\\a')
    await s.up()
    expect(s.path).toBe('C:\\r')
    await s.up()
    expect(s.path).toBe('C:\\r')
  })

  it('updates the tree, volumes and history after a delete and reports freed size', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggle('C:\\r\\a')
    s.toggle('C:\\r\\b')
    const res: DiskDeleteResult = { done: [{ path: 'C:\\r\\a', size: 300 }], failed: [{ path: 'C:\\r\\b', reason: 'busy' }], freed: 300, permanent: false }
    api.del.mockResolvedValue(res)
    api.children.mockImplementation(async (p: string) => listing(p, [entry('b', 200), entry('c', 100)]))
    api.history.mockResolvedValue([{ at: 1, action: 'trash', count: 1, freed: 300, paths: ['C:\\r\\a'], failed: 1 }])
    await s.removeSelected(false)
    expect(api.del).toHaveBeenCalledWith(['C:\\r\\a', 'C:\\r\\b'], false)
    expect(s.report).toMatchObject({ freed: 300, done: 1, permanent: false })
    expect(s.report?.failed).toHaveLength(1)
    expect([...s.selected]).toEqual(['C:\\r\\b'])
    expect(s.items.map((i) => i.name)).toEqual(['b', 'c'])
    expect(s.totalFreed).toBe(300)
  })

  it('keeps everything when the native confirm is cancelled', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggle('C:\\r\\a')
    api.del.mockResolvedValue({ done: [], failed: [], freed: 0, permanent: true, cancelled: true })
    await s.removeSelected(true)
    expect(s.report).toBeNull()
    expect(s.selected.size).toBe(1)
    expect(s.deleting).toBe(false)
  })

  it('reports an error when the bridge cannot delete', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggle('C:\\r\\a')
    api.del.mockResolvedValue(null)
    await s.removeSelected(false)
    expect(s.report?.failed).toHaveLength(1)
    expect(s.report?.done).toBe(0)
    expect(s.deleting).toBe(false)
  })

  it('ignores a stale folder listing that arrives late', async () => {
    const s = useDiskMapStore()
    await s.init()
    let release: (v: DiskChildren) => void = () => {}
    api.children.mockImplementationOnce(() => new Promise<DiskChildren>((r) => (release = r)))
    const slow = s.open('C:\\r\\slow')
    api.children.mockImplementationOnce(async (p: string) => listing(p, [entry('fast', 1)]))
    await s.open('C:\\r\\fast')
    release(listing('C:\\r\\slow', [entry('late', 9)]))
    await slow
    expect(s.path).toBe('C:\\r\\fast')
    expect(s.items.map((i) => i.name)).toEqual(['fast'])
  })

  it('caches sub listings for the sunburst and persists level and chart', async () => {
    const s = useDiskMapStore()
    await s.init()
    await s.loadSub(['C:\\r\\a'])
    await s.loadSub(['C:\\r\\a'])
    expect(api.children.mock.calls.filter((c) => c[0] === 'C:\\r\\a')).toHaveLength(1)
    expect(s.sub.get('C:\\r\\a')).toHaveLength(3)
    s.setLevel('light')
    s.setChart('sunburst')
    expect(localStorage.getItem('ccm.diskLevel')).toBe('light')
    expect(localStorage.getItem('ccm.diskChart')).toBe('sunburst')
  })

  it('reveals a path through the bridge', async () => {
    const s = useDiskMapStore()
    await s.reveal('C:\\r\\a')
    expect(api.reveal).toHaveBeenCalledWith('C:\\r\\a')
  })
})
