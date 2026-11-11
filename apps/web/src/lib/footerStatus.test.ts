import { describe, expect, it } from 'vitest'
import { changedCount, contextPair, devPorts, footerCounts, portsLabel, sessionCounts } from './footerStatus'

const srv = (over: Record<string, unknown>) => ({ kind: 'dev-server' as const, cwd: undefined, sessionId: null, ports: [{ port: 5173, address: '', established: 0 }], ...over })

describe('footerCounts', () => {
  it('counts running and waiting and remembers the first of each', () => {
    const c = footerCounts([
      { key: 'a', state: 'idle' },
      { key: 'b', state: 'working' },
      { key: 'c', state: 'asked' },
      { key: 'd', state: 'subagents' },
      { key: 'e', state: 'asked' },
      { key: 'f', state: null },
      { key: 'g', state: 'failed' },
    ])
    expect(c).toEqual({ running: 2, waiting: 2, firstRunning: 'b', firstWaiting: 'c' })
  })
  it('is empty without panes', () => {
    expect(footerCounts([])).toEqual({ running: 0, waiting: 0, firstRunning: null, firstWaiting: null })
  })
})

describe('devPorts', () => {
  it('matches by session or by folder, sorted and unique', () => {
    const ports = devPorts(
      [
        srv({ sessionId: 's1', ports: [{ port: 5173, address: '', established: 0 }] }),
        srv({ cwd: 'C:\\work\\App\\', ports: [{ port: 4317, address: '', established: 0 }, { port: 5173, address: '', established: 0 }] }),
        srv({ sessionId: 'other', cwd: 'D:/x' }),
        srv({ kind: 'daemon', sessionId: 's1' }),
      ],
      new Set(['s1']),
      ['c:/work/app'],
    )
    expect(ports).toEqual([4317, 5173])
  })
  it('returns nothing when no server is ours', () => {
    expect(devPorts([srv({ sessionId: 'x' })], new Set(), [])).toEqual([])
  })
})

describe('portsLabel', () => {
  it('joins and truncates', () => {
    expect(portsLabel([5173, 4317])).toBe('5173 · 4317')
    expect(portsLabel([1, 2, 3, 4, 5, 6])).toBe('1 · 2 · 3 · 4 +2')
  })
})

describe('changedCount', () => {
  it('is null without data and sums tracked and untracked', () => {
    expect(changedCount(undefined)).toBeNull()
    expect(changedCount({ files: 5, untracked: 2 })).toBe(7)
    expect(changedCount({ files: 0, untracked: 0 })).toBe(0)
  })
})

describe('contextPair', () => {
  const fmt = (n: number): string => `${Math.round(n / 1000)}K`
  it('never invents numbers', () => {
    expect(contextPair(undefined, 500_000, fmt)).toBeNull()
    expect(contextPair(0, 500_000, fmt)).toBeNull()
    expect(contextPair(NaN, 500_000, fmt)).toBeNull()
  })
  it('pairs tokens with a limit only when there is one', () => {
    expect(contextPair(412_000, 500_000, fmt)).toEqual({ used: '412K', limit: '500K' })
    expect(contextPair(412_000, 0, fmt)).toEqual({ used: '412K', limit: null })
  })

  it('counts monitor sessions by group, first of each', () => {
    const c = sessionCounts([
      { id: 'a', status: 'idle' },
      { id: 'b', status: 'working' },
      { id: 'c', status: 'waiting' },
      { id: 'd', status: 'working' },
      { id: 'e', status: 'stale' },
    ])
    expect(c).toEqual({ running: 2, waiting: 1, firstRunning: 'b', firstWaiting: 'c' })
  })
})
