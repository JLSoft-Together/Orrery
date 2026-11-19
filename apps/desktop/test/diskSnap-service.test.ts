import { mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import type { DiskStatus, DiskVolume } from '@ccm/shared'
import type { DiskMap } from '../src/diskMap/ipc'
import { cleanCompare, createDiskSnap, type DiskSnapOptions } from '../src/diskMap/snap'

const dirs: string[] = []
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'disksnap-svc-'))
  dirs.push(d)
  return d
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

const GB = 1024 ** 3
const HOME = 'C:\\Users\\dauph'

function rig(over: Partial<DiskSnapOptions> = {}) {
  const state = {
    status: { scanning: false, paused: false, indexed: true, roots: ['C:\\'] } as DiskStatus,
    free: 100 * GB,
    big: 10 * GB,
    summaryFails: false,
    controls: [] as string[],
    scans: [] as unknown[],
  }
  const handlers = new Map<string, (e: unknown, a: unknown) => unknown>()
  const sent: string[] = []
  const events: { channel: string; payload: unknown }[] = []
  const clock = { now: 1_800_000_000_000 }
  const diskMap = {
    host: {
      call: async (channel: string) => {
        if (channel !== 'disk.summary') throw new Error('unexpected')
        if (state.summaryFails) throw new Error('boom')
        return { roots: [{ path: 'C:\\', size: 50 * GB, dirs: [['C:\\Users', state.big + 5 * GB], ['C:\\Users\\dauph', state.big], ['C:\\Windows', 20 * GB]] }] }
      },
    },
    service: {
      status: async () => state.status,
      volumes: async (): Promise<DiskVolume[]> => [{ mount: 'C:\\', label: 'OS', fs: 'NTFS', kind: 'fixed', network: false, total: 500 * GB, free: state.free }],
      scan: async (req: unknown) => (state.scans.push(req), { started: true }),
      control: async (op: string) => (state.controls.push(op), true),
    },
  } as unknown as DiskMap
  const snap = createDiskSnap({
    ipcMain: { handle: (c, fn) => void handlers.set(c, fn as never), removeHandler: (c) => void handlers.delete(c) },
    isAppFrame: () => true,
    diskMap,
    dataDir: tmp(),
    platform: 'win32',
    home: HOME,
    locale: () => 'en',
    broadcast: (channel, payload) => events.push({ channel, payload }),
    idleSeconds: () => 3600,
    onBattery: () => false,
    telegramOn: async () => true,
    send: async (t) => (sent.push(t), true),
    now: () => clock.now,
    tickMs: 3_600_000,
    ...over,
  })
  const call = (channel: string, arg?: unknown): unknown => handlers.get(channel)!({}, arg)
  return { snap, state, handlers, sent, events, clock, call }
}

