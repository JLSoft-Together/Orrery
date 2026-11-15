import { describe, expect, it } from 'vitest'
import {
  breadcrumbs,
  diskMapChildren,
  diskMapDelete,
  diskMapHistory,
  diskMapScan,
  diskMapStatus,
  diskMapVolumes,
  formatDuration,
  formatPercent,
  formatSize,
  groupOf,
  hasDiskMap,
  isUnder,
  loadChart,
  loadLevel,
  onDiskMapDone,
  onDiskMapProgress,
  parentPath,
  saveChart,
  saveLevel,
  topSlices,
  volumeUsedPct,
  type DiskEntry,
} from './diskMap'

const e = (name: string, size: number, dir = false): DiskEntry => ({ path: `C:\\x\\${name}`, name, size, dir, mtime: 0 })

describe('bridge without disk map methods', () => {
  it('reports unsupported and answers with safe defaults', async () => {
    expect(hasDiskMap).toBe(false)
    expect(await diskMapVolumes()).toEqual([])
    expect(await diskMapStatus()).toBeNull()
    expect(await diskMapScan(['C:\\'], 'balanced')).toBe(false)
    expect(await diskMapChildren('C:\\')).toBeNull()
    expect(await diskMapDelete(['C:\\a'], false)).toBeNull()
    expect(await diskMapHistory()).toEqual([])
    expect(onDiskMapProgress(() => {})()).toBeUndefined()
    expect(onDiskMapDone(() => {})()).toBeUndefined()
  })
})

describe('format helpers', () => {
  it('formats sizes', () => {
    expect(formatSize(0)).toBe('0 B')
    expect(formatSize(1536)).toBe('1.5 KB')
    expect(formatSize(5 * 1024 ** 3)).toBe('5 GB')
    expect(formatSize(-1)).toBe('-')
    expect(formatSize(NaN)).toBe('-')
    expect(formatSize(null)).toBe('-')
  })

  it('formats percent and duration', () => {
    expect(formatPercent(0, 100)).toBe('0%')
    expect(formatPercent(1, 0)).toBe('0%')
    expect(formatPercent(1, 10000)).toBe('<0.1%')
    expect(formatPercent(5, 100)).toBe('5.0%')
    expect(formatPercent(50, 100)).toBe('50%')
    expect(formatDuration(65_000)).toBe('1:05')
    expect(formatDuration(3_725_000)).toBe('1:02:05')
  })

  it('computes volume usage', () => {
    expect(volumeUsedPct({ total: 100, free: 25 })).toBe(75)
    expect(volumeUsedPct({ total: 0, free: 0 })).toBe(0)
  })
})

describe('groupOf', () => {
  it('uses dir, kind and extension', () => {
    expect(groupOf({ dir: true, name: 'a.png' })).toBe('dir')
    expect(groupOf({ dir: false, name: 'a.bin', kind: 'media' })).toBe('media')
    expect(groupOf({ dir: false, name: 'a.zip' })).toBe('archive')
    expect(groupOf({ dir: false, name: 'noext' })).toBe('other')
    expect(groupOf({ dir: false, name: 'x.unknownext' })).toBe('other')
  })
})

describe('topSlices', () => {
  it('returns nothing for empty input and skips zero-byte items', () => {
    expect(topSlices([], 10, 0)).toEqual([])
    expect(topSlices([e('empty', 0, true)], 10, 0)).toEqual([])
  })

  it('merges items beyond the cap into one rest slice', () => {
    const items = Array.from({ length: 5 }, (_, i) => e(`f${i}`, 10 * (i + 1)))
    const out = topSlices(items, 3, 150)
    expect(out).toHaveLength(4)
    expect(out[3]!.entry).toBeNull()
    expect(out[3]!.size).toBe(30)
    expect(out[3]!.more).toBe(2)
  })

  it('adds the size the list did not cover to the rest slice', () => {
    const out = topSlices([e('a', 60)], 10, 100)
    expect(out[1]).toMatchObject({ entry: null, size: 40, more: 0 })
  })
})

describe('paths', () => {
  it('finds parents on Windows and POSIX', () => {
    expect(parentPath('C:\\a\\b')).toBe('C:\\a')
    expect(parentPath('C:\\a')).toBe('C:\\')
    expect(parentPath('/usr/lib')).toBe('/usr')
    expect(parentPath('/usr')).toBe('/')
    expect(parentPath('C:\\')).toBeNull()
  })

  it('builds breadcrumbs', () => {
    expect(breadcrumbs('C:\\Users\\a').map((c) => c.path)).toEqual(['C:\\', 'C:\\Users', 'C:\\Users\\a'])
    expect(breadcrumbs('/usr/lib').map((c) => c.path)).toEqual(['/', '/usr', '/usr/lib'])
    expect(breadcrumbs('')).toEqual([])
  })

  it('checks containment case-insensitively and by segment', () => {
    expect(isUnder('C:\\Data\\x', 'c:\\data')).toBe(true)
    expect(isUnder('C:\\Database', 'C:\\Data')).toBe(false)
    expect(isUnder('C:\\Data', 'C:\\Data\\')).toBe(true)
  })
})

describe('persisted choices', () => {
  it('round-trips level and chart and falls back to defaults', () => {
    localStorage.clear()
    expect(loadLevel()).toBe('balanced')
    saveLevel('max')
    expect(loadLevel()).toBe('max')
    expect(loadChart()).toBe('treemap')
    saveChart('sunburst')
    expect(loadChart()).toBe('sunburst')
    localStorage.setItem('ccm.diskLevel', 'bogus')
    expect(loadLevel()).toBe('balanced')
  })
})
