import { afterEach, describe, expect, it, vi } from 'vitest'
import type { CoreProcsClient, CoreProcRow } from '../src/coreProcs'
import { queryProcessesWindows } from '../src/process'
import { commandLinesWin, coreProcsMode, setSharedCoreProcs, sharedCoreProcs, tableWin, toWinProcs, usageWin } from '../src/procSource'
import { coreFirstPortTable } from '../src/resources/ports'
import { coreReader, type ProcReader, type ProcRow } from '../src/resources/procs'

function fakeClient(over: Partial<CoreProcsClient> = {}): CoreProcsClient {
  return {
    ping: async () => '0',
    table: async () => null,
    usage: async () => null,
    cmdline: async () => null,
    ports: async () => null,
    dispose: () => undefined,
    ...over,
  }
}

const rows = (...r: Array<[number, number, string, string | null]>): CoreProcRow[] =>
  r.map(([pid, ppid, name, startRaw]) => ({ pid, ppid, name, startMs: startRaw ? 1 : null, startRaw }))

afterEach(() => {
  vi.unstubAllEnvs()
  setSharedCoreProcs(undefined)
})

describe('CCM_CORE_PROCS flag', () => {
  it('is node unless set to rust, and then there is no client', () => {
    expect(coreProcsMode({})).toBe('node')
    expect(coreProcsMode({ CCM_CORE_PROCS: 'node' })).toBe('node')
    expect(coreProcsMode({ CCM_CORE_PROCS: ' Rust ' })).toBe('rust')
    vi.stubEnv('CCM_CORE_PROCS', '')
    expect(sharedCoreProcs()).toBeNull()
  })

  it('rust without a core binary warns once and stays on the legacy path', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    vi.stubEnv('CCM_CORE_PROCS', 'rust')
    vi.stubEnv('CCM_CORE_BIN', '')
    expect(sharedCoreProcs()).toBeNull()
    expect(sharedCoreProcs()).toBeNull()
    expect(warn).toHaveBeenCalledTimes(1)
    warn.mockRestore()
  })
})

describe('procSource adapters', () => {
  it('turns core rows into WinProc with FILETIME bigint starts', () => {
    const table = toWinProcs(rows([10, 4, 'a.exe', '133000000000000000'], [11, 10, 'b.exe', null], [12, 10, 'c', 'Mon Jan  1 00:00:00 2024']))
    expect(table[0]!.start).toBe(133000000000000000n)
    expect(table[1]!.start).toBeNull()
    expect(table[2]!.start).toBeNull()
  })

  it('uses the core client when present and the legacy readers otherwise', async () => {
    setSharedCoreProcs(
      fakeClient({
        table: async () => rows([10, 4, 'a.exe', '5']),
        cmdline: async (pids) => pids.map((pid) => ({ pid, cmd: `c${pid}` })),
        usage: async (pids) => pids.map((pid) => ({ pid, workingSet: 1, privateBytes: 2, cpuMs: 3 })),
      }),
    )
    expect(await tableWin()).toEqual([{ pid: 10, ppid: 4, name: 'a.exe', start: 5n }])
    expect((await commandLinesWin([10])).get(10)).toBe('c10')
    expect((await usageWin([10])).get(10)).toEqual({ privateBytes: 2, workingSet: 1, cpuMs: 3 })
  })

  it('falls back when the core client answers null', async () => {
    setSharedCoreProcs(fakeClient())
    const legacy = await tableWin()
    if (process.platform !== 'win32' || process.env.CCM_NATIVE_PROCS === '0') expect(legacy).toBeNull()
    else expect(legacy?.some((p) => p.pid === process.pid)).toBe(true)
  })

  it('queryProcessesWindows accepts an async table source', async () => {
    const start = 133000000000000000n
    const out = await queryProcessesWindows(
      [7],
      async () => [{ pid: 7, ppid: 1, name: 'x.exe', start }],
      async () => null,
    )
    expect(out?.get(7)?.start).toEqual({ kind: 'filetime', value: start })
  })

  it('queryProcessesWindows still takes a sync table source and falls through to CIM on null', async () => {
    const cim = vi.fn(async () => new Map())
    await queryProcessesWindows([7], () => null, cim)
    expect(cim).toHaveBeenCalledTimes(1)
  })
})

