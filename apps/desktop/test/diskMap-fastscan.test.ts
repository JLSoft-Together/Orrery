import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { PassThrough } from 'node:stream'
import type { IpcMainInvokeEvent } from 'electron'
import { afterEach, describe, expect, it } from 'vitest'
import { LineSplitter } from '../src/diskMap/host'
import { createDiskMap, DISKMAP_CHANNELS, DISKMAP_DONE, DISKMAP_PROGRESS } from '../src/diskMap/ipc'
import { cleanFastScan, toDone } from '../src/diskMap/map'
import { DESK_BIN, HAVE_DESK } from './desk-twin'

const REPO = path.resolve(__dirname, '..', '..', '..')
const FIXTURE = path.join(REPO, 'apps', 'core', 'crates', 'core-disk', 'tests', 'fixtures', 'mft-sample.bin')
const EXPECTED_BYTES = 8192 + 4096 + 65536 + (1 << 20) + 16384

class FakeChild extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  pid = 4242
  exitCode: number | null = null
  kill(): boolean {
    this.exitCode = -1
    this.emit('exit', null)
    return true
  }
}

function fakeCore(answer: (channel: string, args: Record<string, unknown>) => { ok: boolean; value?: unknown; error?: string }) {
  const seen: { channel: string; args: Record<string, unknown> }[] = []
  const spawner = () => {
    const child = new FakeChild()
    const reply = (m: object) => child.stdout.write(`${JSON.stringify(m)}\n`)
    const split = new LineSplitter()
    child.stdin.on('data', (c: Buffer) => {
      for (const l of split.push(String(c))) {
        if (!l.trim()) continue
        const m = JSON.parse(l) as { id: number; channel: string; args: Record<string, unknown> }
        if (!m.channel) continue
        seen.push({ channel: m.channel, args: m.args })
        reply({ id: m.id, ...answer(m.channel, m.args) })
      }
    })
    setImmediate(() => reply({ ready: true, pid: 4242, channels: [] }))
    return child as unknown as ChildProcessWithoutNullStreams
  }
  return { spawner, seen }
}

function fakeIpc() {
  const handlers = new Map<string, (event: IpcMainInvokeEvent, arg: unknown) => Promise<unknown>>()
  const ipcMain = {
    handle: (c: string, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<unknown>) => handlers.set(c, fn),
    removeHandler: (c: string) => handlers.delete(c),
  }
  const ok = { sender: { id: 1 } } as unknown as IpcMainInvokeEvent
  return { ipcMain, handlers, call: (c: string, arg?: unknown) => handlers.get(c)!(ok, arg) }
}

function make(platform: NodeJS.Platform, opts: { spawner?: ReturnType<typeof fakeCore>['spawner']; env?: NodeJS.ProcessEnv; bin?: string; dataDir?: string; frame?: boolean } = {}) {
  const ipc = fakeIpc()
  const broadcasts: [string, unknown][] = []
  const map = createDiskMap({
    ipcMain: ipc.ipcMain,
    isAppFrame: () => opts.frame !== false,
    bin: opts.bin ?? 'fake',
    env: opts.env ?? {},
    dataDir: opts.dataDir ?? os.tmpdir(),
    platform,
    home: os.homedir(),
    keep: () => [],
    broadcast: (c, p) => broadcasts.push([c, p]),
    confirm: async () => false,
    trash: async () => undefined,
    remove: async () => undefined,
    reveal: () => undefined,
    spawner: opts.spawner,
    exists: opts.spawner ? () => true : undefined,
  })
  return { ipc, map, broadcasts }
}

const dirs: string[] = []
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

