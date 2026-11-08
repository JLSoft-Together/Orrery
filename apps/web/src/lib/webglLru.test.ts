import { describe, expect, it } from 'vitest'
import { WebglLru } from './webglLru'

function pool() {
  const lru = new WebglLru(4)
  const released: string[] = []
  const show = (key: string): void => lru.show(key, () => released.push(key))
  return { lru, released, show }
}

describe('WebglLru', () => {
  it('six panes shown in turn keep WebGL on the four most recent only', () => {
    const { lru, released, show } = pool()
    const keys = ['a', 'b', 'c', 'd', 'e', 'f']
    keys.forEach((k, i) => {
      show(k)
      if (i > 0) lru.hide(keys[i - 1]!)
    })
    expect(released).toEqual(['a', 'b'])
    expect(lru.keys).toEqual(['f', 'e', 'd', 'c'])
  })

  it('a pane shown again moves to the front and is not the next one evicted', () => {
    const { lru, released, show } = pool()
    for (const k of ['a', 'b', 'c', 'd']) {
      show(k)
      lru.hide(k)
    }
    show('a')
    lru.hide('a')
    expect(lru.keys[0]).toBe('a')
    show('e')
    expect(released).toEqual(['b'])
    expect(lru.has('a')).toBe(true)
  })

  it('never evicts a pane still on screen', () => {
    const { lru, released, show } = pool()
    for (const k of ['a', 'b', 'c', 'd', 'e', 'f']) show(k)
    expect(released).toEqual([])
    lru.hide('a')
    expect(released).toEqual(['a'])
    expect(lru.keys).toHaveLength(5)
  })

  it('drop forgets a pane without calling its release', () => {
    const { lru, released, show } = pool()
    show('a')
    lru.drop('a')
    expect(lru.has('a')).toBe(false)
    expect(released).toEqual([])
    lru.hide('a')
    expect(lru.keys).toEqual([])
  })
})
