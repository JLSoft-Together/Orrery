import { EventEmitter } from 'node:events'
import type { ChildProcess } from 'node:child_process'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { CoreProcsClient } from '../../collector/src/coreProcs'

const sync = vi.fn()
const spawnMock = vi.fn()
vi.mock('node:child_process', async (orig) => ({
  ...(await orig<typeof import('node:child_process')>()),
  execFileSync: (...a: unknown[]) => sync(...a),
  spawn: (...a: unknown[]) => spawnMock(...a),
}))

const { defaultSpawner, freshSnapshot, killTreeSparing, nativeProcRows, setTreeSnapshot, snapshotTick, sparingPlan, treePids, watchedCount } = await import('../src/run/proc')
const { setSharedCoreProcs } = await import('../../collector/src/procSource')

const rows = [
  { pid: 100, ppid: 1, cmd: 'cmd /c gradlew' },
  { pid: 101, ppid: 100, cmd: 'java GradleWrapperMain' },
  { pid: 102, ppid: 101, cmd: 'java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14' },
  { pid: 103, ppid: 102, cmd: 'java worker of daemon' },
  { pid: 104, ppid: 100, cmd: 'node tool' },
  { pid: 105, ppid: 104, cmd: 'node tool child' },
]
const spare = (cmd: string): boolean => /GradleDaemon/.test(cmd)
const child = { pid: 100 } as ChildProcess
const real = process.platform

function fakeChild(pid: number): ChildProcess & EventEmitter {
  return Object.assign(new EventEmitter(), { pid, exitCode: null as number | null }) as ChildProcess & EventEmitter
}

function fakeClient(tables: Array<Array<[number, number, string]>>, calls: { table: number; cmd: number[][] }): CoreProcsClient {
  const dead = (): never => {
    throw new Error('unused')
  }
  return {
    ping: async () => '0',
    table: async () => {
      calls.table++
      return tables[0]!.map(([pid, ppid, name]) => ({ pid, ppid, name, startMs: 1, startRaw: '133000000000000000' }))
    },
    cmdline: async (pids) => {
      calls.cmd.push([...pids])
      return pids.map((pid) => ({ pid, cmd: `c${pid}` }))
    },
    usage: async () => dead(),
    ports: async () => dead(),
    dispose: () => undefined,
  }
}

beforeEach(() => {
  Object.defineProperty(process, 'platform', { value: 'win32' })
  sync.mockReset()
  spawnMock.mockReset()
})
afterEach(() => {
  Object.defineProperty(process, 'platform', { value: real })
  setTreeSnapshot(100, null)
  setTreeSnapshot(200, null)
  setSharedCoreProcs(undefined)
  vi.useRealTimers()
})

describe('killTreeSparing sync (app quit)', () => {
  it('with a fresh snapshot spares the daemon tree, plain-kills its ancestors and /T-kills the other roots', () => {
    setTreeSnapshot(100, rows)
    const onFallback = vi.fn()
    killTreeSparing(child, /GradleDaemon/, { sync: true, onFallback })
    expect(onFallback).not.toHaveBeenCalled()
    expect(sync).toHaveBeenCalledTimes(2)
    const [single, tree] = sync.mock.calls.map((c) => c[1] as string[])
    expect(single).toEqual(['/F', '/PID', '101', '/PID', '100'])
    expect(tree).toEqual(['/T', '/F', '/PID', '104'])
    const all = [...single!, ...tree!]
    expect(all).not.toContain('102')
    expect(all).not.toContain('103')
  })

  it('without a snapshot, or with a stale one (> 6 s), falls back to taskkill /T on the root and reports it', () => {
    const onFallback = vi.fn()
    killTreeSparing(child, /GradleDaemon/, { sync: true, onFallback })
    expect(onFallback).toHaveBeenCalledTimes(1)
    expect(sync.mock.calls[0]![1]).toEqual(['/T', '/F', '/PID', '100'])
    sync.mockReset()
    setTreeSnapshot(100, rows, Date.now() - 7_000)
    expect(freshSnapshot(100)).toBeNull()
    killTreeSparing(child, /GradleDaemon/, { sync: true })
    expect(sync.mock.calls[0]![1]).toEqual(['/T', '/F', '/PID', '100'])
  })
})

describe('snapshot rows', () => {
  it('sparingPlan puts a tree root only once and never lists the daemon tree', () => {
    const plan = sparingPlan(100, rows, spare)
    expect(plan.single).toEqual([101, 100])
    expect(plan.tree).toEqual([104])
  })

  it('keeps startMs from the table so a reused parent pid is not walked', () => {
    const FT = (ms: number): bigint => (BigInt(ms) + 11_644_473_600_000n) * 10_000n
    const table = [
      { pid: 100, ppid: 1, name: 'a', start: FT(5_000) },
      { pid: 101, ppid: 100, name: 'b', start: FT(6_000) },
      { pid: 102, ppid: 100, name: 'c', start: FT(1_000) },
    ]
    const out = nativeProcRows(100, table, () => 'x')!
    expect(out.map((r) => r.startMs)).toEqual([5_000, 6_000, 1_000])
    expect(treePids(100, out, () => false)).toEqual([101, 100])
  })
})

describe('shared snapshot ticker', () => {
  it('reads table() and the union of subtree command lines once per tick for all children', async () => {
    const calls = { table: 0, cmd: [] as number[][] }
    setSharedCoreProcs(fakeClient([[[100, 1, 'a'], [101, 100, 'b'], [200, 1, 'c'], [201, 200, 'd'], [300, 1, 'other']]], calls))
    const a = fakeChild(100)
    const b = fakeChild(200)
    spawnMock.mockReturnValueOnce(a).mockReturnValueOnce(b)
    const spec = { file: 'x', args: [], cwd: '.' }
    defaultSpawner(spec)
    defaultSpawner(spec)
    await vi.waitFor(() => expect(freshSnapshot(100)).not.toBeNull())
    const tables = calls.table
    const cmds = calls.cmd.length
    await snapshotTick()
    expect(calls.table).toBe(tables + 1)
    expect(calls.cmd).toHaveLength(cmds + 1)
    expect([...calls.cmd.at(-1)!].sort()).toEqual([100, 101, 200, 201])
    expect(freshSnapshot(100)?.find((r) => r.pid === 101)?.cmd).toBe('c101')
    expect(freshSnapshot(200)).not.toBeNull()
    expect(watchedCount()).toBe(2)
    a.exitCode = 0
    a.emit('exit')
    b.exitCode = 0
    b.emit('exit')
    expect(watchedCount()).toBe(0)
  })

  it('an exited child does not delete the snapshot of a new child with the same pid', async () => {
    const calls = { table: 0, cmd: [] as number[][] }
    setSharedCoreProcs(fakeClient([[[100, 1, 'a']]], calls))
    const first = fakeChild(100)
    spawnMock.mockReturnValueOnce(first)
    defaultSpawner({ file: 'x', args: [], cwd: '.' })
    await vi.waitFor(() => expect(freshSnapshot(100)).not.toBeNull())
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    first.exitCode = 0
    first.emit('exit')
    const fresh = [{ pid: 100, ppid: 1, cmd: 'new child' }]
    setTreeSnapshot(100, fresh)
    vi.advanceTimersByTime(7_000)
    expect(freshSnapshot(100)).toBe(fresh)
  })
})
