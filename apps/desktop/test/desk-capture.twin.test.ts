import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { AwakeState } from '@ccm/shared'
import { CAPTURE_FILE } from '../src/capture/files'
import { CaptureFolder } from '../src/capture/folder'
import { registerCaptureIpc } from '../src/capture/ipc'
import { AWAKE_FILE, KeepAwake } from '../src/power/awake'
import { awakeControl } from '../src/power/ipc'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-capture-'))
const awakeLog = path.join(tmp, 'awake.log')
const PNG = `data:image/png;base64,${Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3]).toString('base64')}`

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const side = (s: string) => ({ data: path.join(tmp, s, 'data'), cap: path.join(tmp, s, 'cap') })
const N = side('node')
const R = side('rust')
const missing: Record<string, string[]> = { node: [], rust: [] }
const held: string[] = []
const nodeLog: string[] = []
const nodeAwake: AwakeState[] = []
const opened: string[] = []
let keepNode: KeepAwake
let keepRust: KeepAwake

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(swapRoot(v, R.cap, N.cap)).replace(/\d{8}-\d{6}/g, 'STAMP').replace(/"releaseAt":\d+/g, '"releaseAt":1'))

describe.skipIf(!HAVE_DESK)('capture + keep awake: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const s of [N, R]) mkdirSync(s.data, { recursive: true })
    twin = await startTwin({ CCM_DATA_DIR: R.data, CCM_E2E_AWAKE_LOG: awakeLog, CCM_E2E_POWER: 'ac', CCM_E2E_AWAKE_GRACE_MS: '300' })
    const gate = (e: unknown): boolean => e === 'app'
    const reg = (ipc: typeof node, s: typeof N, name: 'node' | 'rust', keep: KeepAwake, desk?: Twin['router']) => {
      const onMissing = (dir: string) => missing[name]!.push(dir)
      const folder = new CaptureFolder({ file: path.join(s.data, CAPTURE_FILE), tmpRoot: path.join(s.data, 'tmp'), videos: () => path.join(s.data, 'videos'), held: () => held, onMissing })
      registerCaptureIpc({
        ipcMain: ipc.ipcMain,
        isAppFrame: gate,
        folder,
        awake: keep,
        setAwakeMode: (mode) => {
          keep.setMode(mode)
          writeFileSync(path.join(s.data, AWAKE_FILE), JSON.stringify({ mode }))
        },
        pickFolder: async () => s.cap,
        openFolder: async (dir) => {
          opened.push(swapRoot(dir, R.cap, N.cap) as string)
          return true
        },
        missing: onMissing,
        held: () => held,
        desk,
      })
      return folder
    }
    keepNode = new KeepAwake({ start: () => (nodeLog.push('start'), 1), stop: () => nodeLog.push('stop'), onBattery: () => false, graceMs: 300, onChange: (s) => nodeAwake.push(s) }, 'plugged')
    keepRust = new KeepAwake({ start: () => 1, stop: () => undefined, onBattery: () => false }, 'plugged')
    reg(node, N, 'node', keepNode)
    reg(rust, R, 'rust', keepRust, twin.router)
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown, ...args: unknown[]) => {
    const a = norm(await node.call(channel, event, ...args))
    const b = norm(await rust.call(channel, event, ...args))
    expect(b).toEqual(a)
    return a
  }
  const files = (s: typeof N, sub: string) => (existsSync(path.join(s.cap, sub)) ? readdirSync(path.join(s.cap, sub)).map((f) => f.replace(/\d{8}-\d{6}/, 'STAMP')).sort() : [])

  it('folder settings, pick, shots and refusals match', async () => {
    await both('ccm:capture-get', 'app')
    await both('ccm:capture-shot', 'app', { png: PNG, device: 'x' })
    for (const patch of [null, 'x', { dir: '/x' }, { keepDays: 2 }, { maxGb: 3 }, { asFile: 'no' }, { asFile: false, keepDays: 14, maxGb: 5 }]) await both('ccm:capture-set', 'app', patch)
    await both('ccm:capture-get', 'guest')
    for (const s of [N, R]) mkdirSync(s.cap, { recursive: true })
    await both('ccm:capture-pick', 'app')
    expect(readFileSync(path.join(R.data, CAPTURE_FILE), 'utf8')).toBe(readFileSync(path.join(N.data, CAPTURE_FILE), 'utf8').split(JSON.stringify(N.cap).slice(1, -1)).join(JSON.stringify(R.cap).slice(1, -1)))
    for (const req of [{ png: PNG, device: '  Pixel 7 (API 34) ', frame: 2 }, { png: PNG, device: 'Pixel 7 (API 34)', frame: 2 }, { png: PNG, device: '...' }, { png: 'data:image/png;base64,AAAA', device: 'x' }, { png: PNG, device: 'x', frame: 0 }, { png: PNG, device: 'x', frame: 1.5 }, { png: PNG }]) {
      await both('ccm:capture-shot', 'app', req)
    }
    expect(files(R, 'shots')).toEqual(files(N, 'shots'))
    await both('ccm:capture-open', 'app')
    expect(opened).toEqual([N.cap, N.cap])
  })

  it('retention sweep on set keeps held and foreign files, missing folder is told once', async () => {
    const old = Date.now() / 1000 - 3 * 86_400
    for (const s of [N, R]) {
      const rec = path.join(s.cap, 'recordings')
      mkdirSync(rec, { recursive: true })
      for (const name of ['orrery-rec-1700000000001.mp4', 'orrery-rec-1700000000002.mp4', 'mine.mp4']) {
        writeFileSync(path.join(rec, name), 'x')
        utimesSync(path.join(rec, name), old, old)
      }
    }
    held.splice(0, held.length, path.join(N.cap, 'recordings', 'orrery-rec-1700000000002.mp4'), path.join(R.cap, 'recordings', 'orrery-rec-1700000000002.mp4'))
    await both('ccm:capture-set', 'app', { keepDays: 1 })
    await new Promise((r) => setTimeout(r, 200))
    expect(files(R, 'recordings')).toEqual(files(N, 'recordings'))
    expect(files(N, 'recordings')).toEqual(['mine.mp4', 'orrery-rec-1700000000002.mp4'])
    for (const s of [N, R]) rmSync(s.cap, { recursive: true, force: true })
    await both('ccm:capture-get', 'app')
    await both('ccm:capture-get', 'app')
    await both('ccm:capture-shot', 'app', { png: PNG, device: 'x' })
    await both('ccm:capture-open', 'app')
    expect(missing.rust.map((d) => swapRoot(d, R.cap, N.cap))).toEqual(missing.node)
    expect(missing.node).toHaveLength(1)
  })

  it('keep awake: mode, busy, grace release and power change match', async () => {
    const nodeCtl = awakeControl(keepNode)
    const rustCtl = awakeControl(keepRust, twin.router)
    await both('ccm:awake-state', 'app')
    for (const m of ['x', 'running']) await both('ccm:awake-mode', 'app', m)
    await both('ccm:awake-mode', 'guest', 'off')
    for (const [a, j] of [[2, 0], [2, 1], [0, 0]] as const) {
      await nodeCtl.setBusy(a, j)
      await rustCtl.setBusy(a, j)
      await both('ccm:awake-state', 'app')
    }
    await new Promise((r) => setTimeout(r, 800))
    await both('ccm:awake-state', 'app')
    await nodeCtl.powerChanged()
    await rustCtl.powerChanged()
    await both('ccm:awake-mode', 'app', 'off')
    await nodeCtl.setBusy(1, 0)
    await rustCtl.setBusy(1, 0)
    await both('ccm:awake-mode', 'app', 'plugged')
    await nodeCtl.dispose()
    await rustCtl.dispose()
    await new Promise((r) => setTimeout(r, 100))
    const rustAwake = twin.events.filter((e) => e.event === 'ccm:awake-event').map((e) => e.payload)
    expect(norm(rustAwake)).toEqual(norm(nodeAwake))
    expect(readFileSync(awakeLog, 'utf8').trim().split('\n')).toEqual(nodeLog)
    expect(readFileSync(path.join(R.data, AWAKE_FILE), 'utf8')).toBe(readFileSync(path.join(N.data, AWAKE_FILE), 'utf8'))
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
