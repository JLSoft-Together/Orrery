import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const made = vi.hoisted(() => ({ ctxs: [] as { close: ReturnType<typeof vi.fn>; state: string }[] }))
function node() {
  return {
    connect: (n: unknown) => n,
    type: '',
    frequency: { value: 0 },
    gain: { setValueAtTime() {}, linearRampToValueAtTime() {}, exponentialRampToValueAtTime() {} },
    start() {},
    stop() {},
  }
}
class FakeAudioContext {
  state = 'running'
  currentTime = 0
  destination = {}
  close = vi.fn(() => Promise.resolve())
  constructor() {
    made.ctxs.push(this)
  }
  resume() {
    return Promise.resolve()
  }
  createOscillator = node
  createGain = node
}

beforeEach(() => {
  vi.useFakeTimers()
  vi.stubGlobal('AudioContext', FakeAudioContext)
  vi.resetModules()
  made.ctxs.length = 0
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('playChime idle close', () => {
  it('closes the context after 30 s of silence and the next chime opens a new one', async () => {
    const { playChime, CHIME_IDLE_CLOSE_MS } = await import('./sound')
    playChime()
    playChime('alert')
    expect(made.ctxs).toHaveLength(1)
    vi.advanceTimersByTime(CHIME_IDLE_CLOSE_MS - 1)
    expect(made.ctxs[0]!.close).not.toHaveBeenCalled()
    vi.advanceTimersByTime(2)
    expect(made.ctxs[0]!.close).toHaveBeenCalledTimes(1)
    expect(() => playChime()).not.toThrow()
    expect(made.ctxs).toHaveLength(2)
  })
})
