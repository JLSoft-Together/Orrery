import { chmodSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { DeviceHistory } from '../src/devices/history'
import { registerDevicesIpc } from '../src/devices/ipc'
import { DeviceManager } from '../src/devices/manager'
import { defaultSpawner } from '../src/run/proc'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-devices-')))
const sdk = path.join(tmp, 'sdk')
const state = path.join(sdk, 'state')
const avdHome = path.join(tmp, 'avd')
const proj = path.join(tmp, 'proj')
const dataNode = path.join(tmp, 'data-node')
const dataRust = path.join(tmp, 'data-rust')
const PNG = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 4, 0, 0, 0, 6]), Buffer.alloc(12)])
const UI_XML = '<?xml version="1.0"?><hierarchy rotation="1"><node class="android.widget.FrameLayout" package="com.a" bounds="[0,0][100,200]" clickable="false"><node class="android.widget.Button" text="OK &amp; go" resource-id="com.a:id/ok" bounds="[10,20][50,60]" clickable="true" focusable="true" password="false"/><node class="android.widget.EditText" text="secret" password="true" bounds="[0,70][100,90]"/></node></hierarchy>'

const ADB = `#!/bin/sh
D="$(dirname "$0")/../state"
echo "$@" >> "$D/log.txt"
case "$1" in
devices) cat "$D/devices.txt" ;;
connect) case "$2" in *bad*) echo "failed to connect to '$2': Connection refused" ;; *) echo "connected to $2" ;; esac ;;
pair) if [ "$3" = "123456" ]; then echo "Successfully paired to $2 [guid=adb-x]"; else echo "Failed: Wrong password"; fi ;;
disconnect) echo "disconnected $2" ;;
mdns) case "$2" in check) echo "mdns daemon version [Openscreen discovery 0.0.0]" ;; services) cat "$D/mdns.txt" ;; esac ;;
track-devices) body="$(cat "$D/devices.txt")"; printf '%04x%s' \${#body} "$body"; exec sleep 30 ;;
-s) id="$2"; shift 2
  case "$1" in
  tcpip) echo "restarting in TCP mode port: $2" ;;
  emu) case "$2" in avd) if [ "$id" = "emulator-5554" ]; then printf 'Pixel_8\\nOK\\n'; else echo "KO: unknown"; fi ;; kill) echo "OK: killing emulator, bye bye" ;; esac ;;
  exec-out) case "$2" in screencap) cat "$D/shot.png" ;; cat) cat "$D/ui.xml" ;; esac ;;
  shell) shift
    case "$1" in
    ip) echo "    inet 192.168.1.5/24 brd 192.168.1.255 scope global wlan0" ;;
    wm) printf 'Physical density: 440\\nOverride density: 420\\n' ;;
    am) echo "Starting: Intent"; echo "Status: ok" ;;
    getprop) echo 34 ;;
    settings) case "$2" in get) case "$4" in font_scale) echo null ;; *) echo 1 ;; esac ;; esac ;;
    cmd) case "$2" in uimode) if [ -z "$4" ]; then echo "Night mode: no"; fi ;; locale) case "$3" in get-app-locales) echo "Locales for $4 for user 0 are [vi-VN,en]" ;; esac ;; esac ;;
    uiautomator) echo "UI hierchary dumped to: /sdcard/orrery-ui.xml" ;;
    rm) ;;
    esac ;;
  esac ;;
esac
`
const EMU = `#!/bin/sh
case "$1" in
-list-avds) echo "INFO    | something"; echo "Pixel_8"; echo "Test_AVD" ;;
-avd) exec sleep 6 ;;
esac
`

const script = (file: string, body: string) => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, body)
  chmodSync(file, 0o755)
}

const DEVICES = 'List of devices attached\nR58M123 device usb:1-1 product:p model:Pixel_7 device:panther transport_id:1\n192.168.1.9:5555 device product:q model:SM_G\nemulator-5554 device product:sdk model:sdk_gphone\nZZ offline\n'

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const ev = { sender: { id: 7, once: () => undefined, on: () => undefined } } as never
const nodeEvents: unknown[] = []
const confirms: string[] = []
let confirmAnswer = true

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"lastSeen":\d+(\.\d+)?/g, '"lastSeen":1').replace(/orrery-[0-9a-f]{8}/g, 'orrery-X'))
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

