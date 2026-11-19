import { describe, expect, it } from 'vitest'
import type { IpcMainInvokeEvent } from 'electron'
import { createDiskRam } from '../src/diskRam/ipc'
import { lockOf, parseRamTargets, RamSampler, selfTree, tagOf, type RawProc } from '../src/diskRam/sampler'

const row = (pid: number, ppid: number, name: string, extra: Partial<RawProc> = {}): RawProc => ({ pid, ppid, name, start: `${pid}000`, startMs: pid, ramBytes: pid * 1024 * 1024, cpuMs: 0, ...extra })

const TABLE: RawProc[] = [
  row(4, 0, 'System'),
  row(600, 4, 'csrss.exe'),
  row(900, 600, 'explorer.exe'),
  row(1000, 900, 'electron.exe'),
  row(1010, 1000, 'electron.exe'),
  row(1020, 1000, 'orrery-core.exe'),
  row(1030, 1020, 'node.exe'),
  row(2000, 900, 'chrome.exe'),
  row(2100, 900, 'claude.exe'),
  row(2200, 900, 'java.exe'),
  row(2300, 900, 'svchost.exe'),
  row(2400, 900, 'protected.exe', { start: null }),
]

describe('RAM tags and locks', () => {
  it('tags agent and dev tools', () => {
    expect(tagOf('claude.exe')).toBe('agent')
    expect(tagOf('codex')).toBe('agent')
    expect(tagOf('java.exe')).toBe('dev')
    expect(tagOf('qemu-system-x86_64.exe')).toBe('dev')
    expect(tagOf('chrome.exe')).toBeNull()
  })

  it('locks Orrery, its children, its parents and system processes', () => {
    const tree = selfTree(TABLE, 1000)
    expect([...tree.self].sort()).toEqual([1000, 1010, 1020, 1030])
    expect([...tree.parents].sort()).toEqual([4, 600, 900])
    const lock = (pid: number) => lockOf(TABLE.find((r) => r.pid === pid)!, tree, 'win32')
    expect(lock(1030)).toBe('self')
    expect(lock(900)).toBe('parent')
    expect(lock(2300)).toBe('system')
    expect(lock(2000)).toBeNull()
  })

  it('survives a ppid cycle', () => {
    const tree = selfTree([row(10, 20, 'a'), row(20, 10, 'b')], 10)
    expect(tree.self.has(20)).toBe(true)
  })

  it('validates kill targets', () => {
    expect(parseRamTargets({ targets: [{ pid: 5, start: 'x' }] })).toEqual([{ pid: 5, start: 'x' }])
    expect(parseRamTargets({ targets: [] })).toBeNull()
    expect(parseRamTargets({ targets: [{ pid: -1, start: 'x' }] })).toBeNull()
    expect(parseRamTargets({ targets: [{ pid: 5 }] })).toBeNull()
    expect(parseRamTargets({ targets: Array.from({ length: 65 }, (_, i) => ({ pid: i + 10, start: 'x' })) })).toBeNull()
  })
})

describe('RAM sampler', () => {
  it('computes CPU % from deltas and sorts by RAM', async () => {
    let now = 10_000
    let cpu = 0
    const s = new RamSampler({
      read: async () => [row(2000, 900, 'chrome.exe', { cpuMs: cpu }), row(2200, 900, 'java.exe', { cpuMs: 0 })],
      selfPid: 1000,
      platform: 'win32',
      cpus: () => 4,
      totalMem: () => 16e9,
      freeMem: () => 8e9,
      now: () => now,
    })
    const a = await s.snapshot()
    expect(a.procs.map((p) => p.pid)).toEqual([2200, 2000])
    expect(a.procs[0]!.cpu).toBeNull()
    now += 2000
    cpu = 4000
    const b = await s.snapshot()
    expect(b.procs.find((p) => p.pid === 2000)!.cpu).toBe(50)
    expect(b.procs.find((p) => p.pid === 2200)!.tag).toBe('dev')
  })

  it('marks rows without a start time as system', async () => {
    const s = new RamSampler({ read: async () => TABLE, selfPid: 1000, platform: 'win32', cpus: () => 1, totalMem: () => 1, freeMem: () => 1 })
    const snap = await s.snapshot()
    expect(snap.procs.find((p) => p.pid === 2400)!.lock).toBe('system')
  })
})

