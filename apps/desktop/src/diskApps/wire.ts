import { statSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import type { IpcMain, IpcMainInvokeEvent, MessageBoxOptions } from 'electron'
import type { DiskHistoryEntry } from '@ccm/shared'
import { defaultProcReader } from '../../../collector/src/resources/procs'
import type { DeleteAsk } from '../diskMap/delete'
import { createDiskRam } from '../diskRam/ipc'
import type { RawProc } from '../diskRam/sampler'
import { killTreeByPid, liveStarts } from '../resources/kill'
import { createDiskApps } from './ipc'
import { macLeftoverRoots, windowsLeftoverRoots, type LeftoverRoot } from './leftovers'
import { launchUninstaller } from './uninstall'

export interface DiskAppsWire {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  packaged: boolean
  env: NodeJS.ProcessEnv
  platform: NodeJS.Platform
  home: string
  execPath: string
  confirm: (event: IpcMainInvokeEvent, opts: MessageBoxOptions) => Promise<boolean>
  confirmDelete: (event: IpcMainInvokeEvent, ask: DeleteAsk, second: boolean) => Promise<boolean>
  keep: () => string[]
  keepTree: () => string[]
  locale: () => string
  record: (e: DiskHistoryEntry) => Promise<void>
  indexSizes: (paths: string[]) => Promise<Record<string, number | null> | null>
  trash: (p: string) => Promise<void>
  remove: (p: string) => Promise<void>
  openSettings: () => Promise<boolean>
}

export function e2eEnv(env: NodeJS.ProcessEnv, packaged: boolean, name: string): string | null {
  if (packaged) return null
  const v = env[`CCM_E2E_${name}`]?.trim()
  return v ? v : null
}

export function macSelfBundle(execPath: string): string | null {
  const i = execPath.indexOf('.app/')
  return i > 0 ? execPath.slice(0, i + 4) : null
}

const isFile = (p: string): boolean => {
  try {
    return statSync(p).isFile()
  } catch {
    return false
  }
}

function leftoverRoots(w: DiskAppsWire): LeftoverRoot[] {
  const fake = e2eEnv(w.env, w.packaged, 'DISKAPPS_ROOTS')
  if (fake) {
    return w.platform === 'darwin'
      ? macLeftoverRoots(fake)
      : windowsLeftoverRoots({ APPDATA: path.join(fake, 'Roaming'), LOCALAPPDATA: path.join(fake, 'Local'), ProgramData: path.join(fake, 'ProgramData') })
  }
  return w.platform === 'darwin' ? macLeftoverRoots(w.home) : w.platform === 'win32' ? windowsLeftoverRoots(w.env) : []
}

async function readProcs(): Promise<RawProc[] | null> {
  const reader = defaultProcReader()
  if (!reader) return null
  const rows = await reader.read()
  if (!rows) return null
  const all = new Set(rows.map((r) => r.pid))
  await reader.enrich(rows, all, new Set())
  return rows.map((r) => ({ pid: r.pid, ppid: r.ppid, name: r.name, start: r.start, startMs: r.startMs, ramBytes: r.ramBytes, cpuMs: r.cpuMs }))
}

export function setupDiskAppsRam(w: DiskAppsWire): { dispose: () => void } {
  const appsConfirm = e2eEnv(w.env, w.packaged, 'DISKAPPS_CONFIRM') === '1'
  const apps = createDiskApps({
    ipcMain: w.ipcMain,
    isAppFrame: w.isAppFrame,
    platform: w.platform,
    home: w.home,
    env: w.env,
    regFixture: w.platform === 'win32' ? e2eEnv(w.env, w.packaged, 'DISKAPPS_REG') : null,
    macRoots: ['/Applications', path.posix.join(w.home, 'Applications')],
    selfBundle: w.platform === 'darwin' ? macSelfBundle(w.execPath) : null,
    leftoverRoots: leftoverRoots(w),
    indexSizes: w.indexSizes,
    exists: isFile,
    confirm: w.confirm,
    confirmDelete: w.confirmDelete,
    autoConfirm: appsConfirm,
    launch: launchUninstaller(w.env),
    trash: w.trash,
    remove: w.remove,
    locale: w.locale,
    keep: w.keep,
    keepTree: w.keepTree,
    record: w.record,
    openSettings: w.openSettings,
  })
  const ram = createDiskRam({
    ipcMain: w.ipcMain,
    isAppFrame: w.isAppFrame,
    read: readProcs,
    selfPid: process.pid,
    platform: w.platform,
    cpus: () => os.cpus().length,
    totalMem: () => os.totalmem(),
    freeMem: () => os.freemem(),
    liveStarts: (pids) => liveStarts(pids),
    killTree: (pid) => killTreeByPid(pid),
    confirm: w.confirm,
    autoConfirm: e2eEnv(w.env, w.packaged, 'DISKRAM_CONFIRM') === '1',
    locale: w.locale,
  })
  return {
    dispose: () => {
      apps.dispose()
      ram.dispose()
    },
  }
}
