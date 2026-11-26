import { describe, expect, it } from 'vitest'
import { firstLine, paletteRows } from './palettePrompts'
import type { SavedPrompt } from './promptModel'

const p = (id: string, title: string, body: string, scope = 'all'): SavedPrompt => ({ id, title, body, scope, updatedAt: 1 })
const items = [p('a', 'Review diff', 'Review the diff of {branch}'), p('b', 'Fix build', 'Run the build and fix errors', 'D:\\work\\app'), p('c', 'Other repo', 'Fix the lint', 'D:\\work\\other'), p('d', 'Giải thích lỗi', 'Explain the error')]

describe('paletteRows', () => {
  it('lists everything for an empty query, current project first, other projects last', () => {
    const rows = paletteRows(items, '', 'D:\\work\\app\\src')
    expect(rows.map((r) => r.prompt.id)).toEqual(['b', 'a', 'd', 'c'])
    expect(rows[0]!.local).toBe(true)
  })
  it('matches title or body, every word, ignoring accents', () => {
    expect(paletteRows(items, 'giai thich', undefined).map((r) => r.prompt.id)).toEqual(['d'])
    expect(paletteRows(items, 'lint fix', undefined).map((r) => r.prompt.id)).toEqual(['c'])
    expect(paletteRows(items, 'nothing here', undefined)).toEqual([])
  })
  it('puts a title match before a body match', () => {
    expect(paletteRows([p('x', 'Plain', 'fix it'), p('y', 'Fix it', 'other')], 'fix', undefined).map((r) => r.prompt.id)).toEqual(['y', 'x'])
  })
  it('shows the project name in the hint of a scoped prompt', () => {
    expect(paletteRows(items, 'build', 'D:\\work\\app')[0]!.hint.startsWith('app: ')).toBe(true)
  })
})

describe('firstLine', () => {
  it('takes the first non-empty line and flattens spaces', () => {
    expect(firstLine('\n  a   b  \nc')).toBe('a b')
    expect(firstLine('x'.repeat(200), 10)).toHaveLength(10)
  })
})
