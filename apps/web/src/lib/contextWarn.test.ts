import { describe, expect, it } from 'vitest'
import { CONTEXT_WARN_DEFAULT, clampContextWarn, contextCrossings, contextOverWarn, parseWarned, serializeWarned } from './contextWarn'

describe('contextWarn', () => {
  it('clamps the setting to 0..2M and falls back on garbage', () => {
    expect(clampContextWarn(-5)).toBe(0)
    expect(clampContextWarn(9_000_000)).toBe(2_000_000)
    expect(clampContextWarn(350_000.4)).toBe(350_000)
    expect(clampContextWarn(Number.NaN)).toBe(CONTEXT_WARN_DEFAULT)
  })

  it('is over only at or above a positive threshold with known tokens', () => {
    expect(contextOverWarn(500_000, 500_000)).toBe(true)
    expect(contextOverWarn(499_999, 500_000)).toBe(false)
    expect(contextOverWarn(900_000, 0)).toBe(false)
    expect(contextOverWarn(undefined, 500_000)).toBe(false)
  })

  it('fires once per crossing and re-arms after dropping below', () => {
    let armed: Set<string> = new Set()
    let r = contextCrossings(armed, [{ id: 'a', tokens: 600_000 }, { id: 'b', tokens: 10 }], 500_000)
    expect(r.fire).toEqual(['a'])
    armed = r.armed
    r = contextCrossings(armed, [{ id: 'a', tokens: 650_000 }], 500_000)
    expect(r.fire).toEqual([])
    armed = r.armed
    r = contextCrossings(armed, [{ id: 'a', tokens: 40_000 }], 500_000)
    expect(r.fire).toEqual([])
    armed = r.armed
    r = contextCrossings(armed, [{ id: 'a', tokens: 520_000 }], 500_000)
    expect(r.fire).toEqual(['a'])
  })

  it('stays silent when off or tokens are missing', () => {
    expect(contextCrossings(new Set(), [{ id: 'a', tokens: 1_000_000 }], 0).fire).toEqual([])
    expect(contextCrossings(new Set(), [{ id: 'a', tokens: undefined }], 500_000).fire).toEqual([])
  })
})

describe('warned sessions storage', () => {
  it('round-trips and survives bad data', () => {
    expect([...parseWarned(serializeWarned(new Set(['a', 'b'])))]).toEqual(['a', 'b'])
    expect(parseWarned(null).size).toBe(0)
    expect(parseWarned('{bad').size).toBe(0)
    expect([...parseWarned('["a",1,null,"b"]')]).toEqual(['a', 'b'])
  })

  it('keeps at most 200 ids', () => {
    const many = new Set(Array.from({ length: 250 }, (_, i) => `t${i}`))
    const back = parseWarned(serializeWarned(many))
    expect(back.size).toBe(200)
    expect(back.has('t249')).toBe(true)
  })
})
