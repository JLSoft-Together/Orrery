import { describe, expect, it } from 'vitest'
import { canvasWithoutDrawer, effectivePinned, pinVerdict } from './explorerPin'

describe('explorerPin', () => {
  it('refuses below 1440 viewport', () => {
    expect(pinVerdict({ viewport: 1439, canvas: 1200, pinned: false })).toEqual({ ok: false, reason: 'viewport' })
  })
  it('refuses when canvas would drop below 720', () => {
    expect(pinVerdict({ viewport: 1440, canvas: 1008, pinned: false })).toEqual({ ok: true })
    expect(pinVerdict({ viewport: 1440, canvas: 1007, pinned: false })).toEqual({ ok: false, reason: 'canvas' })
  })
  it('counts the pinned column back into the canvas', () => {
    expect(canvasWithoutDrawer({ viewport: 1600, canvas: 800, pinned: true })).toBe(1088)
    expect(pinVerdict({ viewport: 1600, canvas: 720, pinned: true })).toEqual({ ok: true })
    expect(pinVerdict({ viewport: 1600, canvas: 719, pinned: true })).toEqual({ ok: false, reason: 'canvas' })
  })
  it('effectivePinned needs the flag and room', () => {
    expect(effectivePinned({ viewport: 1600, canvas: 1200, pinned: false })).toBe(false)
    expect(effectivePinned({ viewport: 1600, canvas: 1300, pinned: true })).toBe(true)
    expect(effectivePinned({ viewport: 1200, canvas: 1000, pinned: true })).toBe(false)
  })
})
