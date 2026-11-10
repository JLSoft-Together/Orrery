import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { isDiskWarnGb, type DiskState } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { DiskWatch } from './watch'

export interface DiskIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  watch: DiskWatch
  /** Open project folders the page reports; only ever measured, never written. */
  setProjects: (dirs: string[]) => void
  openFolder: (dir: string) => Promise<boolean>
  desk?: DeskRouter
}

const CHANNELS = ['ccm:disk-state', 'ccm:disk-refresh', 'ccm:disk-set', 'ccm:disk-projects', 'ccm:disk-open']
const PROJECTS_MAX = 50

export function registerDiskIpc({ ipcMain, isAppFrame, watch, setProjects, openFolder, desk }: DiskIpcDeps): () => void {
  const hosts = { 'disk.open': (p: unknown) => openFolder(String((p as { dir?: unknown } | null)?.dir ?? '')) }
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: 'disk', hosts: () => hosts } : undefined)
  handle('ccm:disk-state', (event): DiskState | null => (isAppFrame(event) ? watch.state : null))
  handle('ccm:disk-refresh', (event): Promise<DiskState> | null => (isAppFrame(event) ? watch.measure() : null))
  handle('ccm:disk-set', (event, raw: unknown) => {
    const gb = raw && typeof raw === 'object' ? (raw as { warnGb?: unknown }).warnGb : undefined
    return isAppFrame(event) && isDiskWarnGb(gb) ? watch.setWarnGb(gb) : null
  })
  handle('ccm:disk-projects', (event, raw: unknown) => {
    if (!isAppFrame(event) || !Array.isArray(raw)) return false
    setProjects(raw.filter((d): d is string => typeof d === 'string' && d.length > 0 && d.length <= 1024).slice(0, PROJECTS_MAX))
    return true
  })
  // Only a drive Orrery measured: the page cannot open an arbitrary path through this.
  handle('ccm:disk-open', (event, raw: unknown) => {
    const drive = isAppFrame(event) && typeof raw === 'string' ? watch.state.drives.find((d) => d.mount === raw) : undefined
    return drive ? openFolder(drive.mount) : false
  })
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}

export interface DiskControl {
  start: () => Promise<unknown>
  stop: () => Promise<unknown>
  poke: (ms?: number) => Promise<unknown>
}

export function diskControl(watch: DiskWatch, desk?: DeskRouter): DiskControl {
  const via = (channel: string, node: (arg: { ms?: number } | null) => unknown) => {
    const run = (_e: unknown, arg: unknown) => node(arg as { ms?: number } | null)
    const routed = desk ? desk.route('disk', channel, () => true, run) : run
    return (arg: { ms?: number } | null) => Promise.resolve(routed(null, arg)).catch(() => undefined)
  }
  const start = via('desk:disk-start', () => watch.start())
  const stop = via('desk:disk-stop', () => watch.stop())
  const poke = via('desk:disk-poke', (arg) => watch.poke(arg?.ms))
  return { start: () => start(null), stop: () => stop(null), poke: (ms) => poke(ms === undefined ? {} : { ms }) }
}
