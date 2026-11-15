import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { rm } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { PassThrough } from 'node:stream'
import type { IpcMainInvokeEvent } from 'electron'
import { afterEach, describe, expect, it } from 'vitest'
import { DiskMapHost, LineSplitter, parseCoreLine } from '../src/diskMap/host'
import { createDiskMap, DISKMAP_CHANNELS, DISKMAP_DONE, DISKMAP_PROGRESS } from '../src/diskMap/ipc'
import { toChildren, toDone, toEntry, toStatus } from '../src/diskMap/map'
import { DESK_BIN, HAVE_DESK } from './desk-twin'

type Script = (msg: Record<string, unknown>, reply: (m: object) => void, child: FakeChild) => void

class FakeChild extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  pid = 777
  exitCode: number | null = null
  kill(): boolean {
    this.exit(null)
    return true
  }
  exit(code: number | null): void {
    if (this.exitCode !== null) return
    this.exitCode = code ?? -1
    this.emit('exit', code)
  }
}

function fake(script: Script) {
  const children: FakeChild[] = []
  const args: string[][] = []
  const spawner = (_bin: string, a: string[]) => {
    args.push(a)
    const child = new FakeChild()
    children.push(child)
    const reply = (m: object) => child.stdout.write(`${JSON.stringify(m)}\n`)
    const split = new LineSplitter()
    child.stdin.on('data', (c: Buffer) => {
      for (const l of split.push(String(c))) if (l.trim()) script(JSON.parse(l) as Record<string, unknown>, reply, child)
    })
    setImmediate(() => reply({ ready: true, pid: 777, channels: ['disk.status'] }))
    return child as unknown as ChildProcessWithoutNullStreams
  }
  return { spawner, children, args }
}

describe('NDJSON parsing', () => {
  it('parses ready, events, replies and junk', () => {
    expect(parseCoreLine('{"ready":true,"pid":5,"channels":["a",1]}\r')).toEqual({ kind: 'ready', pid: 5, channels: ['a'] })
    expect(parseCoreLine('{"event":"disk:progress","payload":{"files":3}}')).toEqual({ kind: 'event', event: 'disk:progress', payload: { files: 3 } })
    expect(parseCoreLine('{"id":7,"ok":true,"value":{"x":1}}')).toEqual({ kind: 'reply', id: 7, ok: true, value: { x: 1 }, error: 'failed' })
    expect(parseCoreLine('{"id":8,"ok":false,"error":"busy"}')).toEqual({ kind: 'reply', id: 8, ok: false, value: null, error: 'busy' })
    expect(parseCoreLine('{"id":1.5}')).toEqual({ kind: 'text', text: '{"id":1.5}' })
    expect(parseCoreLine('thread panicked')).toEqual({ kind: 'text', text: 'thread panicked' })
    expect(parseCoreLine('  ')).toBeNull()
  })

  it('splits chunks on newlines and keeps partial lines', () => {
    const s = new LineSplitter()
    expect(s.push('{"a":1}\n{"b"')).toEqual(['{"a":1}'])
    expect(s.push(':2}\n\n')).toEqual(['{"b":2}', ''])
    expect(s.push('tail')).toEqual([])
  })

  it('maps core shapes to the bridge types', () => {
    expect(toEntry({ path: 'C:\\a\\b.mp4', name: 'b.mp4', dir: false, size: 10, len: 9, mtime: 1000, kind: 'video', cloud: true, count: 0 })).toEqual({ path: 'C:\\a\\b.mp4', name: 'b.mp4', size: 10, dir: false, mtime: 1000, cloud: true, kind: 'video' })
    expect(toEntry({ path: 'C:\\a', name: 'a', dir: true, size: 1, mtime: 0, kind: 'other' })).toEqual({ path: 'C:\\a', name: 'a', size: 1, dir: true, mtime: 0 })
    expect(toEntry({ name: 'x' })).toBeNull()
    expect(toChildren({ path: null, size: 5, total: 1, items: [{ path: 'C:\\', name: 'C:\\', dir: true, size: 5, mtime: 0 }, 3] })).toEqual({ path: '', size: 5, total: 1, items: [{ path: 'C:\\', name: 'C:\\', size: 5, dir: true, mtime: 0 }] })
    expect(toStatus({ scanning: true, paused: false })).toEqual({ scanning: true, paused: false, indexed: false, roots: [] })
    expect(toStatus({ scanning: false, paused: false, index: { roots: ['D:\\w'], entries: 9, scannedAt: 77 } })).toEqual({ scanning: false, paused: false, indexed: true, roots: ['D:\\w'], entries: 9, scannedAt: 77 })
    expect(toDone({ ok: false, cancelled: true })).toEqual({ ok: false, cancelled: true })
  })
})

