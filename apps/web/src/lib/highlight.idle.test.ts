import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const made = vi.hoisted(() => ({ workers: [] as FakeWorker[] }))
class FakeWorker {
  onmessage: ((e: MessageEvent) => void) | null = null
  onerror: (() => void) | null = null
  terminated = false
  posted: { op: string; id: number }[] = []
  constructor() {
    made.workers.push(this)
  }
  postMessage(msg: { op: string; id: number }) {
    this.posted.push(msg)
  }
  terminate() {
    this.terminated = true
  }
  reply(id: number, result: unknown) {
    this.onmessage?.({ data: { id, result } } as MessageEvent)
  }
}

beforeEach(() => {
  vi.useFakeTimers()
  vi.stubGlobal('Worker', FakeWorker)
  vi.resetModules()
  made.workers.length = 0
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('highlight worker idle', () => {
  it('terminates after 60 s without requests and a later request spawns a new worker', async () => {
    const { tokenize, WORKER_IDLE_MS } = await import('./highlight')
    const first = tokenize('a', 'ts')
    const w1 = made.workers[0]!
    w1.reply(w1.posted[0]!.id, null)
    await first
    vi.advanceTimersByTime(WORKER_IDLE_MS + 1)
    expect(w1.terminated).toBe(true)
    const second = tokenize('b', 'ts')
    expect(made.workers).toHaveLength(2)
    const w2 = made.workers[1]!
    w2.reply(w2.posted[0]!.id, { counts: [1], toks: [1, 0, 0], palette: [] })
    expect(await second).toEqual([[{ content: 'b' }]])
  })

  it('does not terminate while a request is still pending', async () => {
    const { tokenize, WORKER_IDLE_MS } = await import('./highlight')
    const p = tokenize('a', 'ts')
    const w = made.workers[0]!
    vi.advanceTimersByTime(WORKER_IDLE_MS * 2)
    expect(w.terminated).toBe(false)
    w.reply(w.posted[0]!.id, null)
    expect(await p).toBeNull()
    vi.advanceTimersByTime(WORKER_IDLE_MS + 1)
    expect(w.terminated).toBe(true)
  })
})
