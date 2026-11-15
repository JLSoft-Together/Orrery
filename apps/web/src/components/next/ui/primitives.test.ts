import { describe, expect, it } from 'vitest'
import { chipTone, gaugeLevel } from './chipTone'
import { wrapIndex } from './focus'

describe('next primitives', () => {
  it('wraps menu / segment focus both ways', () => {
    expect(wrapIndex(-1, 1, 3)).toBe(0)
    expect(wrapIndex(-1, -1, 3)).toBe(2)
    expect(wrapIndex(2, 1, 3)).toBe(0)
    expect(wrapIndex(0, -1, 3)).toBe(2)
    expect(wrapIndex(0, 1, 0)).toBe(-1)
  })

  it('maps every status to one chip tone', () => {
    expect(chipTone('working')).toBe('working')
    expect(chipTone('waiting')).toBe('waiting')
    expect(chipTone('error')).toBe('error')
    expect(chipTone('completed')).toBe('done')
    expect(chipTone('stale')).toBe('meta')
    expect(chipTone(undefined)).toBe('idle')
  })

  it('context gauge turns high at 65%', () => {
    expect(gaugeLevel(64.9)).toBe('ok')
    expect(gaugeLevel(65)).toBe('high')
  })
})