describe('DiskMapHost', () => {
  it('starts lazily with the disk subcommand and matches replies by id', async () => {
    const f = fake((m, reply) => {
      const delay = m.channel === 'slow' ? 20 : 0
      setTimeout(() => reply({ id: m.id, ok: true, value: m.channel }), delay)
    })
    const host = new DiskMapHost({ bin: 'fake', env: {}, exists: () => true, spawner: f.spawner })
    expect(host.running).toBe(false)
    expect(f.children).toHaveLength(0)
    const [a, b] = await Promise.all([host.call('slow'), host.call('fast')])
    expect([a, b]).toEqual(['slow', 'fast'])
    expect(f.args).toEqual([['disk']])
    expect(host.running).toBe(true)
  })

  it('rejects errors, forwards events and rejects pending calls when the core dies', async () => {
    const events: string[] = []
    const exits: boolean[] = []
    const f = fake((m, reply, child) => {
      if (m.channel === 'disk.scan') {
        reply({ event: 'disk:progress', payload: { files: 1 } })
        reply({ id: m.id, ok: true, value: { started: true } })
      } else if (m.channel === 'bad') reply({ id: m.id, ok: false, error: 'not in index: x' })
      else if (m.channel === 'die') setTimeout(() => child.exit(3), 5)
      else reply({ id: m.id, ok: true, value: 1 })
    })
    const host = new DiskMapHost({ bin: 'fake', env: {}, exists: () => true, spawner: f.spawner, onEvent: (e) => events.push(e), onExit: (_c, busy) => exits.push(busy) })
    await host.call('disk.scan')
    host.markScanning(true)
    expect(events).toEqual(['disk:progress'])
    await expect(host.call('bad')).rejects.toThrow('not in index: x')
    await expect(host.call('die')).rejects.toThrow('exited (code 3)')
    expect(exits).toEqual([true])
    expect(host.running).toBe(false)
    expect(host.lastError).toContain('code 3')
    expect(await host.call('again')).toBe(1)
    expect(f.children).toHaveLength(2)
  })

  it('fails fast when the binary is missing and times out silent calls', async () => {
    const missing = new DiskMapHost({ bin: 'nope', env: {}, exists: () => false })
    await expect(missing.call('disk.status')).rejects.toThrow('orrery-core not found')
    const f = fake(() => undefined)
    const host = new DiskMapHost({ bin: 'fake', env: {}, exists: () => true, spawner: f.spawner, callTimeoutMs: 30 })
    await expect(host.call('hang')).rejects.toThrow('timeout hang')
  })

  it('stops with a shutdown line and refuses calls after', async () => {
    const seen: unknown[] = []
    const f = fake((m, reply, child) => {
      seen.push(m)
      if (m.type === 'shutdown') child.exit(0)
      else reply({ id: m.id, ok: true, value: null })
    })
    const host = new DiskMapHost({ bin: 'fake', env: {}, exists: () => true, spawner: f.spawner })
    await host.call('disk.status')
    await host.stop()
    expect(seen).toContainEqual({ type: 'shutdown' })
    expect(host.running).toBe(false)
    await expect(host.call('disk.status')).rejects.toThrow('stopping')
  })
})

function fakeIpc() {
  const handlers = new Map<string, (event: IpcMainInvokeEvent, arg: unknown) => Promise<unknown>>()
  const ipcMain = {
    handle: (c: string, fn: (event: IpcMainInvokeEvent, arg: unknown) => Promise<unknown>) => handlers.set(c, fn),
    removeHandler: (c: string) => handlers.delete(c),
  }
  const ok = { sender: { id: 1 } } as unknown as IpcMainInvokeEvent
  const call = (c: string, arg?: unknown, event = ok) => handlers.get(c)!(event, arg)
  return { ipcMain, handlers, call }
}

const dirs: string[] = []
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

