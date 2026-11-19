import { describe, expect, it } from 'vitest'
import { cleanCompare, cleanMetas, cleanSchedule, findRow, indexRows, sizeSign, sortByDelta, snapshotLabel, type DiskDiffRow } from './diskSnap'
import type { DiskEntry } from './diskMap'

const row = (path: string, before: number, after: number, state: DiskDiffRow['state'] = 'changed'): DiskDiffRow => ({ path, name: path.split('\\').pop()!, before, after, delta: after - before, state })
const entry = (name: string, size: number, dir = true): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir, mtime: 1 })

describe('cleanMetas', () => {
  it('keeps valid snapshots only', () => {
    expect(cleanMetas('x')).toEqual([])
    expect(cleanMetas([{ id: 'a', at: 5, bytes: 7, roots: ['C:\\', 3] }, { id: '', at: 1 }, { id: 'b' }, null])).toEqual([{ id: 'a', at: 5, bytes: 7, roots: ['C:\\'] }])
  })
})

describe('cleanCompare', () => {
  it('rebuilds delta from before and after and drops junk rows', () => {
    const c = cleanCompare({ base: { id: 'a', at: 1, bytes: 0, roots: [] }, path: 'C:\\', covered: true, rows: [{ path: 'C:\\x', name: 'x', before: 10, after: 40, delta: 999, state: 'weird' }, { nope: 1 }], top: [] })!
    expect(c.rows).toEqual([{ path: 'C:\\x', name: 'x', before: 10, after: 40, delta: 30, state: 'same' }])
    expect(c.covered).toBe(true)
  })

  it('is null without a base snapshot', () => {
    expect(cleanCompare(null)).toBeNull()
    expect(cleanCompare({ rows: [] })).toBeNull()
  })
})

describe('cleanSchedule', () => {
  it('accepts known frequencies only', () => {
    expect(cleanSchedule({ freq: 'weekly', telegram: true, last: { at: 3, ok: false }, running: true, paused: false })).toEqual({ freq: 'weekly', telegram: true, last: { at: 3, ok: false }, running: true, paused: false })
    expect(cleanSchedule({ freq: 'hourly' })).toBeNull()
    expect(cleanSchedule(null)).toBeNull()
    expect(cleanSchedule({ freq: 'off', last: 'x' })).toMatchObject({ freq: 'off', telegram: false, last: null })
  })
})

describe('findRow', () => {
  it('matches exactly first, then without regard to case', () => {
    const idx = indexRows([row('C:\\r\\Big', 1, 2), row('C:\\r\\big', 5, 9)])
    expect(findRow(idx, 'C:\\r\\Big')?.after).toBe(2)
    expect(findRow(idx, 'C:\\r\\big')?.after).toBe(9)
    expect(findRow(idx, 'C:\\R\\BIG')).toBeDefined()
    expect(findRow(idx, 'C:\\r\\none')).toBeUndefined()
  })
})

describe('sortByDelta', () => {
  it('orders folders by growth, then size, with files last', () => {
    const idx = indexRows([row('C:\\r\\a', 0, 50), row('C:\\r\\b', 100, 20), row('C:\\r\\c', 5, 6)])
    const items = [entry('b', 20), entry('f.bin', 9999, false), entry('c', 6), entry('z', 800), entry('a', 50), entry('y', 700)]
    expect(sortByDelta(items, idx).map((i) => i.name)).toEqual(['a', 'c', 'z', 'y', 'b', 'f.bin'])
    expect(items.map((i) => i.name)[0]).toBe('b')
  })
})

describe('helpers', () => {
  it('sizeSign', () => {
    expect([sizeSign(5), sizeSign(-5), sizeSign(0)]).toEqual(['up', 'down', 'same'])
  })

  it('snapshotLabel prints date and time', () => {
    expect(snapshotLabel({ id: 'a', at: Date.UTC(2026, 10, 15, 12, 30), bytes: 0, roots: [] }, 'en-GB')).toMatch(/15\/11\/2026/)
  })
})
