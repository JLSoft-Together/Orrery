import { describe, expect, it } from 'vitest'
import { compileQuery, joinRows, MAX_ROWS, runSearch, searchDump, searchLines, type TermDump } from './search'

const dump = (lines: string[], firstRow = 0, cols = 80): TermDump => ({ rows: lines.map((text) => ({ text, wrapped: false })), firstRow, cols })
const now = (fn: () => void): void => fn()
const re = (q: string, regex = false, caseSensitive = false): RegExp => {
  const c = compileQuery(q, { regex, caseSensitive })
  if (!c || !c.ok) throw new Error('bad')
  return c.re
}

describe('compileQuery', () => {
  it('escapes plain text and honours case', () => {
    expect(re('a.b').test('axb')).toBe(false)
    expect(re('a.b').test('A.B')).toBe(true)
    expect(re('a.b', false, true).test('A.B')).toBe(false)
  })

  it('reports an invalid regex instead of throwing', () => {
    const c = compileQuery('(oops', { regex: true, caseSensitive: false })
    expect(c?.ok).toBe(false)
    expect(compileQuery('(oops', { regex: false, caseSensitive: false })?.ok).toBe(true)
    expect(compileQuery('', { regex: true, caseSensitive: false })).toBeNull()
  })
})

describe('searchLines', () => {
  it('gives context lines around the match and counts every match', () => {
    const lines = joinRows(dump(['start', 'one', 'connect ECONNREFUSED :5432 ECONNREFUSED', 'after', 'end'], 100))
    const { hits, count, more } = searchLines(lines, re('econnrefused'), 80, 10)
    expect(more).toBe(false)
    expect(count).toBe(2)
    expect(hits).toHaveLength(1)
    expect(hits[0]).toMatchObject({ row: 102, col: 8, len: 12, before: ['one'], after: ['after'], count: 2 })
    expect(hits[0]!.ranges).toEqual([
      { start: 8, len: 12 },
      { start: 27, len: 12 },
    ])
  })

  it('context stops at buffer edges', () => {
    const { hits } = searchLines(joinRows(dump(['hit first', 'x', 'hit last'])), re('hit'), 80, 10)
    expect(hits[0]!.before).toEqual([])
    expect(hits[1]!.after).toEqual([])
  })

  it('joins wrapped rows and maps the column back to the row it sits on', () => {
    const d: TermDump = { rows: [{ text: 'abcdefghij', wrapped: false }, { text: 'kl NEEDLE', wrapped: true }], firstRow: 0, cols: 10 }
    const lines = joinRows(d)
    expect(lines).toEqual([{ text: 'abcdefghijkl NEEDLE', row: 0 }])
    expect(searchLines(lines, re('needle'), 10, 5).hits[0]).toMatchObject({ row: 1, col: 3 })
  })

  it('does not loop on empty regex matches', () => {
    const { hits, count } = searchLines(joinRows(dump(['abc'])), re('x*', true), 80, 5)
    expect(hits).toHaveLength(0)
    expect(count).toBe(0)
  })

  it('stops at the limit and says there is more', () => {
    const r = searchLines(joinRows(dump(Array.from({ length: 20 }, (_, i) => `err ${i}`))), re('err'), 80, 5)
    expect(r.hits).toHaveLength(5)
    expect(r.more).toBe(true)
  })
})

describe('searchDump', () => {
  it('marks a pane without output as not loaded', () => {
    expect(searchDump('a', null, re('x'), 10).status).toBe('unloaded')
    expect(searchDump('a', dump(['', '   ']), re('x'), 10).status).toBe('unloaded')
  })

  it('reads only the last rows of a big buffer', () => {
    const lines = Array.from({ length: MAX_ROWS + 10 }, (_, i) => (i === 0 ? 'needle old' : i === MAX_ROWS + 9 ? 'needle new' : 'x'))
    const r = searchDump('a', dump(lines), re('needle'), 10)
    expect(r.hits.map((h) => h.row)).toEqual([MAX_ROWS + 9])
  })
})

describe('runSearch', () => {
  it('groups results per pane and keeps unloaded panes', async () => {
    const out = await runSearch(
      [
        { key: 'a', read: async () => dump(['foo', 'bar foo']) },
        { key: 'b', read: async () => null },
        { key: 'c', read: async () => dump(['nothing']) },
        { key: 'd', read: async () => { throw new Error('gone') } },
      ],
      re('foo'),
      { schedule: now },
    )
    expect(out?.total).toBe(2)
    expect(out?.truncated).toBe(false)
    expect(out?.panes.map((p) => [p.key, p.status, p.hits.length])).toEqual([
      ['a', 'ok', 2],
      ['b', 'unloaded', 0],
      ['c', 'ok', 0],
      ['d', 'unloaded', 0],
    ])
  })

  it('caps the total across panes', async () => {
    const many = (n: number): TermDump => dump(Array.from({ length: n }, () => 'hit'))
    const out = await runSearch(
      [
        { key: 'a', read: async () => many(4) },
        { key: 'b', read: async () => many(4) },
        { key: 'c', read: async () => many(4) },
      ],
      re('hit'),
      { schedule: now, max: 6 },
    )
    expect(out?.total).toBe(6)
    expect(out?.truncated).toBe(true)
    expect(out?.panes.map((p) => p.hits.length)).toEqual([4, 2, 0])
  })

  it('stops when cancelled', async () => {
    let calls = 0
    const out = await runSearch(
      [
        { key: 'a', read: async () => (calls++, dump(['x'])) },
        { key: 'b', read: async () => (calls++, dump(['x'])) },
      ],
      re('x'),
      { schedule: now, cancelled: () => calls >= 1 },
    )
    expect(out).toBeNull()
    expect(calls).toBe(1)
  })
})
