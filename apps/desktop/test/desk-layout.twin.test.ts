import { spawn } from 'node:child_process'
import { chmodSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerLayoutIpc, type LayoutIpc } from '../src/layout/ipc'
import { parseLooseKit } from '../src/layout/projectModel'
import { defaultSpawner } from '../src/run/proc'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-layout-')))
const FAKE = path.join(__dirname, 'fixtures', 'fake-layout-daemon.cjs')
const LAYOUT_XML = '<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"/>'
const DEVICE = { width: 360, height: 640, density: 160 }

interface Side {
  data: string
  ipc: ReturnType<typeof fakeIpc>
  api: LayoutIpc
  events: Array<{ type: string; state?: string; stage?: string }>
}

const write = (file: string, text: string): void => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}

const jdk = path.join(tmp, 'jdk')
const proj = path.join(tmp, 'proj')
const ll = path.join(tmp, 'll')
const jar = path.join(tmp, 'daemon.jar')
const layout = path.join(proj, 'app/src/main/res/layout/main.xml')
const drawable = path.join(proj, 'app/src/main/res/drawable/bg.xml')

const proj2 = path.join(tmp, 'proj2')
const proj3 = path.join(tmp, 'proj3')
const kitDir = path.join(tmp, 'kit')
const sdk = path.join(tmp, 'sdk')
const gradleHome = path.join(tmp, 'gradle-home')
const app2Layout = path.join(proj2, 'app/src/main/res/layout/main.xml')
const lib2Layout = path.join(proj2, 'lib/src/main/res/layout/lib.xml')
const app3Layout = path.join(proj3, 'app/src/main/res/layout/main.xml')

const resolveDir = async (p: unknown): Promise<string | null> => (typeof p === 'string' && [proj, proj2, proj3].some((r) => p.startsWith(r + path.sep)) ? p : null)
const server = { urlFor: async (file: string) => ({ url: `http://127.0.0.1:1/f/${path.basename(file)}` }) } as never
const APP = { app: true, sender: { id: 7 } }
const GUEST = { app: false, sender: { id: 8 } }
const gate = (e: unknown): boolean => (e as { app?: boolean }).app === true

const modelDeps = () => ({
  initScript: path.join(tmp, 'x.gradle'),
  looseKit: async () => parseLooseKit(readFileSync(path.join(kitDir, 'kit.json'), 'utf8'), kitDir),
  platform: process.platform,
  env: { PATH: process.env.PATH ?? '', GRADLE_USER_HOME: gradleHome },
  home: tmp,
  getSettings: () => ({ androidSdk: sdk, jdk }),
  spawn: defaultSpawner,
  kill: (child: { kill: (s: string) => void }) => child.kill('SIGKILL'),
  watchDirs: () => () => undefined,
})

const MODEL = (extraRes: boolean): string =>
  JSON.stringify({
    module: ':app',
    variant: 'debug',
    namespace: 'dev.t',
    applicationId: 'dev.t.app',
    resDirs: [path.join(proj2, 'app/src/main/res'), ...(extraRes ? [path.join(proj2, 'lib/src/main/res')] : [])],
    classpath: [path.join(proj2, 'app/build/classes')],
    rJar: path.join(proj2, 'app/build/R.jar'),
    aarRes: [],
    activityThemes: { 'dev.t.Main': '@style/MainTheme' },
    appTheme: '@style/AppTheme',
    projectDeps: [':lib'],
    warnings: ['w1'],
    minSdk: 24,
  })

const gradleScript = (json: string): string => `#!/bin/sh\nfor a in "$@"; do case "$a" in -Pccm.out=*) OUT="\${a#-Pccm.out=}";; esac; done\ncat > "$OUT" <<'EOF'\n${json}\nEOF\n`