describe('diskMap ipc (fake core)', () => {
  it('gates on the app frame, loads the saved index once and refuses a second delete while one runs', async () => {
    const calls: string[] = []
    let release: () => void = () => undefined
    const f = fake((m, reply) => {
      calls.push(String(m.channel))
      const value = m.channel === 'disk.status' ? { scanning: false, paused: false, index: { roots: ['D:\\work'], entries: 3 } } : m.channel === 'disk.children' ? { path: m.args, size: 100, total: 0, items: [] } : { ok: true }
      reply({ id: m.id, ok: true, value })
    })
    const ipc = fakeIpc()
    const dataDir = mkdtempSync(path.join(os.tmpdir(), 'diskmap-ipc-'))
    dirs.push(dataDir)
    const broadcasts: [string, unknown][] = []
    const map = createDiskMap({
      ipcMain: ipc.ipcMain,
      isAppFrame: (e) => (e as unknown as { sender: { id: number } }).sender.id === 1,
      bin: 'fake',
      env: {},
      dataDir,
      platform: 'win32',
      home: 'C:\\Users\\dev',
      keep: () => [],
      broadcast: (c, p) => broadcasts.push([c, p]),
      confirm: () => new Promise<boolean>((r) => (release = () => r(true))),
      trash: async () => undefined,
      remove: async () => undefined,
      reveal: () => undefined,
      spawner: f.spawner,
      exists: () => true,
    })
    expect([...ipc.handlers.keys()].sort()).toEqual([...DISKMAP_CHANNELS].sort())
    const stranger = { sender: { id: 2 } } as unknown as IpcMainInvokeEvent
    expect(await ipc.call('ccm:diskmap-status', undefined, stranger)).toEqual({ scanning: false, paused: false, indexed: false, roots: [] })
    expect(f.children).toHaveLength(0)
    expect(await ipc.call('ccm:diskmap-pause')).toBe(false)
    expect(f.children).toHaveLength(0)
    expect(await ipc.call('ccm:diskmap-status')).toEqual({ scanning: false, paused: false, indexed: true, roots: ['D:\\work'], entries: 3 })
    await ipc.call('ccm:diskmap-status')
    expect(calls.filter((c) => c === 'disk.load')).toHaveLength(0)
    expect(await ipc.call('ccm:diskmap-scan', { roots: ['rel'] })).toEqual({ started: false })
    expect(await ipc.call('ccm:diskmap-children', { path: 'rel' })).toBeNull()
    const first = ipc.call('ccm:diskmap-delete', { paths: ['D:\\work\\a'], permanent: false })
    await new Promise((r) => setTimeout(r, 30))
    const second = await ipc.call('ccm:diskmap-delete', { paths: ['D:\\work\\a'], permanent: false })
    expect(second).toEqual({ done: [], failed: [{ path: 'D:\\work\\a', reason: 'busy' }], freed: 0, permanent: false })
    release()
    expect(await first).toEqual({ done: [{ path: 'D:\\work\\a', size: 100 }], failed: [], freed: 100, permanent: false })
    expect(calls).toContain('disk.forget')
    expect(await ipc.call('ccm:diskmap-history', { limit: 5 })).toEqual([expect.objectContaining({ action: 'trash', count: 1, freed: 100, paths: ['D:\\work\\a'] })])
    expect(await ipc.call('ccm:diskmap-delete', { paths: [] })).toEqual({ done: [], failed: [], freed: 0, permanent: false, cancelled: true })
    f.children[0]!.exit(1)
    expect(broadcasts.filter(([c]) => c === DISKMAP_DONE)).toEqual([])
    map.dispose()
    expect(ipc.handlers.size).toBe(0)
  })

  it('reports a scan that dies with the core as done with an error', async () => {
    const f = fake((m, reply, child) => {
      if (m.channel === 'disk.scan') {
        reply({ id: m.id, ok: true, value: { started: true } })
        setTimeout(() => child.exit(101), 10)
      } else reply({ id: m.id, ok: true, value: m.channel === 'disk.status' ? { scanning: false, paused: false } : { loaded: false } })
    })
    const ipc = fakeIpc()
    const broadcasts: [string, unknown][] = []
    const map = createDiskMap({ ipcMain: ipc.ipcMain, isAppFrame: () => true, bin: 'fake', env: {}, dataDir: os.tmpdir(), platform: 'win32', home: '', keep: () => [], broadcast: (c, p) => broadcasts.push([c, p]), confirm: async () => true, trash: async () => undefined, remove: async () => undefined, reveal: () => undefined, spawner: f.spawner, exists: () => true })
    expect(await ipc.call('ccm:diskmap-scan', { roots: ['D:\\'] })).toEqual({ started: true })
    await new Promise((r) => setTimeout(r, 40))
    expect(broadcasts).toEqual([[DISKMAP_DONE, { ok: false, error: 'orrery-core disk exited (code 101)' }]])
    expect(await ipc.call('ccm:diskmap-status')).toEqual({ scanning: false, paused: false, indexed: false, roots: [] })
    expect(f.children).toHaveLength(2)
    map.dispose()
  })
})

