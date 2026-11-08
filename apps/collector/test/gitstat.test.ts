import { describe, expect, it, vi } from 'vitest'
import type { GitDiffStat } from '@ccm/shared'
import { createDiffStatReader, NOT_A_REPO } from '../src/gitstat'

const stat = (n: number): GitDiffStat => ({ files: n, insertions: 2, deletions: 3, untracked: 0, at: new Date(0).toISOString() })

describe('createDiffStatReader', () => {
  it('runs git at most twice for simultaneous callers on the same cwd, never in parallel', async () => {
    let active = 0
    let peak = 0
    const run = vi.fn(async () => {
      peak = Math.max(peak, ++active)
      await Promise.resolve()
      active--
      return stat(1)
    })
    const read = createDiffStatReader(run)
    const results = await Promise.all([read('D:\\a'), read('D:\\a'), read('D:\\a')])
    expect(run).toHaveBeenCalledTimes(2)
    expect(peak).toBe(1)
    expect(results).toEqual([stat(1), stat(1), stat(1)])
    expect(await read('D:\\b')).toEqual(stat(1))
    expect(run).toHaveBeenCalledTimes(3)
  })

  it('does not cache a finished repo result', async () => {
    let n = 0
    const run = vi.fn(async () => stat(++n))
    const read = createDiffStatReader(run)
    expect((await read('/r'))?.files).toBe(1)
    expect((await read('/r'))?.files).toBe(2)
  })

  it('edits during an in-flight read cause exactly one more run that sees the new data', async () => {
    let n = 0
    const gates: Array<() => void> = []
    const run = vi.fn(async () => {
      const mine = ++n
      await new Promise<void>((r) => gates.push(r))
      return stat(mine)
    })
    const read = createDiffStatReader(run)
    const first = read('/r')
    const second = read('/r')
    const third = read('/r')
    expect(run).toHaveBeenCalledTimes(1)
    gates[0]!()
    expect((await first)?.files).toBe(1)
    await vi.waitFor(() => expect(run).toHaveBeenCalledTimes(2))
    gates[1]!()
    expect((await second)?.files).toBe(2)
    expect((await third)?.files).toBe(2)
    expect(run).toHaveBeenCalledTimes(2)
  })

  it('does not spawn again for a confirmed non-repo cwd within the ttl, and does after it', async () => {
    let t = 0
    const run = vi.fn(async (): Promise<typeof NOT_A_REPO> => NOT_A_REPO)
    const read = createDiffStatReader(run, { now: () => t, notRepoTtlMs: 300_000 })
    expect(await read('/tmp/x')).toBeNull()
    t = 299_000
    expect(await read('/tmp/x')).toBeNull()
    expect(run).toHaveBeenCalledTimes(1)
    t = 301_000
    await read('/tmp/x')
    expect(run).toHaveBeenCalledTimes(2)
  })

  it('does not cache a timeout or other git failure', async () => {
    const run = vi.fn(async () => null)
    const read = createDiffStatReader(run)
    expect(await read('/slow')).toBeNull()
    expect(await read('/slow')).toBeNull()
    expect(run).toHaveBeenCalledTimes(2)
  })
})
