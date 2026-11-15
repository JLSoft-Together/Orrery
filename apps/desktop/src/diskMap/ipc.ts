import fs from 'node:fs'
import path from 'node:path'
import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { DiskChildren, DiskDeleteResult, DiskDupesStart, DiskFastScanResult, DiskHistoryEntry, DiskSearchResult, DiskSizes, DiskStatus, DiskVolume } from '@ccm/shared'
import { measureRest, runDelete, type DeleteAsk } from './delete'
import { DiskHistory } from './history'
import { DiskMapHost, type DiskMapSpawner } from './host'
import { cleanChildren, cleanDupes, cleanFastScan, cleanHistoryLimit, cleanScan, cleanSearch, cleanSizes, isAbsolutePath, MAX_DELETE, toChildren, toDone, toDupesDone, toDupesProgress, toProgress, toSearch, toSizes, toStatus, toVolumes } from './map'

export const DISKMAP_CHANNELS = [
  'ccm:diskmap-volumes', 'ccm:diskmap-status', 'ccm:diskmap-scan', 'ccm:diskmap-pause', 'ccm:diskmap-resume', 'ccm:diskmap-cancel',
  'ccm:diskmap-children', 'ccm:diskmap-search', 'ccm:diskmap-delete', 'ccm:diskmap-reveal', 'ccm:diskmap-history', 'ccm:diskmap-fastscan', 'ccm:diskmap-sizes',
  'ccm:diskmap-dupes', 'ccm:diskmap-dupes-cancel',
] as const
export const DISKMAP_PROGRESS = 'ccm:diskmap-progress'
export const DISKMAP_DONE = 'ccm:diskmap-done'
export const DISKMAP_DUPES_PROGRESS = 'ccm:diskmap-dupes-progress'
export const DISKMAP_DUPES_DONE = 'ccm:diskmap-dupes-done'

export interface DiskMapOptions {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  bin: string
  env: NodeJS.ProcessEnv
  dataDir: string
  platform: NodeJS.Platform
  home: string
  keep: () => string[]
  broadcast: (channel: string, payload: unknown) => void
  confirm: (event: IpcMainInvokeEvent, ask: DeleteAsk, second: boolean) => Promise<boolean>
  autoConfirm?: boolean
  trash: (path: string) => Promise<void>
  remove: (path: string) => Promise<void>
  reveal: (path: string) => void
  spawner?: DiskMapSpawner
  exists?: (file: string) => boolean
}

export interface DiskMap {
  host: DiskMapHost
  service: DiskMapService
  dispose: () => void
  stop: () => Promise<void>
  kill: () => void
}

const EMPTY_SEARCH: DiskSearchResult = { total: 0, ms: 0, items: [] }

export class DiskMapService {
  private loaded: Promise<void> | null = null
  private deleting = false
  dupesRunning = false
  readonly history: DiskHistory

  constructor(readonly host: DiskMapHost, private readonly opts: Omit<DiskMapOptions, 'ipcMain' | 'isAppFrame' | 'bin' | 'env' | 'spawner' | 'exists'>) {
    this.history = new DiskHistory(path.join(opts.dataDir, 'disk'))
  }

  onExit(): void {
    this.loaded = null
  }

  private ensureLoaded(): Promise<void> {
    this.loaded ??= (async () => {
      const st = toStatus(await this.host.call('disk.status'))
      if (!st.indexed && !st.scanning) await this.host.call('disk.load').catch(() => undefined)
    })().catch((err: unknown) => {
      this.loaded = null
      throw err
    })
    return this.loaded
  }

  async volumes(): Promise<DiskVolume[]> {
    return toVolumes(await this.host.call('disk.volumes').catch(() => []))
  }

  async status(): Promise<DiskStatus> {
    await this.ensureLoaded()
    return toStatus(await this.host.call('disk.status'))
  }

  async scan(req: unknown): Promise<{ started: boolean }> {
    const clean = cleanScan(req, this.opts.platform)
    if (!clean) return { started: false }
    await this.ensureLoaded()
    try {
      await this.host.call('disk.scan', { roots: clean.roots, level: clean.level, excludes: clean.excludes, save: true })
      this.host.markScanning(true)
      return { started: true }
    } catch {
      return { started: false }
    }
  }

