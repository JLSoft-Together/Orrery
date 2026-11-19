import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { PassThrough } from 'node:stream'
import type { IpcMainInvokeEvent } from 'electron'
import { afterEach, describe, expect, it } from 'vitest'
import { LineSplitter } from '../src/diskMap/host'
import { createDiskMap, DISKMAP_CHANNELS } from '../src/diskMap/ipc'
import { cleanSizes, MAX_SIZES, toSizes } from '../src/diskMap/map'

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

function setup(reply: (m: Record<string, unknown>) => unknown, dataDir: string) {
  const calls: string[] = []
  let spawned = 0
  const spawner = () => {
    spawned++
    const child = new FakeChild()
    const split = new LineSplitter()
    child.stdin.on('data', (c: Buffer) => {
      for (const l of split.push(String(c))) {
        if (!l.trim()) continue
        const m = JSON.parse(l) as Record<string, unknown>
        calls.push(String(m.channel))
        child.stdout.write(`${JSON.stringify({ id: m.id, ok: true, value: reply(m) })}\n`)
      }
    })
    setImmediate(() => child.stdout.write(`${JSON.stringify({ ready: true, pid: 5, channels: [] })}\n`))
    return child as unknown as ChildProcessWithoutNullStreams
  }
  const handlers = new Map<string, (e: IpcMainInvokeEvent, a: unknown) => Promise<unknown>>()
  const ipcMain = { handle: (c: string, fn: (e: IpcMainInvokeEvent, a: unknown) => Promise<unknown>) => handlers.set(c, fn), removeHandler: (c: string) => handlers.delete(c) }
  const ev = { sender: { id: 1 } } as unknown as IpcMainInvokeEvent
  createDiskMap({
    ipcMain,
    isAppFrame: (e) => (e as unknown as { sender: { id: number } }).sender.id === 1,
    bin: 'fake',
    env: {},
    dataDir,
    platform: 'win32',
    home: 'C:/Users/dev',
    keep: () => [],
    broadcast: () => undefined,
    confirm: async () => true,
    trash: async () => undefined,
    remove: async () => undefined,
    reveal: () => undefined,
    spawner,
    exists: () => true,
  })
  return { call: (a?: unknown, e = ev) => handlers.get('ccm:diskmap-sizes')!(e, a), calls, spawned: () => spawned }
}

const dirs: string[] = []
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'diskmap-sizes-'))
  dirs.push(d)
  return d
}

describe('cleanSizes / toSizes', () => {
  it('accepts absolute paths only, dedupes and caps the batch', () => {
    expect(cleanSizes({ paths: ['C:/a', 'C:/a', 'D:/b'] }, 'win32')).toEqual(['C:/a', 'D:/b'])
    expect(cleanSizes({ paths: ['rel/a'] }, 'win32')).toBeNull()
    expect(cleanSizes({ paths: [] }, 'win32')).toBeNull()
    expect(cleanSizes({ paths: 'C:/a' }, 'win32')).toBeNull()
    expect(cleanSizes(null, 'win32')).toBeNull()
    expect(cleanSizes({ paths: Array.from({ length: MAX_SIZES + 1 }, (_, i) => `C:/d${i}`) }, 'win32')).toBeNull()
    expect(cleanSizes({ paths: Array.from({ length: MAX_SIZES }, (_, i) => `C:/d${i}`) }, 'win32')).toHaveLength(MAX_SIZES)
  })

  it('maps the core answer and rejects a malformed one', () => {
    expect(toSizes({ version: '1:2', sizes: { 'C:/a': 5, 'C:/b': null, 'C:/c': -1, 'C:/d': 'x' } })).toEqual({ version: '1:2', sizes: { 'C:/a': 5, 'C:/b': null, 'C:/c': null, 'C:/d': null } })
    expect(toSizes({ sizes: {} })).toBeNull()
    expect(toSizes({ version: 'v', sizes: [] })).toBeNull()
    expect(toSizes(null)).toBeNull()
  })
})

describe('ccm:diskmap-sizes', () => {
  it('is a registered channel', () => {
    expect(DISKMAP_CHANNELS).toContain('ccm:diskmap-sizes')
  })

  it('does not start the core when there is no saved index and nothing is running', async () => {
    const dataDir = tmp()
    const s = setup(() => ({}), dataDir)
    expect(await s.call({ paths: ['C:/a'] })).toBeNull()
    expect(s.spawned()).toBe(0)
  })

  it('refuses a stranger frame and a bad request without starting the core', async () => {
    const dataDir = tmp()
    mkdirSync(path.join(dataDir, 'disk'))
    writeFileSync(path.join(dataDir, 'disk', 'index.bin'), 'x')
    const s = setup(() => ({}), dataDir)
    expect(await s.call({ paths: ['C:/a'] }, { sender: { id: 2 } } as unknown as IpcMainInvokeEvent)).toBeNull()
    expect(await s.call({ paths: ['rel'] })).toBeNull()
    expect(s.spawned()).toBe(0)
  })

  it('loads the saved index once and asks the core for the batch', async () => {
    const dataDir = tmp()
    mkdirSync(path.join(dataDir, 'disk'))
    writeFileSync(path.join(dataDir, 'disk', 'index.bin'), 'x')
    const s = setup((m) => {
      if (m.channel === 'disk.status') return { scanning: false, paused: false }
      if (m.channel === 'disk.sizes') return { version: '9:3', sizes: { 'C:/a': 10, 'C:/b': null } }
      return { loaded: true }
    }, dataDir)
    expect(await s.call({ paths: ['C:/a', 'C:/b'] })).toEqual({ version: '9:3', sizes: { 'C:/a': 10, 'C:/b': null } })
    await s.call({ paths: ['C:/a'] })
    expect(s.calls.filter((c) => c === 'disk.load')).toHaveLength(1)
    expect(s.calls.filter((c) => c === 'disk.sizes')).toHaveLength(2)
  })
})