describe('diskMap fast scan (MFT)', () => {
  it('accepts only one drive letter on Windows', () => {
    expect(cleanFastScan({ drive: 'c' }, 'win32')).toEqual({ drive: 'C' })
    expect(cleanFastScan({ drive: 'D:' }, 'win32')).toEqual({ drive: 'D' })
    expect(cleanFastScan({ drive: 'e:\\' }, 'win32')).toEqual({ drive: 'E' })
    expect(cleanFastScan({ drive: 'F:/' }, 'win32')).toEqual({ drive: 'F' })
    for (const bad of [{ drive: 'CD' }, { drive: 'C:\\Users' }, { drive: '\\\\srv\\share' }, { drive: '1' }, { drive: 'C' + '\0' }, { drive: 3 }, {}, null, 'C']) expect(cleanFastScan(bad, 'win32')).toBeNull()
    expect(cleanFastScan({ drive: 'C' }, 'darwin')).toBeNull()
    expect(toDone({ ok: false, error: 'elevation-declined', mft: true })).toEqual({ ok: false, error: 'elevation-declined', mft: true })
  })

  it('registers the channel, forwards a clean drive and maps core errors', async () => {
    let next: { ok: boolean; value?: unknown; error?: string } = { ok: true, value: { started: true, mft: true } }
    const core = fakeCore((channel) => (channel === 'disk.mft' ? next : channel === 'disk.status' ? { ok: true, value: { scanning: false, paused: false, index: { roots: [] } } } : { ok: true, value: null }))
    const { ipc, map } = make('win32', { spawner: core.spawner })
    try {
      expect(DISKMAP_CHANNELS).toContain('ccm:diskmap-fastscan')
      expect(ipc.handlers.has('ccm:diskmap-fastscan')).toBe(true)
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'c:' })).toEqual({ started: true })
      expect(core.seen.find((s) => s.channel === 'disk.mft')?.args).toEqual({ drive: 'C', save: true })
      next = { ok: false, error: 'busy' }
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'C' })).toEqual({ started: false, error: 'busy' })
      next = { ok: false, error: 'unsupported' }
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'C' })).toEqual({ started: false, error: 'unsupported' })
      next = { ok: false, error: 'pipe create failed (5) C:\\secret' }
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'C' })).toEqual({ started: false, error: 'failed' })
      const before = core.seen.filter((s) => s.channel === 'disk.mft').length
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'C:\\Windows' })).toEqual({ started: false, error: 'unsupported' })
      expect(core.seen.filter((s) => s.channel === 'disk.mft')).toHaveLength(before)
    } finally {
      map.kill()
      map.dispose()
    }
  })

  it('refuses off Windows and outside the app frame without touching the core', async () => {
    const core = fakeCore(() => ({ ok: true, value: null }))
    const mac = make('darwin', { spawner: core.spawner })
    expect(await mac.ipc.call('ccm:diskmap-fastscan', { drive: 'C' })).toEqual({ started: false, error: 'unsupported' })
    mac.map.dispose()
    const foreign = make('win32', { spawner: core.spawner, frame: false })
    expect(await foreign.ipc.call('ccm:diskmap-fastscan', { drive: 'C' })).toEqual({ started: false, error: 'failed' })
    foreign.map.dispose()
    expect(core.seen).toEqual([])
  })
})

const haveDisk = process.platform === 'win32' && HAVE_DESK

describe.skipIf(!haveDisk)('diskMap fast scan with the real orrery-core and an MFT fixture', () => {
  it('runs the helper over the pipe, loads the index and saves it', async () => {
    const dataDir = mkdtempSync(path.join(os.tmpdir(), 'diskmap-mft-'))
    dirs.push(dataDir)
    const { ipc, map, broadcasts } = make('win32', { bin: DESK_BIN, env: { ...process.env, CCM_MFT_FIXTURE: FIXTURE }, dataDir })
    try {
      expect(await ipc.call('ccm:diskmap-fastscan', { drive: 'q' })).toEqual({ started: true })
      const deadline = Date.now() + 20_000
      while (!broadcasts.some(([c]) => c === DISKMAP_DONE) && Date.now() < deadline) await new Promise((r) => setTimeout(r, 25))
      expect(broadcasts.find(([c]) => c === DISKMAP_DONE)?.[1]).toEqual({ ok: true, entries: expect.any(Number), bytes: EXPECTED_BYTES, roots: ['Q:\\'], mft: true })
      expect(broadcasts.some(([c]) => c === DISKMAP_PROGRESS)).toBe(true)
      expect(await ipc.call('ccm:diskmap-status')).toMatchObject({ indexed: true, roots: ['Q:\\'] })
      const kids = (await ipc.call('ccm:diskmap-children', { path: 'Q:\\Users\\jin', sort: 'name', limit: 10 })) as { items: { name: string }[] }
      expect(kids.items.map((i) => i.name)).toEqual(['a.bin', 'Ảnh dài tên.jpg', 'link1.dat'])
      expect(((await ipc.call('ccm:diskmap-search', { query: 'big.iso' })) as { total: number }).total).toBe(1)
    } finally {
      await map.stop()
      map.dispose()
    }
  }, 30_000)
})
