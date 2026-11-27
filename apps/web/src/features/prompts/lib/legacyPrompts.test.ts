import { beforeEach, describe, expect, it } from 'vitest'
import { LEGACY_KEY, legacyRows, markMigrated, MIGRATED_KEY, readLegacy } from './legacyPrompts'

describe('legacy prompts', () => {
  beforeEach(() => localStorage.clear())

  it('maps the old shape and falls back to all projects for a scope it cannot keep', () => {
    const raw = JSON.stringify([
      { id: '1', name: 'A', text: 'one\r\ntwo', scope: 'global' },
      { id: '2', name: 'B', text: 'b', scope: 'd:\\work\\app' },
      { id: '3', name: 'C', text: 'c', scope: '\\users\\me\\app' },
      { id: '4', name: '', text: 'no title', scope: 'global' },
      'junk',
    ])
    expect(legacyRows(raw)).toEqual([
      { title: 'A', body: 'one\ntwo', scope: 'all' },
      { title: 'B', body: 'b', scope: 'd:\\work\\app' },
      { title: 'C', body: 'c', scope: 'all' },
    ])
  })
  it('survives missing or broken storage', () => {
    expect(legacyRows(null)).toEqual([])
    expect(legacyRows('{nope')).toEqual([])
    expect(legacyRows('{"a":1}')).toEqual([])
  })
  it('reads once: nothing after the migrated flag', () => {
    localStorage.setItem(LEGACY_KEY, JSON.stringify([{ id: '1', name: 'A', text: 'a', scope: 'global' }]))
    expect(readLegacy()).toHaveLength(1)
    markMigrated()
    expect(localStorage.getItem(MIGRATED_KEY)).toBe('1')
    expect(readLegacy()).toEqual([])
  })
})
