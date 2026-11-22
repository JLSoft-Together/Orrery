import { describe, expect, it } from 'vitest'
import { addSkipped, isCramped, parseSkipped, splitSuggestion, SKIP_MAX } from './layoutSplitHint'

const p = (key: string, w: number, h: number) => ({ key, w, h })

describe('splitSuggestion', () => {
  it('needs five panes', () => {
    expect(splitSuggestion([p('a', 100, 100), p('b', 100, 100), p('c', 100, 100), p('d', 100, 100)])).toBeNull()
  })

  it('no hint when every pane has room', () => {
    expect(splitSuggestion(['a', 'b', 'c', 'd', 'e'].map((k) => p(k, 400, 200)))).toBeNull()
  })

  it('suggests the cramped panes', () => {
    const list = [p('a', 800, 800), p('b', 500, 160), p('c', 500, 160), p('d', 500, 300), p('e', 500, 300)]
    expect(splitSuggestion(list)).toEqual(['b', 'c'])
  })

  it('takes the second half when all are cramped', () => {
    const list = ['a', 'b', 'c', 'd', 'e', 'f'].map((k) => p(k, 300, 300))
    expect(splitSuggestion(list)).toEqual(['d', 'e', 'f'])
  })

  it('waits for the canvas to be measured', () => {
    expect(splitSuggestion(['a', 'b', 'c', 'd', 'e'].map((k) => p(k, 0, 0)))).toBeNull()
  })

  it('uses 360 x 180 as the line', () => {
    expect(isCramped({ w: 359, h: 400 })).toBe(true)
    expect(isCramped({ w: 360, h: 179 })).toBe(true)
    expect(isCramped({ w: 360, h: 180 })).toBe(false)
  })
})

describe('skip list', () => {
  it('parses safely', () => {
    expect(parseSkipped(null)).toEqual([])
    expect(parseSkipped('nope')).toEqual([])
    expect(parseSkipped('{"a":1}')).toEqual([])
    expect(parseSkipped('["t1", 2, "t2"]')).toEqual(['t1', 't2'])
  })

  it('adds once and keeps the newest', () => {
    expect(addSkipped(['a', 'b'], 'a')).toEqual(['b', 'a'])
    const many = Array.from({ length: SKIP_MAX }, (_, i) => `t${i}`)
    const out = addSkipped(many, 'new')
    expect(out).toHaveLength(SKIP_MAX)
    expect(out.at(-1)).toBe('new')
    expect(out[0]).toBe('t1')
  })
})