  async fastScan(req: unknown): Promise<DiskFastScanResult> {
    const clean = cleanFastScan(req, this.opts.platform)
    if (!clean) return { started: false, error: 'unsupported' }
    await this.ensureLoaded()
    try {
      await this.host.call('disk.mft', { drive: clean.drive, save: true })
      this.host.markScanning(true)
      return { started: true }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      return { started: false, error: ['unsupported', 'busy'].includes(msg) ? msg : 'failed' }
    }
  }

  async dupes(req: unknown): Promise<DiskDupesStart> {
    const clean = cleanDupes(req, this.opts.platform)
    if (!clean) return { started: false, error: 'invalid' }
    await this.ensureLoaded()
    const base = { minSize: clean.minSize, excludes: clean.excludes ?? [], level: clean.level }
    const attempt = async (under: string[]): Promise<void> => {
      await this.host.call('disk.dupes', { ...base, under })
    }
    this.dupesRunning = true
    this.host.markScanning(true)
    try {
      if (clean.under) await attempt(clean.under)
      else {
        try {
          await attempt([this.opts.home])
        } catch (err) {
          if (!(err instanceof Error) || !err.message.startsWith('not in index')) throw err
          const roots = toStatus(await this.host.call('disk.status')).roots
          if (!roots.length) throw err
          await attempt(roots)
        }
      }
      return { started: true }
    } catch (err) {
      this.dupesRunning = false
      const msg = err instanceof Error ? err.message : String(err)
      if (msg !== 'busy') this.host.markScanning(false)
      return { started: false, error: msg.startsWith('not in index') ? 'not-indexed' : msg === 'busy' ? 'busy' : 'failed' }
    }
  }

  async control(op: 'pause' | 'resume' | 'cancel'): Promise<boolean> {
    if (!this.host.running) return false
    const v = (await this.host.call(`disk.${op}`).catch(() => null)) as { ok?: unknown } | null
    return v?.ok === true
  }

  async children(req: unknown): Promise<DiskChildren | null> {
    const clean = cleanChildren(req, this.opts.platform)
    if (!clean) return null
    await this.ensureLoaded()
    return toChildren(await this.host.call('disk.children', clean).catch(() => null))
  }

  async sizes(req: unknown): Promise<DiskSizes | null> {
    const paths = cleanSizes(req, this.opts.platform)
    if (!paths) return null
    if (!this.host.running && !fs.existsSync(path.join(this.opts.dataDir, 'disk', 'index.bin'))) return null
    await this.ensureLoaded()
    return toSizes(await this.host.call('disk.sizes', { paths }).catch(() => null))
  }

  async search(req: unknown): Promise<DiskSearchResult> {
    const q = cleanSearch(req, this.opts.platform)
    if (!q) return EMPTY_SEARCH
    await this.ensureLoaded()
    const v = await this.host.call('disk.search', q).catch(() => null)
    return v ? toSearch(v) : EMPTY_SEARCH
  }

  async remove(event: IpcMainInvokeEvent, req: unknown): Promise<DiskDeleteResult> {
    const o = req && typeof req === 'object' ? (req as { paths?: unknown; permanent?: unknown }) : {}
    const permanent = o.permanent === true
    const paths = Array.isArray(o.paths) ? o.paths : []
    if (!paths.length || paths.length > MAX_DELETE) return { done: [], failed: [], freed: 0, permanent, cancelled: true }
    if (this.deleting) return { done: [], failed: paths.map((p) => ({ path: typeof p === 'string' ? p : '', reason: 'busy' })), freed: 0, permanent }
    this.deleting = true
    try {
      await this.ensureLoaded()
      const ask = (ask: DeleteAsk, second: boolean): Promise<boolean> => (this.opts.autoConfirm ? Promise.resolve(true) : this.opts.confirm(event, ask, second))
      return await runDelete(paths as string[], permanent, {
        guard: async () => ({ platform: this.opts.platform, home: this.opts.home, keep: this.opts.keep(), roots: toStatus(await this.host.call('disk.status')).roots, volumes: (await this.volumes()).map((v) => v.mount).filter(Boolean) }),
        known: async (p) => toChildren(await this.host.call('disk.children', { path: p, limit: 0 }).catch(() => null)) !== null,
        sizeOf: async (p) => toChildren(await this.host.call('disk.children', { path: p, limit: 0 }))?.size ?? null,
        measure: measureRest,
        confirm: (a) => ask(a, false),
        confirmPermanent: (a) => ask(a, true),
        trash: this.opts.trash,
        remove: this.opts.remove,
        forget: async (gone) => {
          await this.host.call('disk.forget', { paths: gone })
          void this.host.call('disk.save').catch(() => undefined)
        },
        record: (entry) => this.history.append(entry),
      })
    } finally {
      this.deleting = false
    }
  }

