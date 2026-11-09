import { createCoreProcs, type CoreCmdRow, type CorePortRow, type CoreProcRow, type CoreProcsClient } from './coreProcs'
import { nativeCommandLine, nativeProcessTable, nativeUsage, type ProcUsage, type WinProc } from './win-procs'

export type CoreProcsMode = 'rust' | 'node'

export function coreProcsMode(env: NodeJS.ProcessEnv = process.env): CoreProcsMode {
  return env.CCM_CORE_PROCS?.trim().toLowerCase() === 'rust' ? 'rust' : 'node'
}

let shared: CoreProcsClient | null | undefined

export function sharedCoreProcs(): CoreProcsClient | null {
  if (shared !== undefined) return shared
  shared = null
  if (coreProcsMode() !== 'rust') return shared
  const bin = process.env.CCM_CORE_BIN?.trim()
  if (!bin) {
    console.warn('[ccm] CCM_CORE_PROCS=rust but CCM_CORE_BIN is not set, using the legacy process reader')
    return shared
  }
  shared = createCoreProcs({ bin })
  return shared
}

export function setSharedCoreProcs(next: CoreProcsClient | null | undefined): void {
  shared?.dispose()
  shared = next
}

export const coreProcsEnabled = (): boolean => sharedCoreProcs() !== null

export async function coreRows(): Promise<CoreProcRow[] | null> {
  const client = sharedCoreProcs()
  return client ? client.table() : null
}

export async function corePorts(): Promise<CorePortRow[] | null> {
  const client = sharedCoreProcs()
  return client ? client.ports() : null
}

const FILETIME = /^\d+$/

export function toWinProcs(rows: readonly CoreProcRow[]): WinProc[] {
  return rows.map((r) => ({ pid: r.pid, ppid: r.ppid, name: r.name, start: r.startRaw !== null && FILETIME.test(r.startRaw) ? BigInt(r.startRaw) : null }))
}

export type TableWin = () => WinProc[] | null | Promise<WinProc[] | null>

export async function tableWin(): Promise<WinProc[] | null> {
  const rows = await coreRows()
  if (rows) return toWinProcs(rows)
  return nativeProcessTable()
}

export async function commandLinesWin(pids: readonly number[]): Promise<Map<number, string>> {
  const client = sharedCoreProcs()
  const rows: CoreCmdRow[] | null = client ? await client.cmdline(pids) : null
  if (rows) return new Map(rows.map((r) => [r.pid, r.cmd]))
  const out = new Map<number, string>()
  for (const pid of pids) {
    const cmd = nativeCommandLine(pid)
    if (cmd !== null) out.set(pid, cmd)
  }
  return out
}

export async function usageWin(pids: readonly number[]): Promise<Map<number, ProcUsage>> {
  const client = sharedCoreProcs()
  const rows = client ? await client.usage(pids) : null
  if (rows) return new Map(rows.map((r) => [r.pid, { privateBytes: r.privateBytes, workingSet: r.workingSet, cpuMs: r.cpuMs }]))
  const out = new Map<number, ProcUsage>()
  for (const pid of pids) {
    const usage = nativeUsage(pid)
    if (usage) out.set(pid, usage)
  }
  return out
}
