import { describe, expect, it } from 'vitest'
import { ratioLabel, snappedStop, snapRatio } from './layoutSnap'

describe('snapRatio', () => {
  it('snaps to 1/3, 1/2, 2/3 within 12px', () => {
    expect(snapRatio(0.51, 1000)).toBe(0.5)
    expect(snapRatio(0.345, 1000)).toBe(1 / 3)
    expect(snapRatio(0.66, 1000)).toBe(2 / 3)
  })

  it('leaves a ratio further than 12px alone', () => {
    expect(snapRatio(0.52, 1000)).toBe(0.52)
    expect(snapRatio(0.4, 1000)).toBe(0.4)
  })

  it('scales the threshold with the span', () => {
    expect(snapRatio(0.52, 500)).toBe(0.5)
    expect(snapRatio(0.52, 0)).toBe(0.52)
  })

  it('reports the stop it landed on', () => {
    expect(snappedStop(0.5)).toBe(0.5)
    expect(snappedStop(0.51)).toBeNull()
  })
})

describe('ratioLabel', () => {
  it('prints both sides in percent', () => {
    expect(ratioLabel(0.62)).toBe('62 / 38')
    expect(ratioLabel(1 / 3)).toBe('33 / 67')
    expect(ratioLabel(0.5)).toBe('50 / 50')
  })
})
