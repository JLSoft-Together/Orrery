import { describe, expect, it } from 'vitest'
import { forecastFullAt, pushSample, type QuotaSample } from './forecast'

const MIN = 60_000
const T0 = new Date(2026, 10, 22, 10, 0, 0).getTime()
const series = (pairs: Array<[number, number]>): QuotaSample[] => pairs.map(([m, pct]) => ({ at: T0 + m * MIN, pct }))

describe('forecastFullAt', () => {
  it('projects the time the window reaches 100 from the recent rate', () => {
    const s = series([[0, 40], [20, 50]])
    const now = T0 + 20 * MIN
    const reset = now + 5 * 3_600_000
    expect(forecastFullAt(s, now, reset)).toBe(T0 + 20 * MIN + 100 * MIN)
  })

  it('stays silent with under 15 minutes of samples', () => {
    const s = series([[0, 40], [10, 60]])
    expect(forecastFullAt(s, T0 + 10 * MIN, T0 + 5 * 3_600_000)).toBeNull()
  })

  it('stays silent when the rate is about zero', () => {
    const s = series([[0, 40], [20, 40]])
    expect(forecastFullAt(s, T0 + 20 * MIN, T0 + 5 * 3_600_000)).toBeNull()
  })

  it('stays silent when the window resets before it would fill', () => {
    const s = series([[0, 40], [20, 50]])
    const now = T0 + 20 * MIN
    expect(forecastFullAt(s, now, now + 60 * MIN)).toBeNull()
  })

  it('stays silent without a reset time or when samples are old', () => {
    const s = series([[0, 40], [20, 60]])
    expect(forecastFullAt(s, T0 + 20 * MIN, null)).toBeNull()
    expect(forecastFullAt(s, T0 + 120 * MIN, T0 + 600 * MIN)).toBeNull()
  })
})

describe('pushSample', () => {
  it('keeps one sample a minute and 2 hours at most', () => {
    let s: QuotaSample[] = []
    s = pushSample(s, T0, 10)
    s = pushSample(s, T0 + 20_000, 11)
    expect(s).toEqual([{ at: T0 + 20_000, pct: 11 }])
    s = pushSample(s, T0 + 3 * 3_600_000, 20)
    expect(s).toHaveLength(1)
  })

  it('drops history when the percentage falls after a reset', () => {
    let s = series([[0, 80], [20, 90]])
    s = pushSample(s, T0 + 30 * MIN, 2)
    expect(s).toEqual([{ at: T0 + 30 * MIN, pct: 2 }])
    expect(forecastFullAt(s, T0 + 30 * MIN, T0 + 5 * 3_600_000)).toBeNull()
  })
})
