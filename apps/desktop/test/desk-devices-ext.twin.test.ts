import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import initSqlJs from 'sql.js'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { AppDataInspector } from '../src/devices/appData'
import { registerAppDataIpc } from '../src/devices/appDataIpc'
import { DeviceHistory } from '../src/devices/history'
import { registerDevicesIpc } from '../src/devices/ipc'
import { DeviceManager } from '../src/devices/manager'
import { PerfService } from '../src/devices/perf'
import { registerPerfIpc } from '../src/devices/perfIpc'
import { ScreenRecorder } from '../src/devices/recorder'
import { DeviceFiles } from '../src/devices/fileBrowser'
import { registerFilesIpc } from '../src/devices/filesIpc'
import { defaultSpawner, killAdbClient } from '../src/run/proc'
import { fakeIpc, HAVE_DESK, listTree, startTwin, type Twin } from './desk-twin'

const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-devext-')))
const sdk = path.join(tmp, 'sdk')
const state = path.join(sdk, 'state')
const dataNode = path.join(tmp, 'data-node')
const dataRust = path.join(tmp, 'data-rust')
const recNode = path.join(tmp, 'rec-node')
const recRust = path.join(tmp, 'rec-rust')
const outDir = path.join(tmp, 'out')
const pullNode = path.join(tmp, 'pull-node')
const pullRust = path.join(tmp, 'pull-rust')
const pushSrc = path.join(tmp, 'push', 'upload.txt')
const MP4 = Buffer.concat([Buffer.from([0, 0, 0, 16]), Buffer.from('ftypisom'), Buffer.alloc(4), Buffer.from([0, 0, 0, 8]), Buffer.from('moov')])
const PNG = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 4, 0, 0, 0, 6]), Buffer.alloc(12)])
const PREFS = "<?xml version='1.0' encoding='utf-8' standalone='yes' ?>\n<map>\n    <string name=\"token\">a &amp; b</string>\n    <int name=\"launches\" value=\"3\" />\n    <set name=\"tags\"><string>x</string></set>\n</map>\n"
const GFX = '** Graphics info for pid 99 [com.shop] **\nTotal frames rendered: 200\nJanky frames: 20 (10.00%)\n50th percentile: 8ms\n90th percentile: 16ms\n95th percentile: 20ms\n99th percentile: 40ms\n'
const MEM = '** MEMINFO in pid 99 [com.shop] **\n Java Heap: 1000\n Native Heap: 2000\n Graphics: 300\n TOTAL PSS: 9000\n TOTAL RSS: 12000\n'

const ADB = `#!/bin/sh
export LC_ALL=C
D="$(dirname "$0")/../state"
echo "$@" >> "$D/log.txt"
map() { sed "s#/sdcard#$D/sdcard#g; s#/data/local/tmp#$D/tmp#g; s#run-as#run_as#g"; }
run_as() {
  pkg="$(printf '%s' "$1" | tr -d "'")"; shift
  case "$pkg" in com.nodebug) echo "run-as: package not debuggable: $pkg"; return 1 ;; esac
  [ -d "$D/app/$pkg" ] || { echo "run-as: unknown package: $pkg"; return 1; }
  (cd "$D/app/$pkg" && "$@")
}
case "$1" in
devices) cat "$D/devices.txt" ;;
track-devices) body="$(cat "$D/devices.txt")"; printf '%04x%s' \${#body} "$body"; exec sleep 30 ;;
-s) id="$2"; shift 2
  case "$1" in
  pull) src="$(printf '%s' "$2" | map)"; if [ -d "$src" ]; then cp -R "$src" "$3"; else cp "$src" "$3" 2>/dev/null || { echo "adb: error: failed to stat remote object '$2': No such file or directory" >&2; exit 1; }; fi ;;
  exec-out) shift
    case "$1" in
    screencap) cat "$D/shot.png" ;;
    cat) case "$2" in *slow*) i=0; while [ $i -lt 100 ]; do printf '%01000d' 0; sleep 0.1; i=$((i+1)); done ;; *) cmd="$(printf '%s ' "$@" | map)"; eval "$cmd" ;; esac ;;
    *) cmd="$(printf '%s ' "$@" | map)"; eval "$cmd" ;;
    esac ;;
  shell) shift
    case "$1" in
    screenrecord) for a in "$@"; do p="$a"; done; n="$(basename "$p" .mp4)"; printf '%s' "$p" | map > "$D/$n.path"; echo $$ > "$D/$n.pid"; exec sleep 100 ;;
    pkill) n="$(printf '%s' "$4" | tr -d "'")"; kill -INT "$(cat "$D/$n.pid")" 2>/dev/null; cp "$D/good.mp4" "$(cat "$D/$n.path")" ;;
    dumpsys) case "$3" in *com.gone*) echo "No process found for: com.gone" ;; *) case "$2" in gfxinfo) case "$4" in reset) echo ok ;; *) cat "$D/gfx.txt" ;; esac ;; meminfo) cat "$D/mem.txt" ;; esac ;; esac ;;
    cmd) printf 'priority=0 preferredOrder=0\\ncom.shop/.Main\\n' ;;
    am) case "$2" in start) printf 'Starting: Intent\\nStatus: ok\\nLaunchState: COLD\\nTotalTime: 420\\nWaitTime: 433\\nComplete\\n' ;; esac ;;
    *) cmd="$(printf '%s ' "$@" | map)"; eval "$cmd" ;;
    esac ;;
  esac ;;
esac
`