describe('RAM kill', () => {
  const ev = {} as IpcMainInvokeEvent
  const setup = (live: () => Map<number, string>, confirm = true) => {
    const killed: number[] = []
    const asked: string[] = []
    const ram = createDiskRam({
      ipcMain: { handle: () => undefined, removeHandler: () => undefined },
      isAppFrame: () => true,
      read: async () => TABLE,
      selfPid: 1000,
      platform: 'win32',
      cpus: () => 1,
      totalMem: () => 1,
      freeMem: () => 1,
      liveStarts: async () => live(),
      killTree: async (pid) => {
        killed.push(pid)
        return true
      },
      confirm: async (_e, o) => {
        asked.push(String(o.detail))
        return confirm
      },
      autoConfirm: false,
      locale: () => 'vi',
    })
    return { ram, killed, asked }
  }
  const all = () => new Map(TABLE.filter((r) => r.start).map((r) => [r.pid, r.start!]))

  it('refuses Orrery, its parents and system processes', async () => {
    const { ram, killed } = setup(all)
    await ram.sampler.snapshot()
    expect(await ram.kill(ev, { targets: [{ pid: 1000, start: '1000000' }] })).toMatchObject({ ok: false, error: 'locked', lock: 'self' })
    expect(await ram.kill(ev, { targets: [{ pid: 1030, start: '1030000' }] })).toMatchObject({ ok: false, error: 'locked', lock: 'self' })
    expect(await ram.kill(ev, { targets: [{ pid: 900, start: '900000' }] })).toMatchObject({ ok: false, error: 'locked', lock: 'parent' })
    expect(await ram.kill(ev, { targets: [{ pid: 2300, start: '2300000' }] })).toMatchObject({ ok: false, error: 'locked', lock: 'system' })
    expect(killed).toEqual([])
  })

  it('refuses pids that were not listed, reused pids and processes that vanished', async () => {
    let live = all()
    const { ram, killed } = setup(() => live)
    expect(await ram.kill(ev, { targets: [{ pid: 2000, start: '2000000' }] })).toMatchObject({ error: 'gone' })
    await ram.sampler.snapshot()
    expect(await ram.kill(ev, { targets: [{ pid: 2000, start: '999' }] })).toMatchObject({ error: 'gone' })
    live = new Map([[2000, '123']])
    expect(await ram.kill(ev, { targets: [{ pid: 2000, start: '2000000' }] })).toMatchObject({ error: 'gone' })
    live = new Map()
    expect(await ram.kill(ev, { targets: [{ pid: 2000, start: '2000000' }] })).toMatchObject({ error: 'gone' })
    expect(killed).toEqual([])
  })

  it('kills after confirm and re-checks the start time after the dialog', async () => {
    let calls = 0
    const { ram, killed, asked } = setup(() => {
      calls++
      return calls === 1 ? all() : new Map([[2000, '2000000']])
    })
    await ram.sampler.snapshot()
    const res = await ram.kill(ev, { targets: [{ pid: 2000, start: '2000000' }, { pid: 2200, start: '2200000' }] })
    expect(res).toEqual({ ok: true, killed: 1 })
    expect(killed).toEqual([2000])
    expect(asked[0]).toContain('chrome.exe')
  })

  it('does nothing when the dialog is cancelled', async () => {
    const { ram, killed } = setup(all, false)
    await ram.sampler.snapshot()
    expect(await ram.kill(ev, { targets: [{ pid: 2000, start: '2000000' }] })).toMatchObject({ error: 'cancelled' })
    expect(killed).toEqual([])
  })
})
