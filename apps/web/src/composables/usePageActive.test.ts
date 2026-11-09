import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import { pageActive, useActiveInterval, usePageActive } from './usePageActive'

function setHidden(hidden: boolean): void {
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => (hidden ? 'hidden' : 'visible') })
  document.dispatchEvent(new Event('visibilitychange'))
}

describe('usePageActive', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    setHidden(false)
  })
  afterEach(() => {
    vi.useRealTimers()
    setHidden(false)
  })

  it('follows document visibility', () => {
    const active = usePageActive()
    expect(active.value).toBe(true)
    setHidden(true)
    expect(pageActive.value).toBe(false)
    setHidden(false)
    expect(pageActive.value).toBe(true)
  })

  it('ticks while visible, stops while hidden, catches up once on return', () => {
    const fn = vi.fn()
    const scope = effectScope()
    scope.run(() => useActiveInterval(fn, 1000))
    vi.advanceTimersByTime(3000)
    expect(fn).toHaveBeenCalledTimes(3)
    setHidden(true)
    vi.advanceTimersByTime(60_000)
    expect(fn).toHaveBeenCalledTimes(3)
    setHidden(false)
    expect(fn).toHaveBeenCalledTimes(4)
    vi.advanceTimersByTime(2000)
    expect(fn).toHaveBeenCalledTimes(6)
    scope.stop()
    vi.advanceTimersByTime(5000)
    expect(fn).toHaveBeenCalledTimes(6)
  })

  it('does not start while hidden and starts on show', () => {
    setHidden(true)
    const fn = vi.fn()
    const scope = effectScope()
    scope.run(() => useActiveInterval(fn, 1000))
    vi.advanceTimersByTime(5000)
    expect(fn).not.toHaveBeenCalled()
    setHidden(false)
    expect(fn).toHaveBeenCalledTimes(1)
    vi.advanceTimersByTime(1000)
    expect(fn).toHaveBeenCalledTimes(2)
    scope.stop()
  })

  it('skips the catch-up when asked', () => {
    const fn = vi.fn()
    const scope = effectScope()
    scope.run(() => useActiveInterval(fn, 1000, { catchUp: false }))
    setHidden(true)
    setHidden(false)
    expect(fn).not.toHaveBeenCalled()
    scope.stop()
  })
})
