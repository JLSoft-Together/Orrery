import { afterEach, describe, expect, it } from 'vitest'
import { onCoreFallback, reportCoreFallback, resetCoreFallback, type CoreFallbackMessage } from '../src/coreFallback'

afterEach(() => {
  onCoreFallback(null)
  resetCoreFallback()
})

describe('reportCoreFallback', () => {
  it('tells the app once per kind, with a short reason', () => {
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    reportCoreFallback('usage', 'core-transcript stopped')
    reportCoreFallback('usage', 'again')
    reportCoreFallback('transcript', 'x'.repeat(500))
    reportCoreFallback('transcript', 'again')
    expect(got.map((m) => [m.type, m.kind])).toEqual([
      ['core-fallback', 'usage'],
      ['core-fallback', 'transcript'],
    ])
    expect(got[0]!.reason).toBe('core-transcript stopped')
    expect(got[1]!.reason).toHaveLength(200)
  })
})
