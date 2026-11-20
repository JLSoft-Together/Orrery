import { describe, expect, it } from 'vitest'
import { elapsedParts, rowStatusOf, rowToolLine } from './rowText'

describe('rowText', () => {
  it('maps rail states to 4 statuses', () => {
    expect(rowStatusOf('failed')).toBe('error')
    expect(rowStatusOf('asked')).toBe('waiting')
    expect(rowStatusOf('working')).toBe('working')
    expect(rowStatusOf('subagents')).toBe('working')
    expect(rowStatusOf('starting')).toBe('working')
    expect(rowStatusOf('done')).toBe('idle')
    expect(rowStatusOf('parked')).toBe('idle')
  })

  it('splits elapsed time into one unit', () => {
    expect(elapsedParts(0)).toBeNull()
    expect(elapsedParts(38_000)).toEqual({ unit: 's', n: 38 })
    expect(elapsedParts(4 * 60_000 + 20_000)).toEqual({ unit: 'm', n: 4 })
    expect(elapsedParts(3 * 3600_000)).toEqual({ unit: 'h', n: 3 })
    expect(elapsedParts(50 * 3600_000)).toEqual({ unit: 'd', n: 2 })
  })

  it('shows the tool only while working', () => {
    expect(rowToolLine({ state: 'working', tool: 'Edit src/app.ts', waitText: null, stateText: 'x' })).toBe('Edit src/app.ts')
    expect(rowToolLine({ state: 'asked', tool: null, waitText: 'Cần duyệt', stateText: null })).toBe('Cần duyệt')
    expect(rowToolLine({ state: 'idle', tool: 'Edit', waitText: null, stateText: 'Rảnh' })).toBeNull()
  })
})
