import { describe, expect, it } from 'vitest'
import { applyJsonPatch, PatchError } from './jsonPatch'

const doc = () => ({ rev: 1, tabs: [{ key: 'a', panes: [{ key: 'p1', title: 'one' }] }], 'a/b': 1, 'm~n': 2 })

describe('applyJsonPatch', () => {
  it('replaces, adds and removes nested values without touching the input', () => {
    const input = doc()
    const out = applyJsonPatch(input, [
      { op: 'replace', path: '/tabs/0/panes/0/title', value: 'two' },
      { op: 'add', path: '/tabs/0/panes/1', value: { key: 'p2', title: 'x' } },
      { op: 'add', path: '/tabs/0/panes/-', value: { key: 'p3' } },
      { op: 'remove', path: '/tabs/0/panes/0' },
    ])
    expect(out.tabs[0]?.panes.map((p) => p.key)).toEqual(['p2', 'p3'])
    expect(input.tabs[0]?.panes).toHaveLength(1)
    expect(input.tabs[0]?.panes[0]?.title).toBe('one')
  })

  it('handles escaped pointer tokens', () => {
    const out = applyJsonPatch(doc(), [
      { op: 'replace', path: '/a~1b', value: 9 },
      { op: 'replace', path: '/m~0n', value: 8 },
    ])
    expect(out['a/b']).toBe(9)
    expect(out['m~n']).toBe(8)
  })

  it('adds into the middle of an array and at the end by index', () => {
    const out = applyJsonPatch({ list: [1, 3] }, [
      { op: 'add', path: '/list/1', value: 2 },
      { op: 'add', path: '/list/3', value: 4 },
    ])
    expect(out.list).toEqual([1, 2, 3, 4])
  })

  it('replaces the root', () => {
    expect(applyJsonPatch({ a: 1 }, [{ op: 'replace', path: '', value: { b: 2 } }])).toEqual({ b: 2 })
  })

  it('copies values so later patches never alias the op payload', () => {
    const value = { k: [1] }
    const out = applyJsonPatch({ x: null as unknown }, [{ op: 'replace', path: '/x', value }])
    value.k.push(2)
    expect(out.x).toEqual({ k: [1] })
  })

  it.each([
    [{ op: 'remove', path: '/nope' }],
    [{ op: 'replace', path: '/nope', value: 1 }],
    [{ op: 'replace', path: '/tabs/5', value: 1 }],
    [{ op: 'add', path: '/tabs/9', value: 1 }],
    [{ op: 'add', path: '/tabs/-1', value: 1 }],
    [{ op: 'add', path: '/tabs/01', value: 1 }],
    [{ op: 'remove', path: '' }],
    [{ op: 'move', path: '/rev', from: '/rev' }],
    [{ op: 'add', path: 'rev', value: 1 }],
    [{ op: 'add', path: '/rev/deeper', value: 1 }],
    [{ path: '/rev' }],
    ['nope'],
  ])('rejects %j', (op) => {
    expect(() => applyJsonPatch(doc(), [op])).toThrow(PatchError)
  })

  it('does not walk the prototype chain', () => {
    expect(() => applyJsonPatch({}, [{ op: 'add', path: '/__proto__/x/y', value: 1 }])).toThrow(PatchError)
    expect(() => applyJsonPatch({}, [{ op: 'remove', path: '/constructor' }])).toThrow(PatchError)
  })
})
