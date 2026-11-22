import { describe, expect, it } from 'vitest'
import type { TreeRow } from './tree'
import { rowContentWidth, treeContentWidth } from './treeWidth'

const row = (name: string, depth: number): TreeRow => ({ name, depth, path: `/${name}`, directory: false, expanded: false }) as TreeRow
const opts = { indent: 14, measure: (text: string) => text.length * 7 }

describe('treeWidth', () => {
  it('grows with depth and name length', () => {
    expect(rowContentWidth(row('a.ts', 1), opts)).toBeLessThan(rowContentWidth(row('a.ts', 9), opts))
    expect(rowContentWidth(row('a.ts', 3), opts)).toBeLessThan(rowContentWidth(row('a-much-longer-file-name.ts', 3), opts))
  })
  it('takes the widest row and skips the root', () => {
    const rows = [row('root-with-a-very-long-name-that-is-ignored', 0), row('x', 1), row('deep.ts', 12)]
    expect(treeContentWidth(rows, opts)).toBe(rowContentWidth(rows[2]!, opts))
  })
  it('adds the per row extra', () => {
    expect(rowContentWidth(row('a', 1), { ...opts, extra: () => 50 })).toBe(rowContentWidth(row('a', 1), opts) + 50)
  })
  it('is zero without rows', () => {
    expect(treeContentWidth([], opts)).toBe(0)
  })
})