  reveal(p: unknown): void {
    if (isAbsolutePath(p, this.opts.platform)) this.opts.reveal(p)
  }

  historyRead(req: unknown): Promise<DiskHistoryEntry[]> {
    return this.history.read(cleanHistoryLimit(req))
  }
}

export function createDiskMap(opts: DiskMapOptions): DiskMap {
  let service: DiskMapService | null = null
  const host = new DiskMapHost({
    bin: opts.bin,
    env: { ...opts.env, CCM_DATA_DIR: opts.dataDir },
    spawner: opts.spawner,
    exists: opts.exists,
    onEvent: (event, payload) => {
      if (event === 'disk:progress') opts.broadcast(DISKMAP_PROGRESS, toProgress(payload))
      else if (event === 'disk:done') opts.broadcast(DISKMAP_DONE, toDone(payload))
      else if (event === 'disk:dupes-progress') opts.broadcast(DISKMAP_DUPES_PROGRESS, toDupesProgress(payload))
      else if (event === 'disk:dupes-done') {
        if (service) service.dupesRunning = false
        opts.broadcast(DISKMAP_DUPES_DONE, toDupesDone(payload))
      }
    },
    onExit: (_code, busy) => {
      service?.onExit()
      if (service?.dupesRunning) {
        service.dupesRunning = false
        if (busy) opts.broadcast(DISKMAP_DUPES_DONE, toDupesDone({ ok: false, error: host.lastError ?? 'exited' }))
      } else if (busy) opts.broadcast(DISKMAP_DONE, { ok: false, error: host.lastError ?? 'exited' })
    },
  })
  service = new DiskMapService(host, opts)
  const svc = service
  const { ipcMain, isAppFrame } = opts
  const guard = <T>(fallback: T, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<T> | T) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!isAppFrame(event)) return fallback
    try {
      return await fn(event, arg)
    } catch {
      return fallback
    }
  }
  const idle: DiskStatus = { scanning: false, paused: false, indexed: false, roots: [] }
  ipcMain.handle('ccm:diskmap-volumes', guard<DiskVolume[]>([], () => svc.volumes()))
  ipcMain.handle('ccm:diskmap-status', guard<DiskStatus>(idle, () => svc.status()))
  ipcMain.handle('ccm:diskmap-scan', guard({ started: false }, (_e, req) => svc.scan(req)))
  ipcMain.handle('ccm:diskmap-pause', guard(false, () => svc.control('pause')))
  ipcMain.handle('ccm:diskmap-resume', guard(false, () => svc.control('resume')))
  ipcMain.handle('ccm:diskmap-cancel', guard(false, () => svc.control('cancel')))
  ipcMain.handle('ccm:diskmap-children', guard<DiskChildren | null>(null, (_e, req) => svc.children(req)))
  ipcMain.handle('ccm:diskmap-search', guard<DiskSearchResult>(EMPTY_SEARCH, (_e, req) => svc.search(req)))
  ipcMain.handle('ccm:diskmap-delete', guard<DiskDeleteResult>({ done: [], failed: [], freed: 0, permanent: false, cancelled: true }, (e, req) => svc.remove(e, req)))
  ipcMain.handle('ccm:diskmap-reveal', guard<void>(undefined, (_e, p) => svc.reveal(p)))
  ipcMain.handle('ccm:diskmap-fastscan', guard<DiskFastScanResult>({ started: false, error: 'failed' }, (_e, req) => svc.fastScan(req)))
  ipcMain.handle('ccm:diskmap-sizes', guard<DiskSizes | null>(null, (_e, req) => svc.sizes(req)))
  ipcMain.handle('ccm:diskmap-dupes', guard<DiskDupesStart>({ started: false, error: 'failed' }, (_e, req) => svc.dupes(req)))
  ipcMain.handle('ccm:diskmap-dupes-cancel', guard(false, () => svc.control('cancel')))
  ipcMain.handle('ccm:diskmap-history', guard<DiskHistoryEntry[]>([], (_e, req) => svc.historyRead(req)))
  return {
    host,
    service: svc,
    dispose: () => {
      for (const c of DISKMAP_CHANNELS) ipcMain.removeHandler(c)
    },
    stop: () => host.stop(),
    kill: () => host.kill(),
  }
}
