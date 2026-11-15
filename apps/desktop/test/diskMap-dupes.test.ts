import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { PassThrough } from 'node:stream'
import type { IpcMainInvokeEvent } from 'electron'
import { afterEach, describe, expect, it } from 'vitest'
import { LineSplitter } from '../src/diskMap/host'
import { createDiskMap, DISKMAP_CHANNELS, DISKMAP_DUPES_DONE, DISKMAP_DUPES_PROGRESS } from '../src/diskMap/ipc'
import { cleanDupes, MAX_DUPE_FILES, MIN_DUPES_SIZE, toDupesDone, toDupesProgress } from '../src/diskMap/map'

class FakeChild extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  pid = 5
  exitCode: number | null = null
  kill(): boolean {
    this.exitCode = 0
    this.emit('exit', 0)
    return true
  }
}

type Reply = (m: { channel: string; args: Record<string, unknown> }) => { value?: unknown; error?: string }

function setup(reply: Reply) {
  const calls: { channel: string; args: Record<string, unknown> }[] = []
  const sent: { channel: string; payload: unknown }[] = []
  let child: FakeChild | null = null
  const spawner = () => {
    child = new FakeChild()
    const c = child
    const split = new LineSplitter()
    c.stdin.on('data', (chunk: Buffer) => {
      for (const l of split.push(String(chunk))) {
        if (!l.trim()) continue
        const m = JSON.parse(l) as { id: number; channel: string; args: Record<string, unknown> }
        calls.push({ channel: m.channel, args: m.args })
        const r = reply({ channel: m.channel, args: m.args })
        c.stdout.write(`${JSON.stringify(r.error ? { id: m.id, ok: false, error: r.error } : { id: m.id, ok: true, value: r.value ?? {} })}\n`)
      }
    })
    setImmediate(() => c.stdout.write(`${JSON.stringify({ ready: true, pid: 5, channels: [] })}\n`))
    return c as unknown as ChildProcessWithoutNullStreams
  }
  const handlers = new Map<string, (e: IpcMainInvokeEvent, a: unknown) => Promise<unknown>>()
  const ipcMain = { handle: (c: string, fn: (e: IpcMainInvokeEvent, a: unknown) => Promise<unknown>) => handlers.set(c, fn), removeHandler: (c: string) => handlers.delete(c) }
  const ev = { sender: { id: 1 } } as unknown as IpcMainInvokeEvent
  const dataDir = mkdtempSync(path.join(os.tmpdir(), 'diskmap-dupes-'))
  dirs.push(dataDir)
  const dm = createDiskMap({
    ipcMain,
    isAppFrame: (e) => (e as unknown as { sender: { id: number } }).sender.id === 1,
    bin: 'fake',
    env: {},
    dataDir,
    platform: 'win32',
    home: 'C:/Users/dev',
    keep: () => [],
    broadcast: (channel, payload) => sent.push({ channel, payload }),
    confirm: async () => true,
    trash: async () => undefined,
    remove: async () => undefined,
    reveal: () => undefined,
    spawner,
    exists: () => true,
  })
  const emit = (event: string, payload: unknown) => child!.stdout.write(`${JSON.stringify({ event, payload })}\n`)
  return { call: (ch: string, a?: unknown, e = ev) => handlers.get(ch)!(e, a), calls, sent, emit, dm, tick: () => new Promise((r) => setTimeout(r, 20)) }
}

const dirs: string[] = []
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

const base: Reply = (m) => (m.channel === 'disk.status' ? { value: { scanning: false, paused: false, index: { roots: ['D:\\'], entries: 1 } } } : { value: { loaded: true } })

describe('cleanDupes', () => {
  it('accepts an empty request and clamps min size', () => {
    expect(cleanDupes(undefined, 'win32')).toEqual({})
    expect(cleanDupes({ minSize: 5 }, 'win32')).toEqual({ minSize: MIN_DUPES_SIZE })
    expect(cleanDupes({ minSize: 5_000_000.7, level: 'light' }, 'win32')).toEqual({ minSize: 5_000_000, level: 'light' })
  })

  it('validates folders, excludes and numbers', () => {
    expect(cleanDupes({ under: ['C:/a', 'C:/a', 'D:/b'] }, 'win32')).toEqual({ under: ['C:/a', 'D:/b'] })
    expect(cleanDupes({ under: [] }, 'win32')).toEqual({})
    expect(cleanDupes({ under: ['rel'] }, 'win32')).toBeNull()
    expect(cleanDupes({ under: 'C:/a' }, 'win32')).toBeNull()
    expect(cleanDupes({ minSize: -1 }, 'win32')).toBeNull()
    expect(cleanDupes({ minSize: 'x' }, 'win32')).toBeNull()
    expect(cleanDupes({ excludes: ['', 'a'] }, 'win32')).toBeNull()
    expect(cleanDupes({ excludes: ['node_modules'] }, 'win32')).toEqual({ excludes: ['node_modules'] })
    expect(cleanDupes({ level: 'turbo' }, 'win32')).toEqual({})
  })
})

