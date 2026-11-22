import { beforeEach, describe, expect, it } from 'vitest'
import { canCollapseRail, effectiveRailMode, isRailToggleKey, parseRailCollapsed, RAIL_COLLAPSED_KEY, readRailCollapsed, writeRailCollapsed } from './railCollapse'

const key = (over: Partial<KeyboardEvent>): KeyboardEvent => ({ type: 'keydown', code: 'KeyB', ctrlKey: false, metaKey: false, shiftKey: true, altKey: false, isComposing: false, ...over }) as KeyboardEvent

describe('railCollapse', () => {
  beforeEach(() => localStorage.clear())

  it('collapses only the full rail to the icon strip', () => {
    expect(effectiveRailMode('full', true)).toBe('icon')
    expect(effectiveRailMode('full', false)).toBe('full')
    expect(effectiveRailMode('icon', true)).toBe('icon')
    expect(effectiveRailMode('overlay', true)).toBe('overlay')
    expect(canCollapseRail('full')).toBe(true)
    expect(canCollapseRail('icon')).toBe(false)
    expect(canCollapseRail('overlay')).toBe(false)
  })

  it('remembers the state', () => {
    expect(readRailCollapsed()).toBe(false)
    writeRailCollapsed(true)
    expect(localStorage.getItem(RAIL_COLLAPSED_KEY)).toBe('1')
    expect(readRailCollapsed()).toBe(true)
    writeRailCollapsed(false)
    expect(localStorage.getItem(RAIL_COLLAPSED_KEY)).toBeNull()
    expect(parseRailCollapsed('0')).toBe(false)
  })

  it('matches Ctrl+Shift+B on Windows and Cmd+Shift+B on macOS', () => {
    expect(isRailToggleKey(key({ ctrlKey: true }), false)).toBe(true)
    expect(isRailToggleKey(key({ metaKey: true }), false)).toBe(false)
    expect(isRailToggleKey(key({ metaKey: true }), true)).toBe(true)
    expect(isRailToggleKey(key({ ctrlKey: true }), true)).toBe(false)
    expect(isRailToggleKey(key({ ctrlKey: true, shiftKey: false }), false)).toBe(false)
    expect(isRailToggleKey(key({ ctrlKey: true, altKey: true }), false)).toBe(false)
    expect(isRailToggleKey(key({ ctrlKey: true, code: 'KeyE' }), false)).toBe(false)
    expect(isRailToggleKey(key({ ctrlKey: true, isComposing: true }), false)).toBe(false)
    expect(isRailToggleKey(key({ ctrlKey: true, type: 'keyup' }), false)).toBe(false)
  })
})