describe.skipIf(!HAVE_DESK || process.platform === 'win32')('devices: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of [state, avdHome, proj, dataNode, dataRust]) mkdirSync(d, { recursive: true })
    script(path.join(sdk, 'platform-tools', 'adb'), ADB)
    script(path.join(sdk, 'emulator', 'emulator'), EMU)
    writeFileSync(path.join(state, 'devices.txt'), DEVICES)
    writeFileSync(path.join(state, 'mdns.txt'), '')
    writeFileSync(path.join(state, 'shot.png'), PNG)
    writeFileSync(path.join(state, 'ui.xml'), UI_XML)
    writeFileSync(path.join(avdHome, 'Pixel_8.ini'), 'path=' + path.join(avdHome, 'Pixel_8.avd') + '\ntarget=android-34\n')
    mkdirSync(path.join(avdHome, 'Pixel_8.avd'), { recursive: true })
    writeFileSync(path.join(avdHome, 'Pixel_8.avd', 'config.ini'), 'avd.ini.displayname=Pixel 8 Phone\nabi.type=arm64-v8a\nhw.device.name=pixel_8\ntag.id=google_apis\n')
    mkdirSync(path.join(proj, 'app', 'src', 'main', 'res', 'values-vi'), { recursive: true })
    writeFileSync(path.join(proj, 'app', 'src', 'main', 'AndroidManifest.xml'), '<manifest package="com.shop"><application><activity android:name=".Main" android:exported="true"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter><intent-filter><action android:name="android.intent.action.VIEW"/><data android:scheme="https" android:host="shop.example.com" android:pathPrefix="/p"/></intent-filter></activity><activity android:name="Other"/></application></manifest>')
    twin = await startTwin(
      { CCM_DATA_DIR: dataRust, ANDROID_HOME: sdk, ANDROID_SDK_ROOT: '', ANDROID_AVD_HOME: avdHome },
      {
        'devices.use': () => true,
        'devices.confirm': () => confirmAnswer,
      },
    )
    const env = { ...process.env, ANDROID_HOME: sdk, ANDROID_SDK_ROOT: '', ANDROID_AVD_HOME: avdHome }
    const manager = new DeviceManager({
      spawn: defaultSpawner,
      env,
      platform: process.platform,
      home: tmp,
      getSettings: () => ({ androidSdk: null }),
      history: new DeviceHistory(path.join(dataNode, 'adb-devices.json')),
      uiOriginalsFile: path.join(dataNode, 'ui.json'),
      broadcast: (e) => nodeEvents.push(e),
      confirmRemove: async (ids) => (confirms.push(`remove:${ids.join(',')}`), confirmAnswer),
      confirmWipe: async (name) => (confirms.push(`wipe:${name}`), confirmAnswer),
      timing: { wifiSwitchMs: 1_000 },
    })
    const gate = async (d: unknown) => (d === proj ? proj : null)
    registerDevicesIpc({ ipcMain: node.ipcMain, isAppFrame: () => true, manager, resolveDir: gate })
    registerDevicesIpc({ ipcMain: rust.ipcMain, isAppFrame: () => true, manager, resolveDir: gate, desk: twin.router })
  })

  afterAll(async () => {
    await twin.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, ...args: unknown[]) => {
    const a = norm(await node.call(channel, ev, ...args))
    const b = norm(await rust.call(channel, ev, ...args))
    expect(b).toEqual(a)
    return a as Record<string, unknown> & unknown[]
  }

  it('lists devices and keeps remembered ones offline', async () => {
    const snap = (await both('ccm:dev-snapshot')) as unknown as { adb: string; devices: { id: string }[]; autoConnect: boolean }
    expect(snap.devices.map((d) => d.id)).toContain('R58M123')
    writeFileSync(path.join(state, 'devices.txt'), 'List of devices attached\nR58M123 device usb:1-1 product:p model:Pixel_7 device:panther transport_id:1\nemulator-5554 device product:sdk model:sdk_gphone\n')
    const after = (await both('ccm:dev-snapshot')) as unknown as { devices: { id: string; online: boolean }[] }
    expect(after.devices.find((d) => d.id === '192.168.1.9:5555')?.online).toBe(false)
  })

  it('sets alias, auto connect and validates ids', async () => {
    nodeEvents.length = 0
    twin.events.length = 0
    expect(await both('ccm:dev-alias', 'R58M123', '  My phone  ')).toEqual({ ok: true })
    expect(await both('ccm:dev-alias', 'R58M123', 'x'.repeat(65))).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-alias', 'nope', 'a')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-alias', '-bad', 'a')).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-alias', 'R58M123', null)).toEqual({ ok: true })
    expect(await both('ccm:dev-auto-connect', false)).toEqual({ ok: true })
    expect(await both('ccm:dev-auto-connect', 'yes')).toEqual({ ok: false, error: 'denied' })
    expect(norm(twin.events.filter((e) => e.event === 'ccm:dev-event').map((e) => e.payload))).toEqual(norm(nodeEvents))
    expect(nodeEvents.length).toBe(3)
    expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'adb-devices.json'), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'adb-devices.json'), 'utf8'))))
  })

  it('connects, pairs and disconnects', async () => {
    expect(await both('ccm:dev-connect', '192.168.1.20')).toEqual({ ok: true })
    expect(await both('ccm:dev-connect', 'bad.local:5557')).toMatchObject({ ok: false, error: 'connect-failed' })
    expect(await both('ccm:dev-connect', 'bad host!')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-connect', '')).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-pair', '192.168.1.20:40000', '123456')).toEqual({ ok: true, detail: 'paired' })
    expect(await both('ccm:dev-pair', '192.168.1.20:40000', '000000')).toMatchObject({ ok: false, error: 'pair-failed' })
    expect(await both('ccm:dev-pair', '192.168.1.20:40000', '12')).toEqual({ ok: false, error: 'invalid' })
    writeFileSync(path.join(state, 'mdns.txt'), 'adb-x\t_adb-tls-connect._tcp.\t192.168.1.20:41111\n')
    expect(await both('ccm:dev-pair', '192.168.1.20:40000', '123456')).toEqual({ ok: true })
    expect(await both('ccm:dev-disconnect', ['192.168.1.9:5555', 'R58M123', 'nope'])).toHaveLength(3)
    expect(await both('ccm:dev-disconnect', [])).toEqual([])
    expect(await both('ccm:dev-connect-many', ['10.0.0.1', 'bad.one', '10.0.0.3'])).toHaveLength(3)
    expect(await both('ccm:dev-to-wifi', 'R58M123')).toEqual({ ok: true })
    expect(await both('ccm:dev-to-wifi', '')).toEqual({ ok: false, error: 'denied' })
  }, 20_000)

  it('removes remembered devices after a confirm', async () => {
    confirmAnswer = false
    expect(await both('ccm:dev-remove', ['192.168.1.9:5555'])).toEqual({ ok: false, error: 'cancelled' })
    expect(await both('ccm:dev-remove', ['R58M123'])).toEqual({ ok: true })
    confirmAnswer = true
    expect(await both('ccm:dev-remove', ['192.168.1.9:5555'])).toEqual({ ok: true })
    expect(await both('ccm:dev-remove', [])).toEqual({ ok: false, error: 'denied' })
    expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'adb-devices.json'), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'adb-devices.json'), 'utf8'))))
  })

  it('discovers mdns devices', async () => {
    writeFileSync(path.join(state, 'mdns.txt'), 'adb-A\t_adb-tls-connect._tcp.\t192.168.1.50:37000\nadb-B\t_adb-tls-pairing._tcp.\t192.168.1.51:38000\nadb-C\t_adb._tcp\t192.168.1.52:5555\n')
    expect(await both('ccm:dev-discover')).toHaveLength(2)
    writeFileSync(path.join(state, 'mdns.txt'), '')
    expect(await both('ccm:dev-discover')).toEqual({ ok: false, error: 'no-devices' })
  })

  it('captures the screen, dumps the ui, launches and reads the manifest', async () => {
    const cap = await both('ccm:dev-screencap', 'R58M123')
    expect(cap).toMatchObject({ ok: true, width: 4, height: 6 })
    expect(await both('ccm:dev-screencap', 'ZZ')).toEqual({ ok: false, error: 'not-connected' })
    const dump = (await both('ccm:dev-uidump', 'R58M123')) as { ok: boolean; nodes: { text: string }[]; density: number }
    expect(dump.ok).toBe(true)
    expect(dump.density).toBe(420)
    expect(dump.nodes.map((n) => n.text)).toEqual(['', 'OK & go', ''])
    expect(await both('ccm:dev-launch', 'R58M123', { kind: 'uri', uri: 'https://shop.example.com/p/1', pkg: 'com.shop' })).toMatchObject({ ok: true })
    expect(await both('ccm:dev-launch', 'R58M123', { kind: 'activity', pkg: 'com.shop', activity: '.Main' })).toMatchObject({ ok: true })
    expect(await both('ccm:dev-launch', 'R58M123', { kind: 'uri', uri: 'bad uri' })).toEqual({ ok: false, error: 'invalid' })
    const man = (await both('ccm:dev-manifest', proj)) as { ok: boolean; activities: { launcher: boolean }[]; locales: string[] }
    expect(man.ok).toBe(true)
    expect(man.locales).toEqual(['vi'])
    expect(await both('ccm:dev-manifest', path.join(tmp, 'elsewhere'))).toEqual({ ok: false, error: 'denied' })
  })

  it('reads, changes and restores display settings', async () => {
    const read = (await both('ccm:dev-ui-read', 'R58M123', 'com.shop')) as { ok: boolean; state: { sdk: number; night: string; locale: { current: string }; changed: string[] } }
    expect(read.state).toMatchObject({ sdk: 34, night: 'no', locale: { current: 'vi-VN,en' }, changed: [] })
    const night = (await both('ccm:dev-ui-set', 'R58M123', { key: 'night', value: 'yes' }, 'com.shop')) as { state: { changed: string[] } }
    expect(night.state.changed).toEqual(['night'])
    await both('ccm:dev-ui-set', 'R58M123', { key: 'fontScale', value: 1.333 }, null)
    await both('ccm:dev-ui-set', 'R58M123', { key: 'density', value: 400 }, null)
    await both('ccm:dev-ui-set', 'R58M123', { key: 'animations', off: true }, null)
    const loc = (await both('ccm:dev-ui-set', 'R58M123', { key: 'locale', pkg: 'com.shop', value: 'en' }, 'com.shop')) as { state: { changed: string[] } }
    expect(loc.state.changed).toEqual(['night', 'fontScale', 'density', 'animations', 'locale'])
    const on = (await both('ccm:dev-ui-set', 'R58M123', { key: 'animations', off: false }, null)) as { state: { changed: string[] } }
    expect(on.state.changed).not.toContain('animations')
    expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'adb-ui-originals.json'), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'ui.json'), 'utf8'))))
    expect(await both('ccm:dev-ui-set', 'R58M123', { key: 'fontScale', value: 9 }, null)).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-ui-set', 'R58M123', { key: 'night', value: 'yes' }, 'bad pkg')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:dev-ui-read', 'R58M123', 'bad pkg')).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:dev-ui-read', 'ZZ', null)).toEqual({ ok: false, error: 'not-connected' })
    const restored = (await both('ccm:dev-ui-restore', 'R58M123', null)) as { state: { changed: string[] } }
    expect(restored.state.changed).toEqual([])
    expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'adb-ui-originals.json'), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'ui.json'), 'utf8'))))
  })

  it('lists, starts and stops emulators', async () => {
    const list = (await both('ccm:avd-list')) as { ok: boolean; avds: { name: string; serial: string | null; api: number }[] }
    expect(list.avds.find((a) => a.name === 'Pixel_8')).toMatchObject({ serial: 'emulator-5554', api: 34 })
    expect(await both('ccm:avd-start', 'Pixel_8', 'normal')).toEqual({ ok: false, error: 'avd-running' })
    expect(await both('ccm:avd-start', 'Nope', 'normal')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:avd-start', 'Test_AVD', 'bad')).toEqual({ ok: false, error: 'invalid' })
    confirmAnswer = false
    expect(await both('ccm:avd-start', 'Test_AVD', 'wipe')).toEqual({ ok: false, error: 'cancelled' })
    confirmAnswer = true
    expect(await both('ccm:avd-start', 'Test_AVD', 'cold')).toEqual({ ok: true })
    expect(await both('ccm:avd-stop', 'Pixel_8')).toEqual({ ok: true })
    expect(await both('ccm:avd-stop', 'Test_AVD')).toEqual({ ok: false, error: 'avd-not-running' })
  }, 40_000)

  it('holds the tracker and runs a qr pairing to a cancel', async () => {
    expect(await both('ccm:dev-hold', true)).toBe(true)
    expect(await both('ccm:dev-hold', 'x')).toBe(false)
    const a = (await node.call('ccm:dev-qr-start', ev)) as { id: string; payload: string }
    const b = (await rust.call('ccm:dev-qr-start', ev)) as { id: string; payload: string }
    expect(a.payload).toMatch(/^WIFI:T:ADB;S:orrery-[0-9a-f]{8};P:\d{6};;$/)
    expect(b.payload).toMatch(/^WIFI:T:ADB;S:orrery-[0-9a-f]{8};P:\d{6};;$/)
    expect(await both('ccm:dev-qr-cancel', 'unknown')).toEqual({ ok: false, error: 'invalid' })
    expect(await node.call('ccm:dev-qr-cancel', ev, a.id)).toEqual({ ok: true })
    expect(await rust.call('ccm:dev-qr-cancel', ev, b.id)).toEqual({ ok: true })
    await sleep(300)
    const statuses = (events: unknown[], id: string) => events.filter((e) => (e as { type?: string; id?: string }).type === 'qr' && (e as { id?: string }).id === id).map((e) => (e as { status: string }).status)
    expect(statuses(twin.events.map((e) => e.payload), b.id)).toEqual(statuses(nodeEvents, a.id))
    expect(statuses(nodeEvents, a.id).at(-1)).toBe('cancelled')
    expect(await both('ccm:dev-hold', false)).toBe(true)
  })

  it('stayed on the rust path', () => {
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
