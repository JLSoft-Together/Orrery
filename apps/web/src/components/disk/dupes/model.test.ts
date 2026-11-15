import { describe, expect, it } from 'vitest'
import type { DiskDupeGroup } from '../../../lib/diskMap'
import { baseName, defaultTicked, filterGroups, keepOne, keeper, kindOf, pickKeeper, removePaths, tickedPaths, toggleTick, totals, wastedOf } from './model'

const g = (hash: string, size: number, files: [string, number][], count = files.length): DiskDupeGroup => ({ size, hash, count, files: files.map(([path, mtime]) => ({ path, mtime })) })

describe('pickKeeper', () => {
  it('keeps the oldest copy', () => {
    expect(pickKeeper([{ path: 'C:/b/new.bin', mtime: 300 }, { path: 'C:/a/old.bin', mtime: 100 }, { path: 'C:/c/mid.bin', mtime: 200 }])).toBe('C:/a/old.bin')
  })

  it('breaks a tie with the shortest path, then the path text', () => {
    expect(pickKeeper([{ path: 'C:/long/folder/x.bin', mtime: 5 }, { path: 'C:/x.bin', mtime: 5 }])).toBe('C:/x.bin')
    expect(pickKeeper([{ path: 'C:/b.bin', mtime: 5 }, { path: 'C:/a.bin', mtime: 5 }])).toBe('C:/a.bin')
  })

  it('ranks an unknown date last', () => {
    expect(pickKeeper([{ path: 'C:/x.bin', mtime: 0 }, { path: 'C:/yy/y.bin', mtime: 50 }])).toBe('C:/yy/y.bin')
  })
})

describe('ticks', () => {
  const group = g('h1', 100, [['C:/a', 1], ['C:/b', 2], ['C:/c', 3]])

  it('ticks every copy but the keeper', () => {
    expect([...defaultTicked([group])].sort()).toEqual(['C:/b', 'C:/c'])
  })

  it('moves the keeper with keepOne and always leaves exactly one unticked', () => {
    const t = keepOne(defaultTicked([group]), group, 'C:/c')!
    expect([...t].sort()).toEqual(['C:/a', 'C:/b'])
    expect(keeper(t, group)).toBe('C:/c')
    expect(keepOne(t, group, 'C:/zzz')).toBeNull()
  })

  it('refuses to tick the last unticked copy', () => {
    const t = defaultTicked([group])
    expect(toggleTick(t, group, 'C:/a')).toBeNull()
    const free = toggleTick(t, group, 'C:/b')!
    expect([...free]).toEqual(['C:/c'])
    const again = toggleTick(free, group, 'C:/b')!
    expect([...again].sort()).toEqual(['C:/b', 'C:/c'])
    expect(toggleTick(again, group, 'C:/nope')).toBeNull()
  })

  it('counts the bytes that will be freed', () => {
    const second = g('h2', 1000, [['D:/x', 1], ['D:/y', 2]])
    const ticked = defaultTicked([group, second])
    expect(totals([group, second], ticked)).toEqual({ files: 3, bytes: 100 * 2 + 1000 })
    expect(totals([second], ticked)).toEqual({ files: 1, bytes: 1000 })
    expect(tickedPaths([second], ticked)).toEqual(['D:/y'])
  })
})

describe('filters', () => {
  const photos = g('p', 5_000_000, [['C:\\Users\\a\\Pictures\\IMG_1.JPG', 1], ['D:\\backup\\IMG_1.JPG', 2]])
  const video = g('v', 900_000_000, [['C:/Videos/Phim.mkv', 1], ['C:/Old/Phim.mkv', 2]])
  const noext = g('n', 2_000_000, [['C:/x/data', 1], ['C:/y/data', 2]])
  const all = [photos, video, noext]

  it('filters by kind, size and folder text', () => {
    expect(filterGroups(all, { folder: '', kind: '', minSize: 0 })).toHaveLength(3)
    expect(filterGroups(all, { folder: '', kind: 'image', minSize: 0 })).toEqual([photos])
    expect(filterGroups(all, { folder: '', kind: 'video', minSize: 0 })).toEqual([video])
    expect(filterGroups(all, { folder: '', kind: '', minSize: 100_000_000 })).toEqual([video])
    expect(filterGroups(all, { folder: 'BACKUP', kind: '', minSize: 0 })).toEqual([photos])
    expect(filterGroups(all, { folder: 'users/a/pictures', kind: '', minSize: 0 })).toEqual([photos])
    expect(filterGroups(all, { folder: ' nomatch ', kind: '', minSize: 0 })).toEqual([])
  })

  it('classifies by extension', () => {
    expect(kindOf('C:/a/b.MP4')).toBe('video')
    expect(kindOf('C:/a/Báo cáo.PDF')).toBe('doc')
    expect(kindOf('C:/a/noext')).toBe('other')
    expect(kindOf('C:/a.b/noext')).toBe('other')
    expect(baseName('C:\\a\\b\\c.txt')).toBe('c.txt')
  })
})

describe('removePaths', () => {
  it('drops removed copies, keeps counts honest and drops groups with one visible copy', () => {
    const big = g('b', 10, [['a', 1], ['b', 2], ['c', 3]], 5)
    const pair = g('p', 20, [['x', 1], ['y', 2]])
    const out = removePaths([big, pair], new Set(['b', 'y']))
    expect(out).toHaveLength(1)
    expect(out[0]!.files.map((f) => f.path)).toEqual(['a', 'c'])
    expect(out[0]!.count).toBe(4)
    expect(wastedOf(out)).toBe(30)
  })

  it('returns the same list when nothing is gone', () => {
    const list = [g('a', 1, [['p', 1], ['q', 2]])]
    expect(removePaths(list, new Set())).toBe(list)
    expect(removePaths(list, new Set(['zzz']))).toEqual(list)
  })

  it('handles a 10k copy group with hidden copies', () => {
    const files: [string, number][] = Array.from({ length: 500 }, (_, i) => [`C:/f${i}`, i + 1])
    const huge = g('h', 1_000_000, files, 10_000)
    expect(wastedOf([huge])).toBe(1_000_000 * 9_999)
    const t = defaultTicked([huge])
    expect(t.size).toBe(499)
    expect(keeper(t, huge)).toBe('C:/f0')
  })
})
