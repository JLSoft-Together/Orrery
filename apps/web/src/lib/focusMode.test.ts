import { describe, expect, it } from 'vitest'
import { parseFocus, serializeFocus, setFocus } from './focusMode'

describe('parseFocus', () => {
  it('reads a list of tab keys', () => {
    expect([...parseFocus('["t1","t2"]')]).toEqual(['t1', 't2'])
  })
  it('survives garbage and missing values', () => {
    expect(parseFocus(null).size).toBe(0)
    expect(parseFocus('{').size).toBe(0)
    expect(parseFocus('{"a":1}').size).toBe(0)
    expect([...parseFocus('[1,"t1",null]')]).toEqual(['t1'])
  })
})

describe('setFocus', () => {
  it('turns a tab on and off', () => {
    const on = setFocus(new Set(), 't1', true, ['t1', 't2'])
    expect([...on]).toEqual(['t1'])
    expect(setFocus(on, 't1', false, ['t1', 't2']).size).toBe(0)
  })
  it('drops keys of closed tabs', () => {
    expect([...setFocus(new Set(['gone', 't2']), 't1', true, ['t1', 't2'])]).toEqual(['t2', 't1'])
  })
  it('ignores an unknown tab', () => {
    expect(setFocus(new Set(), 'x', true, ['t1']).size).toBe(0)
  })
})

describe('serializeFocus', () => {
  it('round-trips', () => {
    expect([...parseFocus(serializeFocus(new Set(['t1', 't3'])))]).toEqual(['t1', 't3'])
  })
})
