import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { CleanCategory, CleanItem } from '@ccm/shared'
import { oneTapItems, pickable, preselect } from '../lib/diskClean'

const api = vi.hoisted(() => ({
  categories: vi.fn(),
  scan: vi.fn(),
  del: vi.fn(),
  bin: vi.fn(),
}))

vi.mock('../lib/diskClean', async (orig) => ({
  ...(await orig<typeof import('../lib/diskClean')>()),
  hasDiskClean: true,
  diskCleanCategories: () => api.categories(),
  diskCleanScan: (...a: unknown[]) => api.scan(...a),
  diskCleanCancel: async () => true,
  diskCleanDelete: (...a: unknown[]) => api.del(...a),
  diskCleanEmptyBin: () => api.bin(),
  onDiskCleanProgress: () => () => undefined,
}))

import { useDiskCleanStore } from './diskClean'

const DAY = 86_400_000
const now = Date.now()
const cat = (id: string, group: 'dev' | 'system', pre = true): CleanCategory => ({ id, group, preselect: pre, titleKey: `diskClean.cat.${id}.title`, hintKey: `diskClean.cat.${id}.hint` })
const CATS = [cat('node_modules', 'dev'), cat('build', 'dev'), cat('cache', 'dev', false), cat('temp', 'system'), cat('browser', 'system'), cat('crash', 'system'), cat('logs', 'system'), cat('installers', 'system', false), cat('bin', 'system', false), cat('update', 'system', false)]
const sys = (path: string, category: string, over: Partial<CleanItem> = {}): CleanItem => ({ path, size: 100, group: 'system', category, ...over })
const ITEMS: CleanItem[] = [
  sys('C:\\T', 'temp', { action: 'files', files: 3 }),
  sys('C:\\L\\Chrome\\Default\\Cache', 'browser', { kind: 'chrome' }),
  sys('C:\\L\\Edge\\Default\\Cache', 'browser', { kind: 'edge', locked: 'browser' }),
  sys('C:\\W\\Minidump', 'crash', { locked: 'admin' }),
  sys('C:\\L\\CrashDumps', 'crash', { action: 'files' }),
  sys('C:\\D\\setup.exe', 'installers', { file: true, lastTouched: now - 90 * DAY }),
  sys('C:\\$Recycle.Bin', 'bin', { action: 'bin', size: 999 }),
  sys('C:\\W\\SoftwareDistribution\\Download', 'update', { locked: 'admin' }),
  { path: 'C:\\old\\node_modules', size: 50, group: 'dev', category: 'node_modules', project: 'C:\\old', lastTouched: now - 60 * DAY },
  { path: 'C:\\new\\node_modules', size: 50, group: 'dev', category: 'node_modules', project: 'C:\\new', lastTouched: now - DAY },
  { path: 'C:\\open\\build', size: 50, group: 'dev', category: 'build', project: 'C:\\open', lastTouched: now - 90 * DAY },
  { path: 'C:\\py\\.venv', size: 50, group: 'dev', category: 'build', kind: 'python', project: 'C:\\py', lastTouched: now - 90 * DAY },
  { path: 'C:\\U\\.gradle\\caches', size: 5000, group: 'dev', category: 'cache' },
]

describe('system junk selection rules', () => {
  it('admin, browser-open, bin and denied places are never pickable', () => {
    expect(ITEMS.filter((i) => !pickable(i, [])).map((i) => i.path)).toEqual(['C:\\L\\Edge\\Default\\Cache', 'C:\\W\\Minidump', 'C:\\$Recycle.Bin', 'C:\\W\\SoftwareDistribution\\Download'])
  })

  it('preselect ticks safe system groups without an age, never installers, caches, .venv or locked rows', () => {
    const picked = [...preselect(ITEMS, CATS, 30, now, ['C:\\open'])].sort()
    expect(picked).toEqual(['C:\\L\\Chrome\\Default\\Cache', 'C:\\L\\CrashDumps', 'C:\\T', 'C:\\old\\node_modules'].sort())
  })

  it('one tap takes only safe groups: no tool cache, installer, bin, admin, open project, young project or .venv', () => {
    const tap = oneTapItems(ITEMS, 30, now, ['C:\\open']).map((i) => i.path).sort()
    expect(tap).toEqual(['C:\\L\\Chrome\\Default\\Cache', 'C:\\L\\CrashDumps', 'C:\\T', 'C:\\old\\node_modules'].sort())
  })
})

describe('store one tap', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    api.categories.mockReset().mockResolvedValue(CATS)
    api.scan.mockReset().mockResolvedValue({ items: ITEMS })
    api.del.mockReset()
    api.bin.mockReset()
  })

  it('scans the one tap groups, lets a group be unticked and sends a oneTap delete', async () => {
    const s = useDiskCleanStore()
    await s.startOneTap(['C:\\work'], ['C:\\open'])
    expect(api.scan.mock.calls[0]![0]).toEqual(['node_modules', 'build', 'temp', 'browser', 'crash', 'logs'])
    expect(s.tap).toBe('ready')
    expect(s.tapGroups.map((g) => g.id)).toEqual(['node_modules', 'temp', 'browser', 'crash'])
    expect(s.tapSize).toBe(350)
    s.toggleTap('browser')
    expect(s.tapSize).toBe(250)
    api.del.mockResolvedValue({ done: [{ path: 'C:\\T', size: 100 }], failed: [], freed: 100, permanent: true })
    await s.runOneTap(['C:\\open'])
    const [paths, permanent, open, oneTap] = api.del.mock.calls[0]!
    expect((paths as string[]).sort()).toEqual(['C:\\L\\CrashDumps', 'C:\\T', 'C:\\old\\node_modules'].sort())
    expect([permanent, open, oneTap]).toEqual([false, ['C:\\open'], true])
    expect(s.tap).toBe('idle')
    expect(s.report?.freed).toBe(100)
    expect(s.items.some((i) => i.path === 'C:\\T')).toBe(false)
  })

  it('a project opened after the scan drops out of one tap', async () => {
    const s = useDiskCleanStore()
    await s.startOneTap(['C:\\work'], [])
    api.del.mockResolvedValue({ done: [], failed: [], freed: 0, permanent: false })
    await s.runOneTap(['C:\\old'])
    expect(api.del.mock.calls[0]![0]).not.toContain('C:\\old\\node_modules')
  })

  it('cancel in the dialog keeps the panel open', async () => {
    const s = useDiskCleanStore()
    await s.startOneTap([], [])
    api.del.mockResolvedValue({ done: [], failed: [], freed: 0, permanent: false, cancelled: true })
    await s.runOneTap([])
    expect(s.tap).toBe('ready')
  })

  it('bin rows cannot be ticked; Empty asks the desktop and zeroes the size', async () => {
    const s = useDiskCleanStore()
    await s.scan([], [])
    s.toggle('C:\\$Recycle.Bin')
    expect(s.selected.has('C:\\$Recycle.Bin')).toBe(false)
    api.bin.mockResolvedValueOnce({ done: false, freed: 0, cancelled: true })
    await s.emptyBin()
    expect(s.items.find((i) => i.action === 'bin')?.size).toBe(999)
    api.bin.mockResolvedValueOnce({ done: true, freed: 999 })
    await s.emptyBin()
    expect(s.items.find((i) => i.action === 'bin')?.size).toBe(0)
    expect(s.report).toMatchObject({ done: 1, freed: 999 })
  })
})
