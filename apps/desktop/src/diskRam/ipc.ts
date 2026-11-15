import type { IpcMain, IpcMainInvokeEvent, MessageBoxOptions } from 'electron'
import type { RamKillResult, RamProc, RamSnapshot } from '@ccm/shared'
import { formatRam } from '../resources/kill'
import { diskAppsText } from '../diskApps/strings'
import { parseRamTargets, RamSampler, sameStart, type RamSamplerDeps } from './sampler'

export const DISKRAM_CHANNELS = ['ccm:diskram-snapshot', 'ccm:diskram-kill'] as const

export interface DiskRamOptions extends RamSamplerDeps {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  liveStarts: (pids: number[]) => Promise<Map<number, string>>
  killTree: (pid: number) => Promise<boolean>
  confirm: (event: IpcMainInvokeEvent, opts: MessageBoxOptions) => Promise<boolean>
  autoConfirm: boolean
  locale: () => string
}

export interface DiskRam {
  sampler: RamSampler
  kill: (event: IpcMainInvokeEvent, raw: unknown) => Promise<RamKillResult>
  dispose: () => void
}

export function killLines(items: readonly RamProc[]): string[] {
  return items.map((i) => `${i.name}, ${formatRam(i.ramBytes)}, pid ${i.pid}`)
}

export function createDiskRam(opts: DiskRamOptions): DiskRam {
  const sampler = new RamSampler(opts)
  let busy = false
  const kill = async (event: IpcMainInvokeEvent, raw: unknown): Promise<RamKillResult> => {
    const targets = parseRamTargets(raw)
    if (!targets) return { ok: false, killed: 0, error: 'invalid' }
    if (busy) return { ok: false, killed: 0, error: 'busy' }
    busy = true
    try {
      const items: RamProc[] = []
      for (const t of targets) {
        const row = sampler.get(t.pid)
        if (!row || !row.start || !sameStart(row.start, t.start)) return { ok: false, killed: 0, error: 'gone' }
        if (row.lock) return { ok: false, killed: 0, error: 'locked', lock: row.lock }
        items.push(row)
      }
      const before = await opts.liveStarts(items.map((i) => i.pid))
      if (!items.every((i) => sameStart(before.get(i.pid), i.start))) return { ok: false, killed: 0, error: 'gone' }
      const text = diskAppsText(opts.locale())
      const box: MessageBoxOptions = {
        type: 'warning',
        message: text.killTitle(items.length, items[0]!.name),
        detail: text.killDetail(killLines(items)),
        buttons: [text.killConfirm(items.length), text.cancel],
        defaultId: 1,
        cancelId: 1,
        noLink: true,
      }
      if (!opts.autoConfirm && !(await opts.confirm(event, box))) return { ok: false, killed: 0, error: 'cancelled' }
      const live = await opts.liveStarts(items.map((i) => i.pid))
      let killed = 0
      for (const i of items) {
        if (!sameStart(live.get(i.pid), i.start)) continue
        if (await opts.killTree(i.pid)) killed++
      }
      return killed ? { ok: true, killed } : { ok: false, killed: 0, error: 'failed' }
    } finally {
      busy = false
    }
  }
  const guard = <T>(fallback: T, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<T>) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!opts.isAppFrame(event)) return fallback
    try {
      return await fn(event, arg)
    } catch {
      return fallback
    }
  }
  const empty: RamSnapshot = { at: 0, ok: false, totalMem: 0, freeMem: 0, cpus: 1, procs: [] }
  opts.ipcMain.handle('ccm:diskram-snapshot', guard<RamSnapshot>(empty, () => sampler.snapshot()))
  opts.ipcMain.handle('ccm:diskram-kill', guard<RamKillResult>({ ok: false, killed: 0, error: 'failed' }, kill))
  return {
    sampler,
    kill,
    dispose: () => {
      for (const c of DISKRAM_CHANNELS) opts.ipcMain.removeHandler(c)
    },
  }
}