function makeSide(name: string, desk: Twin | null, bundleReady: boolean, confirm: () => boolean, withModel = false): Side {
  const data = path.join(tmp, name)
  const events: Side['events'] = []
  const ipc = fakeIpc()
  const api = registerLayoutIpc({
    ipcMain: ipc.ipcMain,
    isAppFrame: (e) => gate(e),
    resolveDir,
    dataDir: data,
    getJdk: () => jdk,
    confirm: async () => confirm(),
    server,
    daemonJar: jar,
    spawn: (java, args, cwd) => spawn(java, args, { cwd, stdio: ['pipe', 'pipe', 'pipe'] }) as never,
    kill: (c) => (c as unknown as { kill: (s: string) => void }).kill('SIGKILL'),
    broadcast: (e) => events.push(e as never),
    platform: process.platform,
    layoutlibOverride: bundleReady ? ll : null,
    desk: desk ? { router: desk.router } : undefined,
    ...(withModel ? { model: modelDeps() } : {}),
  })
  return { data, ipc, api, events }
}

let twin: Twin
let node: Side
let rust: Side
let twinMissing: Twin
let nodeMissing: Side
let rustMissing: Side
let nodeModel: Side
let rustModel: Side
let twinModel: Twin

describe.skipIf(process.env.CCM_DESK_LAYOUT?.trim() === 'node' || !HAVE_DESK || process.platform === 'win32')('layout: TS handlers and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    write(path.join(jdk, 'bin/java'), `#!/bin/sh\ncase "$1" in -version) echo 'openjdk version "21.0.1"' >&2; exit 0;; esac\nexec "${process.execPath}" "${FAKE}"\n`)
    chmodSync(path.join(jdk, 'bin/java'), 0o755)
    write(path.join(ll, 'layoutlib-16.2.3.jar'), 'x')
    write(jar, 'j')
    write(layout, LAYOUT_XML)
    write(drawable, '<shape xmlns:android="http://schemas.android.com/apk/res/android"/>')
    write(path.join(proj, 'notes.txt'), 'x')
    for (const side of ['node', 'rust', 'node-missing', 'rust-missing', 'node-model', 'rust-model']) write(path.join(tmp, side, 'run-settings.json'), JSON.stringify({ jdk, androidSdk: sdk }))
    write(path.join(kitDir, 'a.jar'), 'x')
    write(path.join(kitDir, 'R.jar'), 'x')
    write(path.join(kitDir, 'kit.json'), JSON.stringify({ classpath: ['a.jar'], rJar: 'R.jar', packageName: 'dev.ccm.loosekit', appTheme: '@style/T', aarRes: [] }))
    write(path.join(sdk, 'platform-tools/adb'), 'x')
    write(path.join(gradleHome, 'caches/modules-2/files-2.1/g/a/1.0/abc/a-1.0.jar'), 'jar')
    write(path.join(proj2, 'settings.gradle.kts'), 'rootProject.name = "demo"\ninclude(":app", ":lib")\n')
    write(path.join(proj2, 'app/build.gradle.kts'), 'plugins { id("com.android.application") }\nandroid { namespace = "dev.t"\ndefaultConfig { minSdk = 24 } }\ndependencies {\n implementation(project(":lib"))\n implementation("g:a:1.0")\n}\n')
    write(path.join(proj2, 'app/src/main/AndroidManifest.xml'), '<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application android:theme="@style/AppTheme"><activity android:name=".Main" android:theme="@style/MainTheme"/></application></manifest>')
    write(app2Layout, LAYOUT_XML)
    write(lib2Layout, LAYOUT_XML)
    write(path.join(proj2, 'lib/build.gradle.kts'), 'plugins { id("com.android.library") }\n')
    write(path.join(proj2, 'app/build/classes/X.class'), 'c')
    write(path.join(proj2, 'app/build/R.jar'), 'r')
    write(path.join(proj2, 'gradlew'), gradleScript(MODEL(false)))
    chmodSync(path.join(proj2, 'gradlew'), 0o755)
    write(path.join(proj3, 'settings.gradle.kts'), 'include(":app")\n')
    write(path.join(proj3, 'app/build.gradle.kts'), 'plugins { id("com.android.application") }\nandroid { namespace = "dev.s"\ndefaultConfig { minSdk = 21\ntargetSdk = 33 } }\ndependencies { implementation("g:a:1.0") }\n')
    write(path.join(proj3, 'app/src/main/AndroidManifest.xml'), '<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application android:theme="@style/S"/></manifest>')
    write(app3Layout, LAYOUT_XML)
    const common = { CCM_LAYOUT_JAR: jar, CCM_LAYOUT_KIT: '', CCM_LAYOUT_LOG: path.join(tmp, 'layout.log') }
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust'), CCM_LAYOUT_OVERRIDE: ll, ...common })
    twinModel = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust-model'), CCM_LAYOUT_OVERRIDE: ll, CCM_LAYOUT_LOOSEKIT: kitDir, CCM_LAYOUT_INIT: path.join(tmp, 'x.gradle'), GRADLE_USER_HOME: gradleHome, ...common })
    twinMissing = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust-missing'), CCM_LAYOUT_OVERRIDE: '', ...common })
    node = makeSide('node', null, true, () => true)
    rust = makeSide('rust', twin, true, () => true)
    nodeMissing = makeSide('node-missing', null, false, () => false)
    rustMissing = makeSide('rust-missing', twinMissing, false, () => false)
    nodeModel = makeSide('node-model', null, true, () => true, true)
    rustModel = makeSide('rust-model', twinModel, true, () => true, true)
  }, 30_000)

  afterAll(async () => {
    await Promise.all([node?.api.stop(), rust?.api.stop(), nodeModel?.api.stop(), rustModel?.api.stop(), twin?.host.stop(), twinMissing?.host.stop(), twinModel?.host.stop()])
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = (v: unknown, s: Side, from: Side): unknown => swapRoot(v, from.data, s.data)

  const both = async (a: Side, b: Side, channel: string, event: unknown, ...args: unknown[]) => {
    const x = await a.ipc.call(channel, event, ...args)
    const y = same(await b.ipc.call(channel, event, ...args), a, b)
    expect(y).toEqual(x)
    return x as Record<string, unknown>
  }
  const go = (channel: string, event: unknown, ...args: unknown[]) => both(node, rust, channel, event, ...args)
  const stamps = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (k, x) => ((k === 'at' || k === 'since') && typeof x === 'number' ? 0 : x)))
  const modelBoth = async (channel: string, event: unknown, ...args: unknown[]) => {
    const x = stamps(await nodeModel.ipc.call(channel, event, ...args))
    const y = stamps(same(await rustModel.ipc.call(channel, event, ...args), nodeModel, rustModel))
    expect(y).toEqual(x)
    return x as Record<string, any>
  }
  const rustModelEvents = (): Array<Record<string, any>> => twinModel.events.filter((e) => e.event === 'ccm:layout-event').map((e) => e.payload as Record<string, any>)
  const modelTrail = (events: Array<Record<string, any>>): string[] => events.filter((e) => ['model', 'loading', 'project-changed'].includes(e.type)).map((e) => `${e.type}:${e.kind ?? e.state?.state ?? e.module}${e.active === undefined ? '' : e.active}`)

  it('status before the daemon runs, with and without the bundle', async () => {
    expect(await go('ccm:layout-status', APP)).toMatchObject({ daemon: 'stopped', jar: true, apiLevel: 36, jdk: { ok: true, version: 21 }, bundle: { state: 'ready', dir: ll } })
    expect(await go('ccm:layout-status', GUEST)).toBeNull()
    expect(await both(nodeMissing, rustMissing, 'ccm:layout-status', APP)).toMatchObject({ bundle: { state: 'missing' } })
    expect(await both(nodeMissing, rustMissing, 'ccm:layout-render', APP, { file: layout, device: DEVICE })).toEqual({ ok: false, code: 'bundle-missing' })
    expect(await both(nodeMissing, rustMissing, 'ccm:layout-download', APP)).toEqual({ state: 'cancelled' })
    expect(await both(nodeMissing, rustMissing, 'ccm:layout-warmup', APP, { file: layout })).toEqual({ ok: false, code: 'bundle-missing' })
  })

  it('render refuses bad requests the same way', async () => {
    expect(await go('ccm:layout-render', GUEST, { file: layout, device: DEVICE })).toEqual({ ok: false, code: 'denied' })
    expect(await go('ccm:layout-render', APP, { file: path.join(proj, 'notes.txt'), device: DEVICE })).toEqual({ ok: false, code: 'denied' })
    expect(await go('ccm:layout-render', APP, { file: '/etc/hosts', device: DEVICE })).toEqual({ ok: false, code: 'denied' })
    expect(await go('ccm:layout-render', APP, { file: layout, device: { width: 1 } })).toEqual({ ok: false, code: 'bad-params' })
    expect(await go('ccm:layout-render', APP, { file: layout, device: DEVICE, text: 'x'.repeat(1024 * 1024 + 1) })).toEqual({ ok: false, code: 'too-large' })
    expect(await go('ccm:layout-render', APP, { file: path.join(proj, 'app/src/main/res/layout/none.xml'), device: DEVICE })).toEqual({ ok: false, code: 'missing' })
    expect(await go('ccm:layout-render', APP, null)).toEqual({ ok: false, code: 'denied' })
  })

  it('render, cache hit, other text, daemon error', async () => {
    const first = await go('ccm:layout-render', APP, { file: layout, device: DEVICE, slot: 1 })
    expect(first).toMatchObject({ ok: true, cached: false, width: 360, height: 640, frameworkOnly: true, viewTree: { class: 'FrameLayout' }, diagnostics: [{ severity: 'warning', message: 'w' }] })
    expect(String(first.pngUrl)).toMatch(/^http:\/\/127\.0\.0\.1:1\/f\/[0-9a-f]{40}\.png\?r=[0-9a-f]{40}$/)
    expect(await go('ccm:layout-render', APP, { file: layout, device: DEVICE, slot: 1 })).toMatchObject({ cached: true, pngUrl: first.pngUrl })
    expect(await go('ccm:layout-render', APP, { file: layout, device: DEVICE, text: '<Other xmlns:android="http://schemas.android.com/apk/res/android"/>', night: true, scale: 0.4 })).toMatchObject({ ok: true, cached: false })
    expect(await go('ccm:layout-render', APP, { file: layout, device: DEVICE, text: `${LAYOUT_XML}<!--ERR-->` })).toEqual({ ok: false, code: 'render-failed', detail: 'boom' })
  })

  it('drawable render, warmup and download of a ready bundle', async () => {
    const d = await go('ccm:drawable-render', APP, { file: drawable, width: 24, height: 24, density: 320, states: ['pressed', 'x', 'pressed'] })
    expect(d).toMatchObject({ ok: true, kind: 'drawable', width: 48, swatches: [{ state: 'default', color: '#FF00FF00' }] })
    expect(await go('ccm:drawable-render', APP, { file: layout, width: 24, height: 24, density: 320 })).toEqual({ ok: false, code: 'not-drawable' })
    expect(await go('ccm:drawable-render', APP, { file: drawable, width: 0, height: 24, density: 320 })).toEqual({ ok: false, code: 'bad-params' })
    expect(await go('ccm:layout-warmup', APP, { file: layout })).toEqual({ ok: true })
    expect(await go('ccm:layout-warmup', APP, { file: '/etc/hosts' })).toEqual({ ok: false, code: 'denied' })
    expect(await go('ccm:layout-download', APP)).toMatchObject({ state: 'ready' })
    expect(await go('ccm:layout-status', APP)).toMatchObject({ daemon: 'ready' })
  })

  it('the agent render goes through the same daemon path', async () => {
    const a = await node.api.agentRender(layout, { device: DEVICE, night: false, slot: 'a' })
    const b = swapRoot(await rust.api.agentRender(layout, { device: DEVICE, night: false, slot: 'a' }), rust.data, node.data)
    expect(b).toEqual(a)
    expect(a).toMatchObject({ ok: true, frameworkOnly: true, width: 360 })
    expect(await rust.api.agentRender(path.join(proj, 'notes.txt'), { device: DEVICE, night: false, slot: 'a' })).toEqual({ ok: false, code: 'not-layout' })
  })

  it('the model service loads its kit through the Rust daemon and renders with it', async () => {
    const info = await both(nodeModel, rustModel, 'ccm:layout-model-status', APP, { file: layout })
    expect(info).toMatchObject({ target: null, hasModel: false })
    const a = await both(nodeModel, rustModel, 'ccm:layout-render', APP, { file: layout, device: DEVICE })
    expect(a).toMatchObject({ ok: true, frameworkOnly: true, builtinLibs: true })
    const loading = (events: Array<{ type?: string }>): string[] => events.filter((e) => e.type === 'loading').map((e) => JSON.stringify(e))
    const rustEvents = (): Array<{ type?: string }> => twinModel.events.filter((e) => e.event === 'ccm:layout-event').map((e) => e.payload as { type?: string })
    expect(loading(rustEvents())).toEqual(loading(nodeModel.events))
    expect(loading(rustEvents())).toEqual(['{"type":"loading","kind":"kit","active":true}', '{"type":"loading","kind":"kit","active":false}'])
    expect(await both(nodeModel, rustModel, 'ccm:layout-render', APP, { file: layout, device: DEVICE })).toMatchObject({ cached: true })
  })

  it('daemon events and the model service see the same states, and nothing fell back', async () => {
    await new Promise((r) => setTimeout(r, 200))
    const states = (s: Side): string[] => s.events.filter((e) => e.type === 'daemon' || e.type === 'daemon-stage').map((e) => `${e.type}:${e.state ?? e.stage}`)
    const fromRust = twin.events.filter((e) => e.event === 'ccm:layout-event').map((e) => e.payload as Side['events'][number])
    const got = fromRust.filter((e) => e.type === 'daemon' || e.type === 'daemon-stage').map((e) => `${e.type}:${e.state ?? e.stage}`)
    expect(got).toEqual(states(node))
    expect(got).toEqual(['daemon:starting', 'daemon-stage:jvm', 'daemon-stage:setup', 'daemon:ready'])
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
    expect(twinMissing.logs).toEqual([])
    expect(twinMissing.fellBack).toEqual([])
  })

  it('model channels: gate, library layout, refresh through the same Gradle command, render with the model', async () => {
    expect(await modelBoth('ccm:layout-model-status', GUEST, { file: app2Layout })).toBeNull()
    expect(await modelBoth('ccm:layout-model-status', APP, { file: path.join(proj, 'notes.txt') })).toBeNull()
    expect(await modelBoth('ccm:layout-model-status', APP, { file: '/etc/hosts' })).toBeNull()
    const idle = await modelBoth('ccm:layout-model-status', APP, { file: app2Layout })
    expect(idle).toMatchObject({ target: { root: proj2, module: ':app', name: 'demo' }, state: { state: 'none' }, hasModel: false, themes: [] })
    const lib = await modelBoth('ccm:layout-model-status', APP, { file: lib2Layout })
    expect(lib).toMatchObject({ target: { module: ':app' }, library: { module: ':lib', candidates: [':app'], linked: true } })
    expect(await modelBoth('ccm:layout-model-pick', APP, { file: lib2Layout, module: ':nope' })).toMatchObject({ target: { module: ':app' } })
    expect(await modelBoth('ccm:layout-model-pick', APP, { file: lib2Layout, module: 'x'.repeat(301) })).toBeNull()
    expect(await modelBoth('ccm:layout-model-pick', APP, { file: lib2Layout, module: ':app' })).toMatchObject({ target: { module: ':app' }, library: { linked: true } })

    const refreshed = await modelBoth('ccm:layout-model-refresh', APP, { file: app2Layout })
    expect(refreshed).toMatchObject({ state: { state: 'ready', resCount: 1, classpathCount: 1, warnings: 1 }, hasModel: true, themes: ['@style/AppTheme', '@style/MainTheme'], codeChanged: false })
    const plain = await modelBoth('ccm:layout-render', APP, { file: app2Layout, device: DEVICE })
    expect(plain).toMatchObject({ ok: true, frameworkOnly: false, builtinLibs: false, sourceOnly: false, cached: false })
    expect(await modelBoth('ccm:layout-render', APP, { file: app2Layout, device: DEVICE })).toMatchObject({ cached: true })
    write(path.join(proj2, 'gradlew'), gradleScript(MODEL(true)))
    expect(await modelBoth('ccm:layout-model-refresh', APP, { file: app2Layout })).toMatchObject({ state: { state: 'ready', resCount: 2 } })
    expect(await modelBoth('ccm:layout-render', APP, { file: app2Layout, device: DEVICE })).toMatchObject({ ok: true, cached: false, frameworkOnly: false })
    write(path.join(proj2, 'gradlew'), '#!/bin/sh\necho nope\nexit 1\n')
    const failed = await modelBoth('ccm:layout-model-refresh', APP, { file: lib2Layout })
    expect(failed).toMatchObject({ state: { state: 'error', code: 'gradle-failed', detail: 'nope\n' }, hasModel: true })
    await new Promise((r) => setTimeout(r, 200))
    expect(modelTrail(rustModelEvents())).toEqual(modelTrail(nodeModel.events as Array<Record<string, any>>))
  })

  it('model: the on-disk cache is shared between the TS and the Rust model', async () => {
    const hashFile = (data: string): string[] => readdirSync(path.join(data, 'layoutlib/models')).filter((f) => f.endsWith('.json') && f !== 'preferred-apps.json')
    expect(hashFile(nodeModel.data)).toEqual(hashFile(rustModel.data))
    const name = hashFile(rustModel.data)[0]!
    expect(readFileSync(path.join(rustModel.data, 'layoutlib/models', name), 'utf8')).toBe(readFileSync(path.join(nodeModel.data, 'layoutlib/models', name), 'utf8'))
    expect(readFileSync(path.join(rustModel.data, 'layoutlib/models/preferred-apps.json'), 'utf8')).toBe(readFileSync(path.join(nodeModel.data, 'layoutlib/models/preferred-apps.json'), 'utf8'))
  })

  it('model: no Gradle model reads res and cached libraries from sources', async () => {
    const info = await modelBoth('ccm:layout-model-status', APP, { file: app3Layout })
    expect(info).toMatchObject({ target: { module: ':app' }, state: { state: 'none' }, hasModel: false })
    const first = await modelBoth('ccm:layout-render', APP, { file: app3Layout, device: DEVICE })
    expect(first).toMatchObject({ ok: true, frameworkOnly: false, builtinLibs: true, sourceOnly: true, cached: false })
    expect(await modelBoth('ccm:layout-model-status', APP, { file: app3Layout })).toMatchObject({ themes: ['@style/S'] })
    write(path.join(proj3, 'app/build/intermediates/compile_and_runtime_not_namespaced_r_class_jar/debug/R.jar'), 'r')
    write(path.join(proj3, 'app/build/tmp/kotlin-classes/debug/X.class'), 'c')
    const stamp = new Date(1_700_000_000_000)
    utimesSync(path.join(proj3, 'app/build/intermediates/compile_and_runtime_not_namespaced_r_class_jar/debug/R.jar'), stamp, stamp)
    await new Promise((r) => setTimeout(r, 10_500))
    const second = await modelBoth('ccm:layout-render', APP, { file: app3Layout, device: DEVICE })
    expect(second).toMatchObject({ ok: true, sourceOnly: false, builtinLibs: true, prebuiltAt: 1_700_000_000_000 })
  }, 30_000)

  it('after a build the Rust model refreshes the app', async () => {
    write(path.join(proj2, 'gradlew'), gradleScript(MODEL(false)))
    rustModel.api.afterBuild(proj2, ':app', 'debug')
    nodeModel.api.afterBuild(proj2, ':app', 'debug')
    await new Promise((r) => setTimeout(r, 1500))
    expect(await modelBoth('ccm:layout-model-status', APP, { file: app2Layout })).toMatchObject({ state: { state: 'ready', resCount: 1 }, hasModel: true })
    expect(twinModel.logs).toEqual([])
    expect(twinModel.fellBack).toEqual([])
  })
})
