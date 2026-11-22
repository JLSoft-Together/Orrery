import { describe, expect, it } from 'vitest'
import type { TermMenu } from './termMenu'
import { isPlainClick, itemAtRow, rowAt, rowBox } from './termMenuHit'

const box = { left: 10, top: 100, width: 800, height: 480 }

const menu = {
  kind: 'ask',
  agent: 'claude',
  items: [
    { index: 1, label: 'Red', row: 8, selected: true },
    { index: 2, label: 'Green', row: 9, selected: false },
  ],
  highlighted: 1,
  numbered: true,
  confident: true,
  firstRow: 8,
  lastRow: 9,
} as TermMenu

describe('rowAt', () => {
  it('maps a y to the viewport row', () => {
    expect(rowAt(box, 24, 50, 100)).toBe(0)
    expect(rowAt(box, 24, 50, 119.9)).toBe(0)
    expect(rowAt(box, 24, 50, 120)).toBe(1)
    expect(rowAt(box, 24, 50, 100 + 8 * 20 + 5)).toBe(8)
    expect(rowAt(box, 24, 50, 579.9)).toBe(23)
  })

  it('is null outside the screen', () => {
    expect(rowAt(box, 24, 9, 200)).toBeNull()
    expect(rowAt(box, 24, 810, 200)).toBeNull()
    expect(rowAt(box, 24, 50, 99)).toBeNull()
    expect(rowAt(box, 24, 50, 580)).toBeNull()
  })

  it('is null for an empty screen box or zero rows', () => {
    expect(rowAt({ ...box, height: 0 }, 24, 50, 100)).toBeNull()
    expect(rowAt(box, 0, 50, 100)).toBeNull()
  })

  it('follows a resized screen', () => {
    expect(rowAt({ ...box, height: 240 }, 24, 50, 100 + 8 * 10 + 1)).toBe(8)
  })
})

describe('itemAtRow', () => {
  it('finds the item on a row', () => {
    expect(itemAtRow(menu, 9)?.label).toBe('Green')
  })

  it('is null for a row outside the menu, no menu or no row', () => {
    expect(itemAtRow(menu, 7)).toBeNull()
    expect(itemAtRow(menu, 10)).toBeNull()
    expect(itemAtRow(null, 8)).toBeNull()
    expect(itemAtRow(menu, null)).toBeNull()
  })
})

describe('rowBox', () => {
  it('returns the band of one row', () => {
    expect(rowBox(box, 24, 8)).toEqual({ left: 10, top: 260, width: 800, height: 20 })
  })
})

describe('isPlainClick', () => {
  const base = { button: 0, shiftKey: false, altKey: false, ctrlKey: false, metaKey: false }

  it('accepts a plain left click', () => {
    expect(isPlainClick(base)).toBe(true)
  })

  it('leaves Shift, Alt, Ctrl, Meta and other buttons to the terminal', () => {
    expect(isPlainClick({ ...base, shiftKey: true })).toBe(false)
    expect(isPlainClick({ ...base, altKey: true })).toBe(false)
    expect(isPlainClick({ ...base, ctrlKey: true })).toBe(false)
    expect(isPlainClick({ ...base, metaKey: true })).toBe(false)
    expect(isPlainClick({ ...base, button: 2 })).toBe(false)
  })
})