const script = (file: string, body: string) => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, body)
  chmodSync(file, 0o755)
}

const mkEv = (id: number, sink: unknown[]) => ({ sender: { id, once: () => undefined, on: () => undefined, isDestroyed: () => false, send: (_c: string, p: unknown) => sink.push(p) } }) as never

const confirms = { node: [] as string[], rust: [] as string[] }
let answer = true
let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const progNode: unknown[] = []
const progRust: unknown[] = []
const evNode = mkEv(11, progNode)
const evRust = mkEv(12, progRust)
const opens = { node: [] as string[], rust: [] as string[] }
let saveTarget = ''
let pullDest = ''
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

const norm = (v: unknown): unknown =>
  JSON.parse(
    JSON.stringify(v)
      .replace(/"modified":"[^"]*"/g, '"modified":"M"')
      .replace(/"handle":"[0-9a-f]{16}"/g, '"handle":"H"')
      .replace(/"token":"[0-9a-f]{32}"/g, '"token":"T"')
      .replace(/"(startedAt|durationMs|at|sinceResetMs)":\d+/g, '"$1":1')
      .replace(/"id":"[0-9a-f]{12}"/g, '"id":"X"')
      .replace(/"frames":\[[^\]]*\]/g, '"frames":"F"')
      .split(pullNode).join('<pull>')
      .split(pullRust).join('<pull>')
      .split(outDir).join('<out>'),
  )

const call = async (side: 'node' | 'rust', channel: string, ...args: unknown[]) => (side === 'node' ? node : rust).call(channel, side === 'node' ? evNode : evRust, ...args)
const both = async (channel: string, ...args: unknown[]) => {
  const a = norm(await call('node', channel, ...args))
  const b = norm(await call('rust', channel, ...args))
  expect(b).toEqual(a)
  return a as Record<string, unknown> & unknown[]
}

async function makeDb(): Promise<Buffer> {
  const SQL = await initSqlJs()
  const db = new SQL.Database()
  db.run("CREATE TABLE t1 (id INTEGER PRIMARY KEY, name TEXT, score REAL, data BLOB, big TEXT); CREATE TABLE empty (a); CREATE VIEW v1 AS SELECT id, name FROM t1;")
  db.run('INSERT INTO t1 VALUES (1, ?, 2.5, ?, NULL)', ['alpha', new Uint8Array([1, 2, 3])])
  db.run('INSERT INTO t1 VALUES (2, ?, 3.0, NULL, ?)', ['beta "q"', 'x'.repeat(5000)])
  db.run('INSERT INTO t1 VALUES (3, NULL, NULL, NULL, NULL)')
  const out = Buffer.from(db.export())
  db.close()
  return out
}

