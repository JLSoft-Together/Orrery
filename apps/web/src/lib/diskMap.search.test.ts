import { describe, expect, it } from 'vitest'
import { BIG_MIN_DEFAULT, buildSearchQuery, dateToMs, defaultFilters, hasCriteria, mbToBytes, monthsAgo, msToDate, SEARCH_LIMIT, shortPath } from './diskMap'

const now = new Date(2026, 10, 15, 12, 0, 0).getTime()
const ctx = { now, path: 'C:\\r\\sub', oldMonths: 12 }

describe('buildSearchQuery', () => {
  it('returns null in browse mode and for an empty search', () => {
    expect(buildSearchQuery('browse', defaultFilters(), ctx)).toBeNull()
    expect(buildSearchQuery('search', defaultFilters(), ctx)).toBeNull()
    expect(buildSearchQuery('search', { ...defaultFilters(), text: '   ' }, ctx)).toBeNull()
  })

  it('maps every filter of the search tab', () => {
    const q = buildSearchQuery(
      'search',
      { text: ' *.mp4 ', kind: 'video', minSize: 100, maxSize: 900, after: 5, before: 9, only: 'file', scoped: true, sort: 'mtime' },
      ctx,
    )
    expect(q).toEqual({ limit: SEARCH_LIMIT, query: '*.mp4', kind: 'video', maxSize: 900, minSize: 100, after: 5, before: 9, only: 'file', under: 'C:\\r\\sub', sort: 'mtime' })
  })

  it('drops scope when no folder is open and ignores zero sizes', () => {
    const q = buildSearchQuery('search', { ...defaultFilters(), text: 'a', scoped: true, minSize: 0, maxSize: 0 }, { ...ctx, path: '' })
    expect(q).toEqual({ limit: SEARCH_LIMIT, query: 'a', sort: 'size' })
  })

  it('big files: files only, sorted by size, default min 100 MB', () => {
    const q = buildSearchQuery('big', defaultFilters(), ctx)!
    expect(q).toMatchObject({ only: 'file', sort: 'size', minSize: BIG_MIN_DEFAULT })
    expect(buildSearchQuery('big', { ...defaultFilters(), minSize: 1024 ** 3 }, ctx)!.minSize).toBe(1024 ** 3)
  })

  it('big files ignore date and only filters', () => {
    const q = buildSearchQuery('big', { ...defaultFilters(), after: 1, before: 2, only: 'dir' }, ctx)!
    expect(q.after).toBeUndefined()
    expect(q.before).toBeUndefined()
    expect(q.only).toBe('file')
  })

  it('old files: before = N months ago, searches even with empty filters', () => {
    for (const n of [3, 6, 12, 24]) {
      const q = buildSearchQuery('old', defaultFilters(), { ...ctx, oldMonths: n })!
      expect(q.before).toBe(monthsAgo(now, n))
      expect(q).toMatchObject({ only: 'file', sort: 'size' })
      expect(q.minSize).toBeUndefined()
    }
    expect(monthsAgo(now, 12)).toBe(new Date(2025, 10, 15, 12, 0, 0).getTime())
  })
})

describe('filter helpers', () => {
  it('hasCriteria', () => {
    expect(hasCriteria(defaultFilters())).toBe(false)
    expect(hasCriteria({ ...defaultFilters(), kind: 'image' })).toBe(true)
    expect(hasCriteria({ ...defaultFilters(), scoped: true })).toBe(false)
  })

  it('date round trip, before covers the whole picked day', () => {
    const start = dateToMs('2026-03-09')!
    expect(msToDate(start)).toBe('2026-03-09')
    const end = dateToMs('2026-03-09', true)!
    expect(end - start).toBe(86_400_000)
    expect(msToDate(end, true)).toBe('2026-03-09')
    expect(dateToMs('nope')).toBeNull()
    expect(msToDate(null)).toBe('')
  })

  it('mbToBytes', () => {
    expect(mbToBytes(1)).toBe(1024 ** 2)
    expect(mbToBytes(0)).toBeNull()
    expect(mbToBytes(-3)).toBeNull()
    expect(mbToBytes(Number.NaN)).toBeNull()
  })

  it('shortPath keeps both ends and the length', () => {
    expect(shortPath('C:\\a', 20)).toBe('C:\\a')
    const long = `C:\\${'x'.repeat(100)}\\file.bin`
    const s = shortPath(long, 40)
    expect(s.length).toBe(40)
    expect(s.startsWith('C:\\')).toBe(true)
    expect(s.endsWith('file.bin')).toBe(true)
    expect(s).toContain('...')
  })
})
