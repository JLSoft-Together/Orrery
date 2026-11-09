import { describe, expect, it } from 'vitest'
import type { Edge } from '@vue-flow/core'
import { stableEdges } from './stableEdges'

const edge = (over: Partial<Edge> = {}): Edge => ({ id: 'a->b', source: 'a', target: 'b', type: 'tree', data: { pulse: undefined }, ...over })

describe('stableEdges', () => {
  it('keeps the previous array when nothing an edge shows changed', () => {
    const prev = [edge(), edge({ id: 'a->c', target: 'c' })]
    const next = [edge(), edge({ id: 'a->c', target: 'c' })]
    expect(stableEdges(prev, next)).toBe(prev)
  })

  it('takes the new array when a class, pulse or id changes', () => {
    const prev = [edge()]
    expect(stableEdges(prev, [edge({ class: 'ccm-edge-hot' })])).not.toBe(prev)
    expect(stableEdges(prev, [edge({ data: { pulse: { n: 1, dir: 'up' } } })])).not.toBe(prev)
    expect(stableEdges(prev, [edge({ id: 'x' })])).not.toBe(prev)
    expect(stableEdges(prev, [])).not.toBe(prev)
  })

  it('has nothing to keep on the first run', () => {
    const next = [edge()]
    expect(stableEdges(undefined, next)).toBe(next)
  })
})