describe('createDiskSnap', () => {
  it('writes a snapshot after a finished scan and tells the page', async () => {
    const r = rig()
    await r.snap.onDone({ ok: true })
    const list = (await r.call('ccm:diskmap-snap-list')) as { id: string; bytes: number }[]
    expect(list).toHaveLength(1)
    expect(list[0]!.bytes).toBe(50 * GB)
    expect(r.events.map((e) => e.channel)).toContain('ccm:diskmap-snap-new')
    r.snap.dispose()
  })

  it('writes nothing for a failed or cancelled scan', async () => {
    const r = rig()
    await r.snap.onDone({ ok: false, cancelled: true })
    await r.snap.onDone({ ok: false, error: 'x' })
    expect(await r.call('ccm:diskmap-snap-list')).toEqual([])
    r.snap.dispose()
  })

  it('writes nothing when there is no index or the summary fails', async () => {
    const r = rig()
    r.state.summaryFails = true
    await r.snap.onDone({ ok: true })
    r.state.summaryFails = false
    r.state.status = { ...r.state.status, indexed: false }
    await r.snap.onDone({ ok: true })
    expect(await r.call('ccm:diskmap-snap-list')).toEqual([])
    r.snap.dispose()
  })

  it('a user scan writes a snapshot but sends no Telegram', async () => {
    const r = rig()
    await r.snap.onDone({ ok: true })
    r.state.free = 60 * GB
    r.state.big = 40 * GB
    await r.snap.onDone({ ok: true })
    expect(r.sent).toEqual([])
    expect(((await r.call('ccm:diskmap-snap-list')) as unknown[]).length).toBe(2)
    r.snap.dispose()
  })

  it('a background scan that dropped free space by more than 5 GB sends one report', async () => {
    const r = rig()
    await r.snap.onDone({ ok: true })
    r.state.free = 60 * GB
    r.state.big = 40 * GB
    await r.snap.scheduler.tick()
    expect(r.state.scans).toEqual([{ roots: ['C:\\'], level: 'light', excludes: [] }])
    await r.snap.onDone({ ok: true })
    expect(r.sent).toHaveLength(1)
    expect(r.sent[0]).toContain('Background disk scan finished')
    expect(r.sent[0]).toContain('~')
    expect(r.sent[0]).not.toContain(HOME)
    r.snap.dispose()
  })

  it('a background scan with a small drop sends nothing, and the first snapshot has no baseline', async () => {
    const r = rig()
    await r.snap.scheduler.tick()
    await r.snap.onDone({ ok: true })
    expect(r.sent).toEqual([])
    r.clock.now += 2 * 24 * 3600_000
    r.state.free = 98 * GB
    r.state.status = { ...r.state.status, scanning: false }
    await r.snap.scheduler.tick()
    await r.snap.onDone({ ok: true })
    expect(r.sent).toEqual([])
    r.snap.dispose()
  })

  it('stays silent when Telegram is off or the switch is off', async () => {
    const off = rig({ telegramOn: async () => false })
    await off.snap.onDone({ ok: true })
    off.state.free = 10 * GB
    await off.snap.scheduler.tick()
    await off.snap.onDone({ ok: true })
    expect(off.sent).toEqual([])
    off.snap.dispose()

    const sw = rig()
    await sw.snap.onDone({ ok: true })
    await sw.call('ccm:diskmap-schedule-set', { telegram: false })
    sw.state.free = 10 * GB
    await sw.snap.scheduler.tick()
    await sw.snap.onDone({ ok: true })
    expect(sw.sent).toEqual([])
    sw.snap.dispose()
  })

  it('a disk alert compares live free space with the latest snapshot', async () => {
    const r = rig()
    expect(await r.snap.onAlert()).toBe(false)
    await r.snap.onDone({ ok: true })
    r.state.free = 99 * GB
    expect(await r.snap.onAlert()).toBe(false)
    r.state.free = 80 * GB
    expect(await r.snap.onAlert()).toBe(true)
    expect(r.sent).toHaveLength(1)
    expect(r.sent[0]).toContain('Disk almost full')
    expect(await r.snap.onAlert()).toBe(false)
    r.snap.dispose()
  })

  it('overlapping done events write every snapshot in order', async () => {
    const r = rig()
    await Promise.all([r.snap.onDone({ ok: true }), r.snap.onDone({ ok: true }), r.snap.onDone({ ok: true })])
    expect(((await r.call('ccm:diskmap-snap-list')) as unknown[]).length).toBe(3)
    r.snap.dispose()
  })

  it('compares the live index with a snapshot, folder by folder', async () => {
    const r = rig()
    await r.snap.onDone({ ok: true })
    const [meta] = (await r.call('ccm:diskmap-snap-list')) as { id: string }[]
    r.state.big = 40 * GB
    const top = (await r.call('ccm:diskmap-snap-compare', { id: meta!.id })) as { rows: { path: string }[]; top: { path: string; delta: number }[]; covered: boolean }
    expect(top.covered).toBe(true)
    expect(top.rows.map((x) => x.path)).toEqual(['C:\\'])
    expect(top.top[0]).toMatchObject({ path: 'C:\\Users\\dauph', delta: 30 * GB })
    const users = (await r.call('ccm:diskmap-snap-compare', { id: meta!.id, path: 'C:\\Users' })) as { rows: { name: string; delta: number; state: string }[] }
    expect(users.rows).toEqual([expect.objectContaining({ name: 'dauph', delta: 30 * GB, state: 'changed' })])
    r.snap.dispose()
  })

  it('compare refuses bad input and unknown snapshots', async () => {
    const r = rig()
    await r.snap.onDone({ ok: true })
    for (const bad of [null, {}, { id: '../x' }, { id: '00000009-1800000000000' }, { id: '00000001-1800000000000', path: 5 }]) expect(await r.call('ccm:diskmap-snap-compare', bad)).toBeNull()
    expect(cleanCompare({ id: '00000001-1800000000000', path: '' })).toEqual({ id: '00000001-1800000000000' })
    expect(cleanCompare({ id: '00000001-1800000000000', path: 'x'.repeat(3000) })).toBeNull()
    r.snap.dispose()
  })

  it('schedule get and set go through the scheduler', async () => {
    const r = rig()
    expect(await r.call('ccm:diskmap-schedule-get')).toMatchObject({ freq: 'daily', telegram: true, last: null, running: false })
    expect(await r.call('ccm:diskmap-schedule-set', { freq: 'weekly' })).toMatchObject({ freq: 'weekly' })
    expect(await r.call('ccm:diskmap-schedule-set', { freq: 'nope', telegram: 1 })).toMatchObject({ freq: 'weekly', telegram: true })
    expect(r.events.some((e) => e.channel === 'ccm:diskmap-schedule')).toBe(true)
    r.snap.dispose()
  })

  it('answers from the page only', async () => {
    const r = rig({ isAppFrame: () => false })
    expect(await r.call('ccm:diskmap-snap-list')).toEqual([])
    expect(await r.call('ccm:diskmap-snap-compare', { id: '00000001-1800000000000' })).toBeNull()
    expect(await r.call('ccm:diskmap-schedule-set', { freq: 'off' })).toMatchObject({ freq: 'off', last: null })
    expect(r.snap.scheduler.state.freq).toBe('daily')
    r.snap.dispose()
  })

  it('beforeScan cancels a running background scan', async () => {
    const r = rig()
    await r.snap.scheduler.tick()
    expect(r.snap.scheduler.bgRunning).toBe(true)
    const p = r.snap.beforeScan()
    await Promise.resolve()
    expect(r.state.controls).toContain('cancel')
    await r.snap.onDone({ ok: false, cancelled: true })
    await p
    expect(r.snap.scheduler.bgRunning).toBe(false)
    r.snap.dispose()
  })

  it('dispose removes the handlers', () => {
    const r = rig()
    expect(r.handlers.size).toBe(4)
    r.snap.dispose()
    expect(r.handlers.size).toBe(0)
  })
})