describe.skipIf(!HAVE_DESK || process.platform === 'win32')('devices extras: TS handlers and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of [state, dataNode, dataRust, recNode, recRust, outDir, pullNode, pullRust, path.dirname(pushSrc)]) mkdirSync(d, { recursive: true })
    script(path.join(sdk, 'platform-tools', 'adb'), ADB)
    writeFileSync(path.join(state, 'devices.txt'), 'List of devices attached\nR58M123 device usb:1-1 product:p model:Pixel_7 device:panther transport_id:1\nZZ offline\n')
    writeFileSync(path.join(state, 'shot.png'), PNG)
    writeFileSync(path.join(state, 'good.mp4'), MP4)
    writeFileSync(path.join(state, 'gfx.txt'), GFX)
    writeFileSync(path.join(state, 'mem.txt'), MEM)
    writeFileSync(pushSrc, 'hello push')
    const sd = path.join(state, 'sdcard')
    mkdirSync(path.join(sd, 'DCIM', 'Camera'), { recursive: true })
    mkdirSync(path.join(state, 'tmp'), { recursive: true })
    writeFileSync(path.join(sd, 'notes.txt'), 'hello device\n')
    writeFileSync(path.join(sd, 'big.bin'), Buffer.alloc(3000, 7))
    writeFileSync(path.join(sd, 'my file.txt'), 'spaced')
    writeFileSync(path.join(sd, 'run.sh'), 'echo hi')
    writeFileSync(path.join(sd, 'slow.bin'), 'x')
    writeFileSync(path.join(sd, 'DCIM', 'Camera', 'a.jpg'), 'jpg')
    const app = path.join(state, 'app', 'com.shop')
    mkdirSync(path.join(app, 'databases'), { recursive: true })
    mkdirSync(path.join(app, 'shared_prefs'), { recursive: true })
    mkdirSync(path.join(app, 'files'), { recursive: true })
    writeFileSync(path.join(app, 'files', 'cfg.json'), '{"a":1}')
    writeFileSync(path.join(app, 'databases', 'main.db'), await makeDb())
    writeFileSync(path.join(app, 'databases', 'junk.db'), Buffer.alloc(300, 1))
    writeFileSync(path.join(app, 'shared_prefs', 'p.xml'), PREFS)
    writeFileSync(path.join(app, 'shared_prefs', 'bad.xml'), 'not xml')
    const stamp = new Date('2024-01-02T10:11:00')
    const pin = (dir: string): void => {
      for (const n of readdirSync(dir)) {
        const p = path.join(dir, n)
        if (statSync(p).isDirectory()) pin(p)
        utimesSync(p, stamp, stamp)
      }
    }
    pin(sd)

    twin = await startTwin(
      { CCM_DATA_DIR: dataRust, ANDROID_HOME: sdk, ANDROID_SDK_ROOT: '' },
      {
        'devices.use': () => true,
        'devices.recDir': () => recRust,
        'devices.recKeeps': () => false,
        'devices.confirm': (p) => {
          const r = p as Record<string, unknown>
          confirms.rust.push([r.kind, r.name ?? r.pkg, r.where ?? r.file ?? '', r.dir ?? r.before ?? '', r.after ?? ''].join('|'))
          return answer
        },
      },
    )
    const env = { ...process.env, ANDROID_HOME: sdk, ANDROID_SDK_ROOT: '' }
    const manager = new DeviceManager({
      spawn: defaultSpawner,
      env,
      platform: process.platform,
      home: tmp,
      getSettings: () => ({ androidSdk: null }),
      history: new DeviceHistory(path.join(dataNode, 'adb-devices.json')),
      uiOriginalsFile: path.join(dataNode, 'ui.json'),
      broadcast: () => undefined,
      confirmRemove: async () => true,
      confirmWipe: async () => true,
    })
    const withDevice = <T,>(id: string, fn: Parameters<typeof manager.withReady<T>>[1]) => manager.withReady(id, fn)
    const recorder = new ScreenRecorder({ withDevice, tmpDir: recNode, kill: killAdbClient })
    const gate = async (d: unknown) => (d === pullNode || d === pullRust ? (d as string) : null)
    const saveRecording = async () => saveTarget
    for (const [side, ipc] of [['node', node], ['rust', rust]] as const) {
      const desk = side === 'rust' ? twin.router : undefined
      registerDevicesIpc({ ipcMain: ipc.ipcMain, isAppFrame: () => true, manager, resolveDir: gate, recorder, saveRecording, desk })
      const files = new DeviceFiles({
        withDevice,
        kill: killAdbClient,
        confirmDelete: async (r) => (confirms.node.push(['fs-delete', r.name, r.where, r.dir, ''].join('|')), answer),
        confirmReplace: async (r) => (confirms.node.push(['fs-replace', r.name, r.where, '', ''].join('|')), answer),
      })
      registerFilesIpc({
        ipcMain: ipc.ipcMain,
        isAppFrame: () => true,
        files,
        resolveDir: gate,
        pickFolder: async () => pullDest,
        pickFiles: async () => [pushSrc],
        openLocal: async (local, mode) => (opens[side].push(`${mode}:${path.basename(local)}`), true),
        desk,
      })
      const perf = new PerfService({
        withDevice,
        dataDir: dataNode,
        confirmCold: async (pkg) => (confirms.node.push(['perf-cold', pkg, '', '', ''].join('|')), answer),
      })
      registerPerfIpc({ ipcMain: ipc.ipcMain, isAppFrame: () => true, perf, desk })
      const inspector = new AppDataInspector({
        withDevice,
        sql: () => initSqlJs(),
        dataDir: dataNode,
        confirmWrite: async (r) => (confirms.node.push(['pref-write', r.pkg, r.file, r.before, r.after].join('|')), answer),
        confirmRestore: async (r) => (confirms.node.push(['pref-restore', r.pkg, r.file, '', ''].join('|')), answer),
      })
      registerAppDataIpc({ ipcMain: ipc.ipcMain, isAppFrame: () => true, inspector, desk })
    }
  })

  afterAll(async () => {
    await twin.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('records the screen, saves and discards the clip', async () => {
    expect(await both('ccm:dev-rec-start', 'ZZ')).toEqual({ ok: false, error: 'not-connected' })
    expect(await both('ccm:dev-rec-start', '-x')).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-rec-stop', 'R58M123')).toEqual({ ok: false, error: 'invalid' })
    const started = {} as Record<string, { ok: boolean; limitMs: number }>
    for (const side of ['node', 'rust'] as const) started[side] = (await call(side, 'ccm:dev-rec-start', 'R58M123')) as never
    expect(started.node).toMatchObject({ ok: true, limitMs: 30_000 })
    expect(started.rust).toMatchObject({ ok: true, limitMs: 30_000 })
    expect(await both('ccm:dev-rec-start', 'R58M123')).toEqual({ ok: false, error: 'busy' })
    await sleep(1500)
    const stopped = {} as Record<string, { ok: boolean; token: string; bytes: number; frames: string[] }>
    for (const side of ['node', 'rust'] as const) stopped[side] = (await call(side, 'ccm:dev-rec-stop', 'R58M123')) as never
    for (const side of ['node', 'rust'] as const) {
      expect(stopped[side]).toMatchObject({ ok: true, bytes: MP4.length, framesFailed: 0 })
      expect(stopped[side].frames.length).toBeGreaterThan(0)
      expect(stopped[side].frames[0]).toMatch(/^data:image\/png;base64,/)
    }
    expect(await both('ccm:dev-rec-stop', 'R58M123')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-rec-save', 'zz', 'x')).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-rec-save', 'a'.repeat(32), 'x')).toEqual({ ok: false, error: 'invalid' })
    saveTarget = path.join(outDir, 'node.mp4')
    expect(await call('node', 'ccm:dev-rec-save', stopped.node.token, 'My clip!')).toEqual({ ok: true, name: 'node.mp4' })
    saveTarget = path.join(outDir, 'rust.mp4')
    expect(await call('rust', 'ccm:dev-rec-save', stopped.rust.token, 'My clip!')).toEqual({ ok: true, name: 'rust.mp4' })
    expect(readFileSync(path.join(outDir, 'rust.mp4'))).toEqual(readFileSync(path.join(outDir, 'node.mp4')))
    saveTarget = ''
    expect(await call('node', 'ccm:dev-rec-save', stopped.node.token, 'x')).toEqual({ ok: false, error: 'cancelled' })
    expect(await call('rust', 'ccm:dev-rec-save', stopped.rust.token, 'x')).toEqual({ ok: false, error: 'cancelled' })
    expect(await call('node', 'ccm:dev-rec-discard', stopped.node.token)).toBe(true)
    expect(await call('rust', 'ccm:dev-rec-discard', stopped.rust.token)).toBe(true)
    expect(await both('ccm:dev-rec-discard', stopped.node.token)).toBe(false)
    expect(listTree(recNode)).toEqual({})
    expect(listTree(recRust)).toEqual({})
    const held = twin.events.filter((e) => e.event === 'ccm:dev-rec-held').map((e) => (e.payload as string[]).length)
    expect(held.at(-1)).toBe(0)
    expect(Math.max(...held)).toBe(1)
  }, 30_000)

  it('cancels a recording and drops the device file', async () => {
    for (const side of ['node', 'rust'] as const) expect(await call(side, 'ccm:dev-rec-start', 'R58M123')).toMatchObject({ ok: true })
    await sleep(300)
    expect(await both('ccm:dev-rec-cancel', 'R58M123')).toEqual({ ok: true })
    expect(await both('ccm:dev-rec-cancel', 'R58M123')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-rec-stop', 'R58M123')).toEqual({ ok: false, error: 'invalid' })
    expect(listTree(recRust)).toEqual({})
    expect(readdirNames(path.join(state, 'sdcard')).filter((n) => n.startsWith('orrery-rec'))).toEqual([])
  }, 20_000)

  it('lists device folders and validates scopes', async () => {
    const sd = { kind: 'sdcard' }
    const list = (await both('ccm:devfs-list', 'R58M123', sd, '/sdcard')) as unknown as { ok: boolean; entries: { name: string; kind: string; size: number | null }[] }
    expect(list.ok).toBe(true)
    expect(list.entries.map((e) => e.name)).toEqual(['DCIM', 'big.bin', 'my file.txt', 'notes.txt', 'run.sh', 'slow.bin'])
    expect(list.entries.find((e) => e.name === 'big.bin')).toMatchObject({ kind: 'file', size: 3000 })
    expect(await both('ccm:devfs-list', 'R58M123', sd, '/sdcard/DCIM/Camera')).toMatchObject({ ok: true })
    expect(await both('ccm:devfs-list', 'R58M123', sd, '/sdcard/missing')).toMatchObject({ ok: false, error: 'not-found' })
    expect(await both('ccm:devfs-list', 'R58M123', sd, '/etc')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:devfs-list', 'R58M123', sd, '/sdcard/../x')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:devfs-list', 'R58M123', { kind: 'other' }, '/sdcard')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:devfs-list', 'ZZ', sd, '/sdcard')).toEqual({ ok: false, error: 'not-connected' })
    expect(await both('ccm:devfs-list', 'R58M123', { kind: 'app', pkg: 'com.shop' }, '/files')).toMatchObject({ ok: true })
    expect(await both('ccm:devfs-list', 'R58M123', { kind: 'app', pkg: 'com.nodebug' }, '/')).toEqual({ ok: false, error: 'not-debuggable' })
    expect(await both('ccm:devfs-list', 'R58M123', { kind: 'app', pkg: 'com.unknown' }, '/')).toEqual({ ok: false, error: 'unknown-package' })
  })

  it('makes folders and deletes after a confirm', async () => {
    const sd = { kind: 'sdcard' }
    confirms.node.length = 0
    confirms.rust.length = 0
    expect(await call('node', 'ccm:devfs-mkdir', 'R58M123', sd, '/sdcard', 'newdir')).toEqual({ ok: true })
    expect(await call('rust', 'ccm:devfs-mkdir', 'R58M123', sd, '/sdcard', 'newdir2')).toEqual({ ok: true })
    expect(await both('ccm:devfs-mkdir', 'R58M123', sd, '/sdcard', 'newdir')).toMatchObject({ ok: false })
    expect(await both('ccm:devfs-mkdir', 'R58M123', sd, '/sdcard', 'a/b')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:devfs-delete', 'R58M123', sd, '/sdcard', true)).toEqual({ ok: false, error: 'outside' })
    answer = false
    expect(await both('ccm:devfs-delete', 'R58M123', sd, '/sdcard/newdir', true)).toEqual({ ok: false, error: 'cancelled' })
    answer = true
    expect(await call('node', 'ccm:devfs-delete', 'R58M123', sd, '/sdcard/newdir', true)).toEqual({ ok: true })
    expect(await call('rust', 'ccm:devfs-delete', 'R58M123', sd, '/sdcard/newdir2', true)).toEqual({ ok: true })
    expect(await both('ccm:devfs-delete', 'R58M123', { kind: 'app', pkg: 'com.shop' }, '/', true)).toEqual({ ok: false, error: 'outside' })
    expect(confirms.rust.map((c) => c.replaceAll('newdir2', 'newdir'))).toEqual(confirms.node)
    expect(confirms.node).toHaveLength(2)
  })

  it('pulls files and folders with progress, then opens them safely', async () => {
    const sd = { kind: 'sdcard' }
    for (const [side, dir] of [['node', pullNode], ['rust', pullRust]] as const) {
      pullDest = dir
      const req = (p: string, kind: string, size: number | null, transfer: string) => ({ scope: sd, path: p, kind, size, transfer, dest: { kind: 'pick' } })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/big.bin', 'file', 3000, '0123456789abcdef'))).toMatchObject({ ok: true, name: 'big.bin' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/big.bin', 'file', 3000, '0123456789abcdef'))).toMatchObject({ ok: true, name: 'big (1).bin' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/my file.txt', 'file', 6, '1123456789abcdef'))).toMatchObject({ ok: true })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/DCIM', 'dir', null, '2123456789abcdef'))).toMatchObject({ ok: true, name: 'DCIM' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/big.bin', 'file', 99, '3123456789abcdef'))).toMatchObject({ ok: false, error: 'failed', detail: '3000 / 99 B' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/missing.bin', 'file', null, '4123456789abcdef'))).toMatchObject({ ok: false })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { ...req('/sdcard/notes.txt', 'file', 13, '5123456789abcdef'), dest: { kind: 'dir', dir: tmp } })).toEqual({ ok: false, error: 'outside' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { ...req('/sdcard/notes.txt', 'file', 13, '5123456789abcdef'), dest: { kind: 'dir', dir: dir } })).toMatchObject({ ok: true, name: 'notes.txt' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { ...req('/sdcard/notes.txt', 'file', 13, 'bad'), dest: { kind: 'pick' } })).toEqual({ ok: false, error: 'invalid' })
      pullDest = ''
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', req('/sdcard/notes.txt', 'file', 13, '6123456789abcdef'))).toEqual({ ok: false, error: 'cancelled' })
      pullDest = dir
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { scope: { kind: 'app', pkg: 'com.shop' }, path: '/files', kind: 'dir', size: null, transfer: '7123456789abcdef', dest: { kind: 'pick' } })).toEqual({ ok: false, error: 'unsupported' })
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { scope: { kind: 'app', pkg: 'com.shop' }, path: '/files/cfg.json', kind: 'file', size: 7, transfer: '8123456789abcdef', dest: { kind: 'pick' } })).toMatchObject({ ok: true, name: 'cfg.json' })
    }
    expect(listTree(pullRust)).toEqual(listTree(pullNode))
    expect(Object.keys(listTree(pullNode)).sort()).toEqual(['DCIM/Camera/a.jpg', 'big (1).bin', 'big.bin', 'cfg.json', 'my file.txt', 'notes.txt'])
    const doneOf = (p: unknown[]) => (p as { name: string; done: number }[]).filter((x) => x.name === 'big.bin').map((x) => x.done).at(-1)
    expect(doneOf(progRust)).toBe(3000)
    expect(doneOf(progNode)).toBe(3000)
    expect((progRust as { transfer: string }[])[0]).toEqual((progNode as { transfer: string }[])[0])
    expect(Object.keys((progRust as object[])[0]!)).toEqual(Object.keys((progNode as object[])[0]!))
    const pulled = (dir: string, n: string) => path.join(dir, n)
    expect(await call('node', 'ccm:devfs-open', pulled(pullNode, 'notes.txt'), 'open')).toBe(true)
    expect(await call('rust', 'ccm:devfs-open', pulled(pullRust, 'notes.txt'), 'open')).toBe(true)
    expect(await call('node', 'ccm:devfs-open', pulled(pullNode, 'notes.txt'), 'reveal')).toBe(true)
    expect(await call('rust', 'ccm:devfs-open', pulled(pullRust, 'notes.txt'), 'reveal')).toBe(true)
    expect(await both('ccm:devfs-open', path.join(tmp, 'x.txt'), 'open')).toBe(false)
    expect(await both('ccm:devfs-open', 5, 'open')).toBe(false)
    expect(opens.rust).toEqual(opens.node)
    expect(opens.node).toEqual(['open:notes.txt', 'reveal:notes.txt'])
  }, 30_000)

  it('pushes files, asks before replacing, and cancels a running pull', async () => {
    const sd = { kind: 'sdcard' }
    const push = (side: 'node' | 'rust', source: unknown, dir = '/sdcard/DCIM', transfer = '9123456789abcdef') => call(side, 'ccm:devfs-push', 'R58M123', { scope: sd, dir, source, transfer })
    confirms.node.length = 0
    confirms.rust.length = 0
    for (const side of ['node', 'rust'] as const) {
      expect(await push(side, { kind: 'pick' })).toEqual({ ok: true, pushed: ['upload.txt'], skipped: [] })
      expect(readFileSync(path.join(state, 'sdcard', 'DCIM', 'upload.txt'), 'utf8')).toBe('hello push')
      answer = false
      expect(await push(side, { kind: 'paths', paths: [pushSrc, path.join(tmp, 'nope.txt')] })).toEqual({ ok: true, pushed: [], skipped: ['nope.txt', 'upload.txt'] })
      answer = true
      expect(await push(side, { kind: 'paths', paths: [pushSrc] })).toEqual({ ok: true, pushed: ['upload.txt'], skipped: [] })
      rmSync(path.join(state, 'sdcard', 'DCIM', 'upload.txt'))
      expect(await push(side, { kind: 'paths', paths: ['relative.txt'] })).toEqual({ ok: false, error: 'invalid' })
      expect(await push(side, { kind: 'paths', paths: [] })).toEqual({ ok: false, error: 'invalid' })
      expect(await push(side, { kind: 'other' })).toEqual({ ok: false, error: 'invalid' })
      expect(await push(side, { kind: 'pick' }, '/etc')).toEqual({ ok: false, error: 'invalid' })
    }
    expect(confirms.rust).toEqual(confirms.node)
    expect(confirms.node.filter((c) => c.startsWith('fs-replace'))).toHaveLength(2)
    for (const [side, dir] of [['node', pullNode], ['rust', pullRust]] as const) {
      pullDest = dir
      const running = call(side, 'ccm:devfs-pull', 'R58M123', { scope: sd, path: '/sdcard/slow.bin', kind: 'file', size: null, transfer: 'abcdef0123456789', dest: { kind: 'pick' } })
      await sleep(500)
      expect(await call(side, 'ccm:devfs-pull', 'R58M123', { scope: sd, path: '/sdcard/notes.txt', kind: 'file', size: 13, transfer: 'bbcdef0123456789', dest: { kind: 'pick' } })).toEqual({ ok: false, error: 'busy' })
      expect(await call(side, 'ccm:devfs-cancel', 'bad')).toBe(false)
      expect(await call(side, 'ccm:devfs-cancel', 'ffffffffffffffff')).toBe(false)
      expect(await call(side, 'ccm:devfs-cancel', 'abcdef0123456789')).toBe(true)
      expect(await running).toEqual({ ok: false, error: 'cancelled' })
    }
    expect(listTree(pullRust)).toEqual(listTree(pullNode))
  }, 30_000)

  it('measures frames and memory, resets, cold starts and keeps history', async () => {
    const results: Record<string, unknown[]> = { node: [], rust: [] }
    for (const side of ['node', 'rust'] as const) {
      results[side].push(await call(side, 'ccm:perf-measure', 'R58M123', 'com.shop', 'Pixel 7\u0007'))
      results[side].push(await call(side, 'ccm:perf-reset', 'R58M123', 'com.shop'))
      results[side].push(await call(side, 'ccm:perf-measure', 'R58M123', 'com.shop', 'Pixel 7'))
      results[side].push(await call(side, 'ccm:perf-measure', 'R58M123', 'com.gone', 'Pixel 7'))
      results[side].push(await call(side, 'ccm:perf-reset', 'R58M123', 'com.gone'))
      results[side].push(await call(side, 'ccm:perf-measure', 'ZZ', 'com.shop', ''))
    }
    expect(norm(results.rust)).toEqual(norm(results.node))
    const first = norm(results.node[0]) as { ok: boolean; sample: { frames: { jankyPct: number; p95: number }; memory: { totalPss: number } } }
    expect(first.ok).toBe(true)
    expect(first.sample.frames).toMatchObject({ total: 200, janky: 20, jankyPct: 10, p95: 20 })
    expect(first.sample.memory.totalPss).toBe(9000)
    expect(norm(results.node[2])).toMatchObject({ sample: { sinceResetMs: 1 } })
    confirms.node.length = 0
    confirms.rust.length = 0
    answer = false
    expect(await both('ccm:perf-cold', 'R58M123', 'com.shop', 'Pixel 7')).toEqual({ ok: false, error: 'cancelled' })
    answer = true
    const cold = (await both('ccm:perf-cold', 'R58M123', 'com.shop', 'Pixel 7')) as unknown as { ok: boolean; sample: { kind: string; coldStart: object } }
    expect(cold.sample).toMatchObject({ kind: 'cold', coldStart: { totalMs: 420, waitMs: 433, launchState: 'COLD' } })
    expect(confirms.rust).toEqual(confirms.node)
    expect(await both('ccm:perf-cold', 'R58M123', 'bad pkg', '')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:perf-history', 'bad pkg')).toEqual({ ok: false, error: 'invalid' })
    const hist = (await both('ccm:perf-history', 'com.shop')) as unknown as { samples: unknown[] }
    expect(hist.samples.length).toBe(3)
    expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'perf', 'com.shop.json'), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'perf', 'com.shop.json'), 'utf8'))))
    expect(readFileSync(path.join(dataRust, 'perf', 'com.shop.json'), 'utf8')).toMatch(/^\{\n "version": 1,\n "samples": \[\n  \{\n   "id"/)
    expect(await both('ccm:perf-clear', 'com.shop')).toEqual({ ok: true, pkg: 'com.shop', samples: [] })
    expect(existsSync(path.join(dataRust, 'perf', 'com.shop.json'))).toBe(false)
  }, 20_000)

  it('lists app data and refuses non-debuggable apps', async () => {
    const list = (await both('ccm:appdata-list', 'R58M123', 'com.shop')) as unknown as { databases: { name: string }[]; prefs: { name: string }[] }
    expect(list.databases.map((d) => d.name)).toEqual(['junk.db', 'main.db'])
    expect(list.prefs.map((d) => d.name)).toEqual(['bad.xml', 'p.xml'])
    expect(await both('ccm:appdata-list', 'R58M123', 'com.nodebug')).toEqual({ ok: false, error: 'not-debuggable' })
    expect(await both('ccm:appdata-list', 'R58M123', 'com.unknown')).toEqual({ ok: false, error: 'unknown-package' })
    expect(await both('ccm:appdata-list', 'ZZ', 'com.shop')).toEqual({ ok: false, error: 'not-connected' })
    expect(await both('ccm:appdata-list', 'R58M123', 'bad pkg')).toEqual({ ok: false, error: 'invalid' })
  })

  it('opens a database copy and pages, queries and refuses writes', async () => {
    expect(await both('ccm:appdata-db-open', 'R58M123', 'com.shop', 'junk.db')).toEqual({ ok: false, error: 'not-sqlite' })
    expect(await both('ccm:appdata-db-open', 'R58M123', 'com.shop', 'nope.db')).toEqual({ ok: false, error: 'gone' })
    expect(await both('ccm:appdata-db-open', 'R58M123', 'com.shop', '../x')).toEqual({ ok: false, error: 'invalid' })
    const a = (await call('node', 'ccm:appdata-db-open', 'R58M123', 'com.shop', 'main.db')) as { ok: boolean; handle: string; tables: { name: string; rows: number | null; type: string }[]; wal: string }
    const b = (await call('rust', 'ccm:appdata-db-open', 'R58M123', 'com.shop', 'main.db')) as typeof a
    expect(norm(b)).toEqual(norm(a))
    expect(a.tables.map((t) => [t.name, t.type, t.rows])).toEqual([['empty', 'table', 0], ['t1', 'table', 3], ['v1', 'view', null]])
    expect(a.wal).toBe('none')
    const rows = async (side: 'node' | 'rust', h: string, ...rest: unknown[]) => norm(await call(side, 'ccm:appdata-db-rows', h, ...rest))
    expect(await rows('rust', b.handle, 't1', 0)).toEqual(await rows('node', a.handle, 't1', 0))
    const page = (await rows('node', a.handle, 't1', 0)) as unknown as { rows: unknown[][]; total: number; columns: string[] }
    expect(page.total).toBe(3)
    expect(page.rows[0]).toEqual([1, 'alpha', 2.5, { blob: 3 }, null])
    expect((page.rows[1]![4] as string).length).toBe(4001)
    expect(await rows('rust', b.handle, 'v1', 0)).toEqual(await rows('node', a.handle, 'v1', 0))
    expect(await rows('rust', b.handle, 'nope', 0)).toEqual({ ok: false, error: 'invalid' })
    expect(await rows('rust', b.handle, 't1', 5)).toEqual(await rows('node', a.handle, 't1', 5))
    expect(await rows('rust', 'f'.repeat(16), 't1', 0)).toEqual({ ok: false, error: 'gone' })
    expect(await rows('rust', 'zz', 't1', 0)).toEqual({ ok: false, error: 'invalid' })
    const queries = ['select id, name from t1 where id > 1 order by id', 'SELECT count(*) AS n FROM t1', 'pragma table_info(t1)', 'with x as (select 1 as a) select * from x', 'select * from nope', 'selec 1', 'delete from t1', 'select 1; select 2', 'pragma journal_mode = wal', '   ', 'select \'unterminated']
    for (const q of queries) {
      const x = norm(await call('node', 'ccm:appdata-db-query', a.handle, q))
      const y = norm(await call('rust', 'ccm:appdata-db-query', b.handle, q))
      expect(y, q).toEqual(x)
    }
    expect(norm(await call('rust', 'ccm:appdata-db-query', b.handle, 'delete from t1'))).toEqual({ ok: false, error: 'sql-rejected', detail: 'not-read' })
    expect(await call('rust', 'ccm:appdata-db-close', b.handle)).toBe(true)
    expect(await call('node', 'ccm:appdata-db-close', a.handle)).toBe(true)
    expect(norm(await call('rust', 'ccm:appdata-db-query', b.handle, 'select 1'))).toEqual({ ok: false, error: 'gone' })
    const handles: string[] = []
    for (let i = 0; i < 4; i++) handles.push(((await call('rust', 'ccm:appdata-db-open', 'R58M123', 'com.shop', 'main.db')) as { handle: string }).handle)
    expect(norm(await call('rust', 'ccm:appdata-db-query', handles[0]!, 'select 1'))).toEqual({ ok: false, error: 'gone' })
    expect(norm(await call('rust', 'ccm:appdata-db-query', handles[3]!, 'select 1'))).toMatchObject({ ok: true })
  }, 20_000)

  it('reads, edits and restores a preference with an original kept', async () => {
    const file = path.join(state, 'app', 'com.shop', 'shared_prefs', 'p.xml')
    expect(await both('ccm:appdata-prefs-read', 'R58M123', 'com.shop', 'p.xml')).toMatchObject({ ok: true, hasOriginal: false, entries: [{ key: 'token', type: 'string', value: 'a & b' }, { key: 'launches', type: 'int', value: '3' }, { key: 'tags', type: 'set', value: ['x'] }] })
    expect(await both('ccm:appdata-prefs-read', 'R58M123', 'com.shop', 'bad.xml')).toEqual({ ok: false, error: 'pull-failed' })
    expect(await both('ccm:appdata-prefs-read', 'R58M123', 'com.shop', 'nope.xml')).toEqual({ ok: false, error: 'gone' })
    expect(await both('ccm:appdata-prefs-read', 'R58M123', 'com.shop', 'p.txt')).toEqual({ ok: false, error: 'invalid' })
    const edit = { key: 'launches', type: 'int', before: '3', after: ' 4 ' }
    expect(await both('ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', { ...edit, before: '9' })).toEqual({ ok: false, error: 'changed' })
    expect(await both('ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', { ...edit, after: 'x' })).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', { ...edit, type: 'bogus' })).toEqual({ ok: false, error: 'invalid' })
    confirms.node.length = 0
    confirms.rust.length = 0
    answer = false
    expect(await both('ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', edit)).toEqual({ ok: false, error: 'cancelled' })
    answer = true
    const a = norm(await call('node', 'ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', edit))
    expect(readFileSync(file, 'utf8')).toContain('<int name="launches" value="4" />')
    writeFileSync(file, PREFS)
    const b = norm(await call('rust', 'ccm:appdata-prefs-write', 'R58M123', 'com.shop', 'p.xml', edit))
    expect(b).toEqual(a)
    expect(a).toMatchObject({ ok: true, hasOriginal: true })
    expect(readFileSync(file, 'utf8')).toBe(PREFS.replace('value="3"', 'value="4"'))
    expect(confirms.rust).toEqual(confirms.node)
    expect(confirms.node).toEqual(['pref-write|com.shop|p.xml|3|4', 'pref-write|com.shop|p.xml|3|4'])
    expect(listTree(path.join(dataRust, 'appdata-originals'))).toEqual(listTree(path.join(dataNode, 'appdata-originals')))
    expect(Object.values(listTree(path.join(dataRust, 'appdata-originals')))).toEqual([PREFS])
    writeFileSync(file, PREFS.replace('value="3"', 'value="4"'))
    expect(norm(await call('node', 'ccm:appdata-prefs-restore', 'R58M123', 'com.shop', 'p.xml'))).toMatchObject({ ok: true, hasOriginal: false })
    expect(readFileSync(file, 'utf8')).toBe(PREFS)
    writeFileSync(file, PREFS.replace('value="3"', 'value="4"'))
    expect(norm(await call('rust', 'ccm:appdata-prefs-restore', 'R58M123', 'com.shop', 'p.xml'))).toMatchObject({ ok: true, hasOriginal: false })
    expect(readFileSync(file, 'utf8')).toBe(PREFS)
    expect(await both('ccm:appdata-prefs-restore', 'R58M123', 'com.shop', 'p.xml')).toEqual({ ok: false, error: 'no-original' })
    expect(listTree(path.join(dataRust, 'appdata-originals'))).toEqual({})
  }, 20_000)

  it('stayed on the rust path', () => {
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})

function readdirNames(dir: string): string[] {
  return readdirSync(dir).sort()
}
