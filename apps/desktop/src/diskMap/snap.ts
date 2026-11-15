import path from 'node:path'
import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { DISK_SNAPSHOT_DEPTH, isDiskFreq, type DiskVolume, type DiskCompare, type DiskCompareRequest, type DiskDone, type DiskSchedule, type DiskSchedulePatch, type DiskSnapshotMeta } from '@ccm/shared'
import type { DiskMap } from './ipc'
import { DiskReporter, type ReportLocale } from './report'
import { Scheduler, SCHEDULE_FILE } from './schedule'
import { buildCompared, compareAt, isSnapshotId, SnapshotStore, toSnapshot, type Compared, type Snapshot } from './snapshots'

export const SNAP_CHANNELS = ['ccm:diskmap-snap-list', 'ccm:diskmap-snap-compare', 'ccm:diskmap-schedule-get', 'ccm:diskmap-schedule-set'] as const
export const SCHEDULE_EVENT = 'ccm:diskmap-schedule'
export const SNAP_EVENT = 'ccm:diskmap-snap-new'
const COMPARE_TTL_MS = 60_000
const PATH_MAX = 2048

export interface DiskSnapOptions {
  ipcMain: Pick<IpcMain, 'handle' | 'removeHandler'>
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  diskMap: DiskMap
  dataDir: string
  platform: NodeJS.Platform
  home: string
  locale: () => ReportLocale
  broadcast: (channel: string, payload: unknown) => void
  idleSeconds: () => number
  onBattery: () => boolean
  telegramOn: () => Promise<boolean>
  send: (text: string) => Promise<boolean>
  now?: () => number
  tickMs?: number
  store?: SnapshotStore
  volumes?: () => Promise<DiskVolume[]>
}

export interface DiskSnap {
  store: SnapshotStore
  scheduler: Scheduler
  reporter: DiskReporter
  capture: () => Promise<Snapshot | null>
  onDone: (done: DiskDone) => Promise<void>
  onAlert: () => Promise<boolean>
  beforeScan: () => Promise<void>
  compare: (req: unknown) => Promise<DiskCompare | null>
  dispose: () => void
}

export function cleanCompare(raw: unknown): DiskCompareRequest | null {
  const o = raw && typeof raw === 'object' ? (raw as { id?: unknown; path?: unknown }) : null
  if (!o || !isSnapshotId(o.id)) return null
  if (o.path === undefined || o.path === '') return { id: o.id }
  if (typeof o.path !== 'string' || o.path.length > PATH_MAX || o.path.includes('\0')) return null
  return { id: o.id, path: o.path }
}

