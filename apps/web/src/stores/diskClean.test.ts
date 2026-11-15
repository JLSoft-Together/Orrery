import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { CleanItem, CleanProgress } from '@ccm/shared'

const api = vi.hoisted(() => ({
  categories: vi.fn(),
  scan: vi.fn(),
  cancel: vi.fn(),
  del: vi.fn(),
  progress: null as null | ((p: CleanProgress) => void),
}))

vi.mock('../lib/diskClean', async (orig) => ({
  ...(await orig<typeof import('../lib/diskClean')>()),
  hasDiskClean: true,
  diskCleanCategories: () => api.categories(),
  diskCleanScan: (...a: unknown[]) => api.scan(...a),
  diskCleanCancel: () => api.cancel(),
  diskCleanDelete: (...a: unknown[]) => api.del(...a),
  onDiskCleanProgress: (cb: (p: CleanProgress) => void) => {
    api.progress = cb
    return () => {
      api.progress = null
    }
  },
}))

import { useDiskCleanStore } from './diskClean'

const DAY = 86_400_000
const now = Date.now()
const it_ = (path: string, over: Partial<CleanItem> = {}): CleanItem => ({ path, size: 100, group: 'dev', category: 'node_modules', project: path.replace(/\\[^\\]+$/, ''), ...over })

const items: CleanItem[] = [
  it_('C:\\old\\node_modules', { lastTouched: now - 60 * DAY }),
  it_('C:\\new\\node_modules', { lastTouched: now - 1 * DAY }),
  it_('C:\\busy\\build', { category: 'build', lastTouched: now - 90 * DAY }),
  it_('C:\\Users\\u\\.gradle\\caches', { category: 'cache', project: undefined, size: 5000 }),
]

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  vi.clearAllMocks()
  api.categories.mockResolvedValue([
    { id: 'node_modules', group: 'dev', preselect: true, titleKey: 'a', hintKey: 'b' },
    { id: 'build', group: 'dev', preselect: true, titleKey: 'a', hintKey: 'b' },
    { id: 'cache', group: 'dev', preselect: false, titleKey: 'a', hintKey: 'b' },
  ])
  api.scan.mockResolvedValue({ items })
})

describe('diskClean store', () => {
  it('scan ticks only stale, unlocked project folders; caches never', async () => {
    const s = useDiskCleanStore()
    await s.scan(['C:\\old', 'C:\\new', 'C:\\busy'], ['C:\\busy\\lib'])
    expect(api.scan).toHaveBeenCalledWith(['node_modules', 'build'], ['C:\\old', 'C:\\new', 'C:\\busy'], ['C:\\busy\\lib'])
    expect([...s.selected]).toEqual(['C:\\old\\node_modules'])
    expect(s.byCategory.get('cache')).toHaveLength(1)
  })

  it('a locked row cannot be ticked, alone or with its group', async () => {
    const s = useDiskCleanStore()
    await s.scan(['C:\\busy'], ['C:\\busy'])
    s.toggle('C:\\busy\\build')
    expect(s.selected.has('C:\\busy\\build')).toBe(false)
    s.toggleCategory('build')
    expect(s.selected.has('C:\\busy\\build')).toBe(false)
    s.toggle('C:\\Users\\u\\.gradle\\caches')
    expect(s.selected.has('C:\\Users\\u\\.gradle\\caches')).toBe(true)
  })

  it('rechecks locks right before delete: a project opened after the scan is not sent', async () => {
    const s = useDiskCleanStore()
    await s.scan(['C:\\old', 'C:\\new'], [])
    s.toggle('C:\\new\\node_modules')
    expect(s.selected.size).toBe(3)
    api.del.mockResolvedValue({ done: [{ path: 'C:\\old\\node_modules', size: 100 }], failed: [], freed: 100, permanent: false })
    await s.removeSelected(false, ['C:\\new'])
    expect(api.del).toHaveBeenCalledWith(['C:\\old\\node_modules', 'C:\\busy\\build'], false, ['C:\\new'])
    expect(s.items.some((i) => i.path === 'C:\\old\\node_modules')).toBe(false)
    expect(s.selected.has('C:\\new\\node_modules')).toBe(false)
    expect(s.report).toMatchObject({ done: 1, freed: 100 })
  })

  it('sends nothing when every pick is locked', async () => {
    const s = useDiskCleanStore()
    await s.scan(['C:\\old'], [])
    expect(await s.removeSelected(true, ['C:\\old', 'C:\\busy'])).toBeNull()
    expect(api.del).not.toHaveBeenCalled()
  })

  it('caches are measured only on demand and merged without dropping picks', async () => {
    const s = useDiskCleanStore()
    api.scan.mockResolvedValueOnce({ items: items.slice(0, 3) })
    await s.scan(['C:\\old'], [])
    expect(s.measured.has('cache')).toBe(false)
    const before = [...s.selected]
    api.scan.mockResolvedValueOnce({ items: [items[3]!] })
    await s.measure('cache', [])
    expect(api.scan).toHaveBeenLastCalledWith(['cache'], [], [])
    expect(s.measured.has('cache')).toBe(true)
    expect(s.items).toHaveLength(4)
    expect([...s.selected]).toEqual(before)
    api.scan.mockResolvedValueOnce({ items: [], cancelled: true })
    s.measured = new Set()
    await s.measure('cache', [])
    expect(s.measured.has('cache')).toBe(false)
  })

  it('openFor filters a project and scans only its project groups', async () => {
    const s = useDiskCleanStore()
    s.openFor('C:\\old')
    expect(s.active).toBe(true)
    await s.scan(['C:\\a', 'C:\\b'], [])
    expect(api.scan).toHaveBeenCalledWith(['node_modules', 'build'], ['C:\\old'], [])
    expect(s.visible.map((i) => i.path)).toEqual(['C:\\old\\node_modules'])
  })

  it('streams items from progress while scanning and respects the days setting', async () => {
    const s = useDiskCleanStore()
    s.setDays(0.4)
    expect(s.days).toBe(1)
    let release: (v: unknown) => void = () => undefined
    api.scan.mockReturnValue(new Promise((r) => (release = r)))
    const p = s.scan(['C:\\new'], [])
    await vi.waitFor(() => expect(api.scan).toHaveBeenCalled())
    api.progress!({ category: 'node_modules', state: 'items', items: [items[1]!], count: 1, bytes: 100 })
    expect(s.items).toHaveLength(1)
    release({ items: [items[1]!] })
    await p
    expect([...s.selected]).toEqual(['C:\\new\\node_modules'])
  })
})
