import { describe, expect, it } from 'vitest'
import { clampExplorerWidth, dragWidth, EXPLORER_DRAWER_W, fitPinnedWidth, parseExplorerWidth, pinnedRoom } from './explorerPin'

describe('explorerPin width', () => {
  it('clamps to min and max', () => {
    expect(clampExplorerWidth(100)).toBe(220)
    expect(clampExplorerWidth(900)).toBe(640)
    expect(clampExplorerWidth(333.4)).toBe(333)
  })
  it('honours a smaller room but never goes under the minimum', () => {
    expect(clampExplorerWidth(600, 400)).toBe(400)
    expect(clampExplorerWidth(600, 100)).toBe(220)
  })
  it('parses stored values', () => {
    expect(parseExplorerWidth(null)).toBe(EXPLORER_DRAWER_W)
    expect(parseExplorerWidth('abc')).toBe(EXPLORER_DRAWER_W)
    expect(parseExplorerWidth('9999')).toBe(640)
    expect(parseExplorerWidth('300')).toBe(300)
  })
  it('dragging left widens, right narrows', () => {
    expect(dragWidth(288, 1000, 940)).toBe(348)
    expect(dragWidth(288, 1000, 1100)).toBe(220)
  })
  it('keeps 560px of canvas for the pinned column', () => {
    expect(pinnedRoom(1200)).toBe(632)
    expect(dragWidth(288, 1000, 0, pinnedRoom(1200))).toBe(632)
  })
  it('fits the pinned column into the canvas', () => {
    expect(fitPinnedWidth(1050, 288)).toBe(288)
    expect(fitPinnedWidth(1050, 640)).toBe(482)
    expect(fitPinnedWidth(840, 400)).toBe(272)
    expect(fitPinnedWidth(788, 288)).toBe(220)
  })
  it('refuses to pin when even the minimum column does not fit', () => {
    expect(fitPinnedWidth(787, 288)).toBeNull()
    expect(fitPinnedWidth(0, 288)).toBeNull()
  })
})