export function createDiskSnap(opts: DiskSnapOptions): DiskSnap {
  const { diskMap, platform } = opts
  const store = opts.store ?? new SnapshotStore(path.join(opts.dataDir, 'disk', 'snapshots'))
  let chain: Promise<unknown> = Promise.resolve()
  const serial = <T>(fn: () => Promise<T>): Promise<T> => {
    const run = chain.then(fn, fn)
    chain = run.catch(() => undefined)
    return run
  }

  const capture = async (): Promise<Snapshot | null> => {
    const st = await diskMap.service.status().catch(() => null)
    if (!st || !st.indexed || !st.roots.length) return null
    const summary = await diskMap.host.call('disk.summary', { roots: st.roots, depth: DISK_SNAPSHOT_DEPTH }).catch(() => null)
    const volumes = (await (opts.volumes ?? (() => diskMap.service.volumes()))().catch(() => [] as DiskVolume[])).filter((v) => !v.network).map((v) => ({ mount: v.mount, label: v.label, free: v.free, total: v.total }))
    return toSnapshot(summary, volumes, (opts.now ?? Date.now)())
  }

  const scheduler = new Scheduler({
    file: path.join(opts.dataDir, 'disk', SCHEDULE_FILE),
    now: opts.now,
    tickMs: opts.tickMs,
    idleSeconds: opts.idleSeconds,
    onBattery: opts.onBattery,
    status: async () => {
      const st = await diskMap.service.status()
      return { indexed: st.indexed, scanning: st.scanning, roots: st.roots }
    },
    start: async (roots) => (await diskMap.service.scan({ roots, level: 'light', excludes: [] })).started,
    control: (op) => diskMap.service.control(op),
    onChange: (state) => opts.broadcast(SCHEDULE_EVENT, state),
  })

  const reporter = new DiskReporter({
    platform,
    home: opts.home,
    locale: opts.locale,
    enabled: () => scheduler.telegram,
    send: async (text) => ((await opts.telegramOn().catch(() => false)) ? opts.send(text) : false),
    now: opts.now,
  })

  let cache: { id: string; at: number; base: DiskSnapshotMeta; compared: Compared } | null = null

  const onDone = (done: DiskDone): Promise<void> => {
    const bg = scheduler.noteDone(done)
    cache = null
    if (!done.ok) return Promise.resolve()
    return serial(async () => {
      const [prev, cur] = [await store.latest().catch(() => null), await capture()]
      if (!cur) return
      const meta = await store.write(cur).catch(() => null)
      if (meta) opts.broadcast(SNAP_EVENT, meta)
      if (bg && prev) await reporter.report(prev.snap, cur, 'bgscan')
    }).catch(() => undefined)
  }

  const onAlert = (): Promise<boolean> =>
    serial(async () => {
      const base = await store.latest()
      const cur = base ? await capture() : null
      return base && cur ? reporter.report(base.snap, cur, 'alert') : false
    }).catch(() => false)

  const compare = async (raw: unknown): Promise<DiskCompare | null> => {
    const req = cleanCompare(raw)
    if (!req) return null
    const now = (opts.now ?? Date.now)()
    if (!req.path || !cache || cache.id !== req.id || now - cache.at > COMPARE_TTL_MS) {
      const [snap, metas, cur] = await Promise.all([store.read(req.id), store.list(), capture()])
      const base = metas.find((m) => m.id === req.id)
      if (!snap || !base || !cur) return null
      cache = { id: req.id, at: now, base, compared: buildCompared(snap, cur, platform) }
    }
    return compareAt(cache.compared, req.path ?? '', platform, cache.base)
  }

  const { ipcMain, isAppFrame } = opts
  const guard = <T>(fallback: T, fn: (arg: unknown) => Promise<T> | T) => async (event: IpcMainInvokeEvent, arg: unknown): Promise<T> => {
    if (!isAppFrame(event)) return fallback
    try {
      return await fn(arg)
    } catch {
      return fallback
    }
  }
  const idle: DiskSchedule = { freq: 'off', telegram: false, last: null, running: false, paused: false }
  ipcMain.handle('ccm:diskmap-snap-list', guard<DiskSnapshotMeta[]>([], () => store.list()))
  ipcMain.handle('ccm:diskmap-snap-compare', guard<DiskCompare | null>(null, (arg) => compare(arg)))
  ipcMain.handle('ccm:diskmap-schedule-get', guard<DiskSchedule>(idle, () => scheduler.state))
  ipcMain.handle('ccm:diskmap-schedule-set', guard<DiskSchedule>(idle, (arg) => {
    const o = arg && typeof arg === 'object' ? (arg as { freq?: unknown; telegram?: unknown }) : {}
    const patch: DiskSchedulePatch = {}
    if (isDiskFreq(o.freq)) patch.freq = o.freq
    if (typeof o.telegram === 'boolean') patch.telegram = o.telegram
    return scheduler.set(patch)
  }))
  scheduler.start()

  return {
    store,
    scheduler,
    reporter,
    capture,
    onDone,
    onAlert,
    beforeScan: () => scheduler.preempt(),
    compare,
    dispose: () => {
      scheduler.stop()
      for (const c of SNAP_CHANNELS) ipcMain.removeHandler(c)
    },
  }
}
