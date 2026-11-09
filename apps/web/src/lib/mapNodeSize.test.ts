import { describe, expect, it } from 'vitest'
import { MAIN_HEIGHT, MAIN_WIDTH, PILL_HEIGHT, PILL_WIDTH, TRAY_WIDTH } from './layout'
import { nodeSize, SeenNodes } from './mapNodeSize'

describe('nodeSize', () => {
  it('uses measured dimensions first', () => {
    expect(nodeSize({ id: 'a', type: 'main', dimensions: { width: 310, height: 120 } }, { a: { w: 400, h: 200 } })).toEqual({ width: 310, height: 120 })
  })

  it('falls back to the cached size when the node was never measured', () => {
    expect(nodeSize({ id: 'a', type: 'main', dimensions: { width: 0, height: 0 } }, { a: { w: 400, h: 200 } })).toEqual({ width: 400, height: 200 })
  })

  it('mixes a partial cache with the type default', () => {
    expect(nodeSize({ id: 'a', type: 'sub' }, { a: { w: 260 } })).toEqual({ width: 260, height: PILL_HEIGHT })
  })

  it('falls back to the type default without measure or cache', () => {
    expect(nodeSize({ id: 'a', type: 'main' })).toEqual({ width: MAIN_WIDTH, height: MAIN_HEIGHT })
    expect(nodeSize({ id: 'p', type: 'sub', dimensions: { width: 0, height: 0 } })).toMatchObject({ width: PILL_WIDTH })
    expect(nodeSize({ id: 't', type: 'tray' }).width).toBe(TRAY_WIDTH)
  })
})

describe('SeenNodes', () => {
  it('reports a node as new only the first time', () => {
    const seen = new SeenNodes()
    expect(seen.take('a')).toBe(true)
    expect(seen.take('a')).toBe(false)
  })

  it('forgets removed ids so a returning agent animates again', () => {
    const seen = new SeenNodes()
    seen.take('a')
    seen.take('b')
    seen.prune(new Set(['b']))
    expect(seen.take('a')).toBe(true)
    expect(seen.take('b')).toBe(false)
  })
})