describe('coreFirstPortTable', () => {
  it('maps core rows and falls back to the legacy table', async () => {
    const legacy = vi.fn(async () => new Map())
    const viaCore = coreFirstPortTable(legacy, async () => [{ pid: 10, state: 2, address: '0.0.0.0', port: 3000 }])
    expect((await viaCore!())?.get(10)).toEqual([{ port: 3000, address: '0.0.0.0', established: 0 }])
    expect(legacy).not.toHaveBeenCalled()
    await coreFirstPortTable(legacy, async () => null)!()
    expect(legacy).toHaveBeenCalledTimes(1)
    expect(coreFirstPortTable(null)).toBeNull()
  })
})

describe('coreReader', () => {
  const legacyRows: ProcRow[] = [{ pid: 1, ppid: 0, name: 'legacy', start: null, startMs: null, ramBytes: null, cpuMs: null }]
  const legacy: ProcReader = { read: async () => legacyRows, enrich: vi.fn() }
  const counting = () => {
    const usage = vi.fn(async (pids: readonly number[]) => pids.map((pid) => ({ pid, workingSet: pid * 10, privateBytes: 1, cpuMs: pid })))
    const cmdline = vi.fn(async (pids: readonly number[]) => pids.map((pid) => ({ pid, cmd: `c${pid}` })))
    const client = fakeClient({ table: async () => rows([10, 4, 'a.exe', '5'], [11, 10, 'b.exe', '6'], [12, 10, 'c.exe', '7']), usage, cmdline })
    return { client, usage, cmdline }
  }

  it('asks the client only for the pids enrich names and caches command lines (native Windows style)', async () => {
    const { client, usage, cmdline } = counting()
    const reader = coreReader(client, legacy, false)
    const first = (await reader.read())!
    expect(usage).not.toHaveBeenCalled()
    expect(cmdline).not.toHaveBeenCalled()
    await reader.enrich(first, new Set([10]), new Set([11]))
    expect(usage.mock.calls).toEqual([[[10]]])
    expect(cmdline.mock.calls).toEqual([[[11]]])
    expect(first[0]).toMatchObject({ ramBytes: 100, cpuMs: 10 })
    expect(first[1]).toMatchObject({ ramBytes: null, cmd: 'c11' })
    expect(first[2]!.cmd).toBeUndefined()
    const second = (await reader.read())!
    expect(second[1]!.cmd).toBe('c11')
    await reader.enrich(second, new Set(), new Set([11]))
    expect(cmdline).toHaveBeenCalledTimes(1)
  })

  it('fills every row on the first read for the unix style and keeps command lines cached', async () => {
    const { client, usage, cmdline } = counting()
    const reader = coreReader(client, legacy, true)
    const first = (await reader.read())!
    expect(usage.mock.calls).toEqual([[[10, 11, 12]]])
    expect(cmdline.mock.calls).toEqual([[[10, 11, 12]]])
    expect(first[0]).toMatchObject({ ramBytes: 100, cpuMs: 10, cmd: 'c10' })
    const second = (await reader.read())!
    expect(second.map((r) => r.cmd)).toEqual(['c10', 'c11', 'c12'])
    expect(cmdline).toHaveBeenCalledTimes(1)
    expect(usage).toHaveBeenCalledTimes(2)
  })

  it('delegates read and enrich to the legacy reader when the core table is missing', async () => {
    const reader = coreReader(fakeClient(), legacy, false)
    expect(await reader.read()).toBe(legacyRows)
    await reader.enrich(legacyRows, new Set([1]), new Set())
    expect(legacy.enrich).toHaveBeenCalledTimes(1)
  })
})