describe('toDupesProgress / toDupesDone', () => {
  it('maps progress and falls back for unknown stages', () => {
    expect(toDupesProgress({ stage: 'full', files: 4, filesDone: 2, bytes: 100, bytesRead: 50, current: 'x', elapsedMs: 9, paused: true, groups: 1, candidates: 7, skipped: 2 })).toEqual({ stage: 'full', candidates: 7, files: 4, filesDone: 2, bytes: 100, bytesRead: 50, groups: 1, skipped: 2, current: 'x', elapsedMs: 9, paused: true })
    expect(toDupesProgress({ stage: 'nope' }).stage).toBe('idle')
    expect(toDupesProgress(null).files).toBe(0)
  })

  it('drops malformed groups and caps the files', () => {
    const done = toDupesDone({
      ok: true,
      wasted: 30,
      totalGroups: 3,
      groups: [
        { size: 10, hash: 'a', count: 3, files: [{ path: 'C:/a', mtime: 1 }, { path: 'C:/b', mtime: 2 }, { nope: 1 }] },
        { size: 5, hash: 'b', count: 1, files: [{ path: 'C:/c', mtime: 1 }] },
        { size: 5, hash: 'c' },
        null,
      ],
      skipped: { stale: 1, locked: 2 },
    })
    expect(done.ok).toBe(true)
    expect(done.groups).toHaveLength(1)
    expect(done.groups[0]).toEqual({ size: 10, hash: 'a', count: 3, files: [{ path: 'C:/a', mtime: 1 }, { path: 'C:/b', mtime: 2 }] })
    expect(done.skipped).toEqual({ stale: 1, cloud: 0, locked: 2, changed: 0, gone: 0 })
    const many = toDupesDone({ ok: true, groups: Array.from({ length: 10 }, () => ({ size: 1, hash: 'h', count: MAX_DUPE_FILES, files: Array.from({ length: MAX_DUPE_FILES / 2 }, (_, i) => ({ path: `C:/f${i}`, mtime: 0 })) })) })
    expect(many.groups.reduce((n, g) => n + g.files.length, 0)).toBeLessThanOrEqual(MAX_DUPE_FILES)
    expect(toDupesDone({ ok: false, cancelled: true }).cancelled).toBe(true)
    expect(toDupesDone({ ok: false, error: 'x'.repeat(500) }).error).toHaveLength(300)
  })
})

describe('ccm:diskmap-dupes', () => {
  it('registers both channels', () => {
    expect(DISKMAP_CHANNELS).toContain('ccm:diskmap-dupes')
    expect(DISKMAP_CHANNELS).toContain('ccm:diskmap-dupes-cancel')
  })

  it('searches under home by default and passes the filters', async () => {
    const s = setup((m) => (m.channel === 'disk.dupes' ? { value: { started: true } } : base(m)))
    expect(await s.call('ccm:diskmap-dupes', { minSize: 4_000_000, excludes: ['x'], level: 'light' })).toEqual({ started: true })
    const c = s.calls.find((x) => x.channel === 'disk.dupes')!
    expect(c.args).toEqual({ minSize: 4_000_000, excludes: ['x'], level: 'light', under: ['C:/Users/dev'] })
  })

  it('falls back to the indexed roots when home is not indexed', async () => {
    const s = setup((m) => {
      if (m.channel === 'disk.dupes') return (m.args.under as string[])[0] === 'C:/Users/dev' ? { error: 'not in index: C:/Users/dev' } : { value: { started: true } }
      return base(m)
    })
    expect(await s.call('ccm:diskmap-dupes', {})).toEqual({ started: true })
    const tries = s.calls.filter((x) => x.channel === 'disk.dupes').map((x) => x.args.under)
    expect(tries).toEqual([['C:/Users/dev'], ['D:\\']])
  })

  it('maps errors to short codes', async () => {
    const s = setup((m) => (m.channel === 'disk.dupes' ? { error: 'busy' } : base(m)))
    expect(await s.call('ccm:diskmap-dupes', {})).toEqual({ started: false, error: 'busy' })
    expect(await s.call('ccm:diskmap-dupes', { under: ['rel'] })).toEqual({ started: false, error: 'invalid' })
    const t = setup((m) => (m.channel === 'disk.dupes' ? { error: 'not in index: X' } : { value: { scanning: false, paused: false, index: { roots: [] } } }))
    expect(await t.call('ccm:diskmap-dupes', {})).toEqual({ started: false, error: 'not-indexed' })
    const stranger = { sender: { id: 2 } } as unknown as IpcMainInvokeEvent
    expect(await s.call('ccm:diskmap-dupes', {}, stranger)).toEqual({ started: false, error: 'failed' })
  })

  it('forwards progress and the done payload, and cancel reaches the core', async () => {
    const s = setup((m) => (m.channel === 'disk.cancel' ? { value: { ok: true } } : m.channel === 'disk.dupes' ? { value: { started: true } } : base(m)))
    await s.call('ccm:diskmap-dupes', {})
    s.emit('disk:dupes-progress', { stage: 'quick', files: 3, filesDone: 1 })
    s.emit('disk:dupes-done', { ok: true, wasted: 7, totalGroups: 0, groups: [] })
    await s.tick()
    expect(s.sent.find((x) => x.channel === DISKMAP_DUPES_PROGRESS)?.payload).toMatchObject({ stage: 'quick', files: 3, filesDone: 1 })
    expect(s.sent.find((x) => x.channel === DISKMAP_DUPES_DONE)?.payload).toMatchObject({ ok: true, wasted: 7, groups: [] })
    expect(await s.call('ccm:diskmap-dupes-cancel')).toBe(true)
    expect(s.calls.some((x) => x.channel === 'disk.cancel')).toBe(true)
  })
})
