import { describe, expect, it } from 'vitest'
import { clampView, deskFont, fitScale, IDENTITY, maxZoom, panView, pinchView } from './fitLayout'

describe('fit layout', () => {
  it('desk font fits the width between 9 and 13 px', () => {
    expect(deskFont(365, 48)).toBe(12)
    expect(deskFont(800, 48)).toBe(13)
    expect(deskFont(365, 180)).toBe(9)
  })

  it('scales a too wide terminal down to the width, never up', () => {
    expect(fitScale(975, 365)).toBeCloseTo(0.374, 3)
    expect(fitScale(300, 365)).toBe(1)
    expect(fitScale(0, 365)).toBe(1)
  })

})

describe('pinch zoom', () => {
  const content = { w: 975, h: 540 }
  const box = { w: 365, h: 480 }
  const base = fitScale(975, 365)

  it('starts fitted to the width and zooms up to 1.5x the natural size', () => {
    expect(clampView({ zoom: 1, x: -50, y: -20 }, base, content, box)).toEqual({ zoom: 1, x: 0, y: 0 })
    expect(maxZoom(base)).toBeCloseTo(1.5 / base, 5)
    expect(maxZoom(1)).toBe(1.5)
  })

  it('keeps the point under the fingers while pinching and clamps the pan inside the content', () => {
    const v = pinchView(IDENTITY, { dist: 100, mid: { x: 300, y: 100 } }, { dist: 250, mid: { x: 300, y: 100 } }, base, content, box)
    expect(v.zoom).toBeCloseTo(2.5, 5)
    const s = base * v.zoom
    expect((300 - v.x) / s).toBeCloseTo(300 / base, 3)
    expect(v.x).toBeLessThanOrEqual(0)
    expect(v.x).toBeGreaterThanOrEqual(box.w - content.w * s)
    const far = panView(v, { x: -5000, y: -5000 }, base, content, box)
    expect(far.x).toBeCloseTo(box.w - content.w * s, 5)
    expect(far.y).toBeCloseTo(Math.min(0, box.h - content.h * s), 5)
    const back = pinchView(v, { dist: 200, mid: { x: 100, y: 100 } }, { dist: 20, mid: { x: 100, y: 100 } }, base, content, box)
    expect(back).toEqual({ zoom: 1, x: 0, y: 0 })
  })
})
