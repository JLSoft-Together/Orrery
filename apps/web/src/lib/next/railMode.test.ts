import { describe, expect, it } from 'vitest'
import { clampRailWidth, defaultRailWidth, nextRailMode, parseRailWidth, railModeFor, railWidth } from './railMode'

describe('railModeFor', () => {
  it('maps the breakpoints', () => {
    expect(railModeFor(1600)).toBe('full')
    expect(railModeFor(1440)).toBe('full')
    expect(railModeFor(1200)).toBe('full')
    expect(railModeFor(1199)).toBe('icon')
    expect(railModeFor(1024)).toBe('icon')
    expect(railModeFor(1023)).toBe('overlay')
    expect(railModeFor(1000)).toBe('overlay')
  })
})

describe('nextRailMode hysteresis', () => {
  it('shrinks 24px below the breakpoint', () => {
    expect(nextRailMode('full', 1199)).toBe('full')
    expect(nextRailMode('full', 1176)).toBe('full')
    expect(nextRailMode('full', 1175)).toBe('icon')
    expect(nextRailMode('icon', 1023)).toBe('icon')
    expect(nextRailMode('icon', 1000)).toBe('icon')
    expect(nextRailMode('icon', 999)).toBe('overlay')
  })
  it('does not flip while wobbling around 1200 and 1024', () => {
    let mode = nextRailMode('full', 1190)
    for (const w of [1210, 1180, 1199, 1201, 1177, 1223]) mode = nextRailMode(mode, w)
    expect(mode).toBe('full')
    mode = nextRailMode('icon', 1030)
    for (const w of [1010, 1040, 1001, 1047, 1020]) mode = nextRailMode(mode, w)
    expect(mode).toBe('icon')
  })
  it('needs 24px more to grow back', () => {
    expect(nextRailMode('icon', 1200)).toBe('icon')
    expect(nextRailMode('icon', 1223)).toBe('icon')
    expect(nextRailMode('icon', 1224)).toBe('full')
    expect(nextRailMode('overlay', 1024)).toBe('overlay')
    expect(nextRailMode('overlay', 1047)).toBe('overlay')
    expect(nextRailMode('overlay', 1048)).toBe('icon')
  })
  it('does not flip while wobbling around a breakpoint', () => {
    let mode = nextRailMode('icon', 1190)
    for (const w of [1195, 1205, 1199, 1210, 1220, 1190]) mode = nextRailMode(mode, w)
    expect(mode).toBe('icon')
  })
  it('jumps two steps on a big resize', () => {
    expect(nextRailMode('overlay', 1600)).toBe('full')
    expect(nextRailMode('full', 800)).toBe('overlay')
  })
  it('keeps the mode on an unusable width', () => {
    expect(nextRailMode('icon', 0)).toBe('icon')
    expect(nextRailMode('full', Number.NaN)).toBe('full')
  })
})

describe('railWidth', () => {
  it('is 264 from 1440 and 248 below', () => {
    expect(railWidth('full', 1440, null)).toBe(264)
    expect(railWidth('full', 1600, null)).toBe(264)
    expect(railWidth('full', 1300, null)).toBe(248)
    expect(defaultRailWidth(1200)).toBe(248)
  })
  it('collapses to 56 and overlays at 248', () => {
    expect(railWidth('icon', 1100, 300)).toBe(56)
    expect(railWidth('overlay', 900, 300)).toBe(248)
  })
  it('uses the dragged width clamped to 224-360', () => {
    expect(railWidth('full', 1600, 300)).toBe(300)
    expect(railWidth('full', 1600, 100)).toBe(224)
    expect(railWidth('full', 1600, 900)).toBe(360)
    expect(clampRailWidth(Number.NaN)).toBe(224)
  })
  it('parses stored values', () => {
    expect(parseRailWidth('300')).toBe(300)
    expect(parseRailWidth('50')).toBe(224)
    expect(parseRailWidth('abc')).toBeNull()
    expect(parseRailWidth(null)).toBeNull()
    expect(parseRailWidth('0')).toBeNull()
  })
})