const haveDisk = HAVE_DESK && existsSync(DESK_BIN)

describe.skipIf(!haveDisk)('diskMap with the real orrery-core', () => {
  it('scans, lists, deletes a part that is locked out and keeps the index in step', async () => {
    const root = mkdtempSync(path.join(os.tmpdir(), 'diskmap-real-'))
    const dataDir = mkdtempSync(path.join(os.tmpdir(), 'diskmap-data-'))
    dirs.push(root, dataDir)
    const w = (rel: string, n: number) => {
      mkdirSync(path.dirname(path.join(root, rel)), { recursive: true })
      writeFileSync(path.join(root, rel), Buffer.alloc(n, 1))
    }
    w('keep/a.bin', 10_000)
    w('Ảnh cũ 🎉/b.bin', 50_000)
    w('Ảnh cũ 🎉/c.bin', 20_000)
    w('junk/d.bin', 30_000)
    const ipc = fakeIpc()
    const broadcasts: [string, unknown][] = []
    const map = createDiskMap({
      ipcMain: ipc.ipcMain,
      isAppFrame: () => true,
      bin: DESK_BIN,
      env: process.env,
      dataDir,
      platform: process.platform,
      home: os.homedir(),
      keep: () => [],
      broadcast: (c, p) => broadcasts.push([c, p]),
      confirm: async () => true,
      autoConfirm: true,
      trash: (p) => (p.endsWith('junk') ? Promise.reject(Object.assign(new Error('locked'), { code: 'EBUSY' })) : rm(p, { recursive: true })),
      remove: (p) => rm(p, { recursive: true }),
      reveal: () => undefined,
    })
    try {
      expect(await ipc.call('ccm:diskmap-status')).toMatchObject({ indexed: false, scanning: false })
      expect(await ipc.call('ccm:diskmap-scan', { roots: [root], level: 'light' })).toEqual({ started: true })
      const deadline = Date.now() + 20_000
      while (!broadcasts.some(([c]) => c === DISKMAP_DONE) && Date.now() < deadline) await new Promise((r) => setTimeout(r, 25))
      expect(broadcasts.find(([c]) => c === DISKMAP_DONE)?.[1]).toMatchObject({ ok: true })
      expect(broadcasts.every(([c]) => c === DISKMAP_DONE || c === DISKMAP_PROGRESS)).toBe(true)
      const before = (await ipc.call('ccm:diskmap-children', { path: root, sort: 'size', limit: 10 })) as { size: number; items: { name: string; size: number }[] }
      expect(before.items.map((i) => i.name)).toEqual(['Ảnh cũ 🎉', 'junk', 'keep'])
      const photos = before.items[0]!.size
      const r = (await ipc.call('ccm:diskmap-delete', { paths: [path.join(root, 'Ảnh cũ 🎉'), path.join(root, 'junk')], permanent: false })) as { done: { path: string; size: number }[]; failed: { reason: string }[]; freed: number }
      expect(r.done).toEqual([{ path: path.join(root, 'Ảnh cũ 🎉'), size: photos }])
      expect(r.failed).toEqual([{ path: path.join(root, 'junk'), reason: 'EBUSY' }])
      const after = (await ipc.call('ccm:diskmap-children', { path: root })) as { size: number; items: { name: string }[] }
      expect(after.size).toBe(before.size - photos)
      expect(after.items.map((i) => i.name)).toEqual(['junk', 'keep'])
      expect((await ipc.call('ccm:diskmap-search', { query: 'b.bin' })) as { total: number }).toMatchObject({ total: 0 })
      expect(await ipc.call('ccm:diskmap-history', {})).toEqual([expect.objectContaining({ action: 'trash', count: 1, freed: photos, failed: 1 })])
      const outside = (await ipc.call('ccm:diskmap-delete', { paths: [path.join(os.tmpdir(), 'elsewhere')], permanent: true })) as { failed: { reason: string }[] }
      expect(outside.failed[0]!.reason).toBe('outside')
      expect(map.host.running).toBe(true)
    } finally {
      await map.stop()
      map.dispose()
    }
    expect(map.host.running).toBe(false)
  }, 30_000)
})
