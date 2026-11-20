import { describe, expect, it } from 'vitest'
import type { RailRow } from '../rail/railModel'
import type { LiveRow } from './railLive'
import { footModeFor, isToolRow, parseToolsOpen, splitTools, toggleOpen, toolDetail } from './railTools'

const row = (key: string, kind: RailRow['kind'], pane: Record<string, unknown>): LiveRow => ({ key, src: { kind, pane } as unknown as RailRow }) as LiveRow

describe('tool rows', () => {
  const rows = new Map<string, LiveRow>([
    ['a', row('a', 'agent', { kind: 'term' })],
    ['f', row('f', 'other', { kind: 'file', dirty: true })],
    ['g', row('g', 'other', { kind: 'file' })],
    ['c', row('c', 'other', { kind: 'changes', count: 3 })],
    ['r', row('r', 'other', { kind: 'run' })],
    ['t', row('t', 'other', {})],
  ])
  it('classifies', () => {
    expect(['a', 'f', 'g', 'c', 'r', 't'].map((k) => isToolRow(rows.get(k)!))).toEqual([false, true, true, true, false, false])
  })
  it('splits and counts dirty', () => {
    const s = splitTools(['a', 'f', 'g', 'c', 'r'], (k) => rows.get(k))
    expect(s.main).toEqual(['a', 'r'])
    expect(s.tools).toEqual(['f', 'g', 'c'])
    expect(s.dirty).toBe(1)
  })
  it('details', () => {
    expect(toolDetail(rows.get('f')!)).toEqual({ kind: 'dirty' })
    expect(toolDetail(rows.get('c')!)).toEqual({ kind: 'files', n: 3 })
    expect(toolDetail(rows.get('g')!)).toBeNull()
  })
  it('open set', () => {
    expect([...toggleOpen(new Set(), 'p')]).toEqual(['p'])
    expect([...toggleOpen(new Set(['p']), 'p')]).toEqual([])
    expect([...parseToolsOpen('["a",1]')]).toEqual(['a'])
    expect([...parseToolsOpen('{bad')]).toEqual([])
  })
})

describe('footModeFor', () => {
  it('picks by width', () => {
    expect(footModeFor(343, false)).toBe('beside')
    expect(footModeFor(247, false)).toBe('under')
    expect(footModeFor(150, false)).toBe('icon')
    expect(footModeFor(300, true)).toBe('icon')
    expect(footModeFor(0, false)).toBe('under')
  })
})
