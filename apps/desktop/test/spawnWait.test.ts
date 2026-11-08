import { afterEach, describe, expect, it, vi } from 'vitest'
import { createSpawnWaits } from '../src/pty/spawnWait'

describe('open_session spawn wait', () => {
  afterEach(() => vi.useRealTimers())

  it('hands over the pane id whether the spawn lands before or after the page answer', async () => {
    const waits = createSpawnWaits(10_000)
    const early = waits.expect('t1')
    waits.spawned('t1', 'pty-1')
    expect(await early.wait()).toBe('pty-1')
    const late = waits.expect('t2')
    const pending = late.wait()
    waits.spawned('t2', 'pty-2')
    expect(await pending).toBe('pty-2')
    expect(waits.size()).toBe(0)
  })

  it('ends empty after the timeout (a reused pane never spawns) and forgets the ticket', async () => {
    vi.useFakeTimers()
    const waits = createSpawnWaits(10_000)
    const pending = waits.expect('t').wait()
    await vi.advanceTimersByTimeAsync(10_000)
    expect(await pending).toBeUndefined()
    expect(waits.size()).toBe(0)
    waits.spawned('t', 'pty-late')
  })

  it('drop forgets a refused ticket; unknown tickets are ignored', () => {
    const waits = createSpawnWaits(10_000)
    waits.expect('t').drop()
    waits.spawned('t', 'pty-1')
    waits.spawned('nope', 'pty-2')
    expect(waits.size()).toBe(0)
  })
})
