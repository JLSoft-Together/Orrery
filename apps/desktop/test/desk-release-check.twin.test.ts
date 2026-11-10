import { chmodSync, cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerReleaseIpc } from '../src/releaseCheck/ipc'
import { findSignTools } from '../src/releaseCheck/sign'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const FIXTURE = path.resolve(__dirname, '../e2e/fixtures/release-check')
const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-release-')))
const proj = path.join(tmp, 'proj')
const jdk = path.join(tmp, 'jdk')
const sdk = path.join(tmp, 'sdk')
const dataNode = path.join(tmp, 'data-node')
const dataRust = path.join(tmp, 'data-rust')
const SHA = '11'.repeat(32)
const SHA_DEBUG = 'ee'.repeat(32)
const KEY_SHA = Array.from({ length: 32 }, () => 'AB').join(':')
let picked: string | null = null

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const ev = {} as never

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"checkedAt":\d+/g, '"checkedAt":1').replace(/"at":\d+/g, '"at":1'))

const put = (rel: string, text: string): string => {
  const file = path.join(proj, rel)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
  return file
}

const script = (file: string, body: string) => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, `#!/bin/sh\n${body}\n`)
  chmodSync(file, 0o755)
}

describe.skipIf(!HAVE_DESK || process.platform === 'win32')('release check: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of [proj, dataNode, dataRust]) mkdirSync(d, { recursive: true })
    cpSync(FIXTURE, path.join(proj, 'shop'), { recursive: true })
    put('shop/app/build/outputs/apk/release/app-release.apk', 'x')
    put('flut/pubspec.yaml', 'name: flut_app\nversion: 1.4.0+12\ndependencies:\n  flutter:\n    sdk: flutter\n  google_mobile_ads: ^5.0.0\n')
    put('flut/lib/main.dart', "import 'a.dart';\nconst id = 'ca-app-pub-3940256099942544/1033173712';\n")
    put('flut/android/app/build.gradle', "android {\n compileSdk 34\n defaultConfig {\n  targetSdk 33\n  minSdkVersion flutter.minSdkVersion\n }\n buildTypes {\n  release {\n   signingConfig signingConfigs.debug\n  }\n }\n}\n")
    put('flut/android/app/src/main/AndroidManifest.xml', '<manifest><uses-permission android:name="android.permission.READ_SMS"/><application android:usesCleartextTraffic="true"><service android:name=".S" android:exported="true"/></application></manifest>')
    put('flut/build/app/outputs/flutter-apk/app-debug.apk', 'x')
    put('flut/build/app/outputs/bundle/release/app-release.aab', 'x')
    script(path.join(jdk, 'bin', 'java'), `if [ "$1" = "-version" ]; then echo 'openjdk version "21.0.1" 2023-10-17' 1>&2; exit 0; fi\ncase "$5" in *debug*) echo "Signer #1 certificate DN: CN=Android Debug, O=Android, C=US"; echo "Signer #1 certificate SHA-256 digest: ${SHA_DEBUG}";; *) echo "Signer #1 certificate DN: CN=Shop, O=Shop"; echo "Signer #1 certificate SHA-256 digest: ${SHA}";; esac`)
    script(path.join(jdk, 'bin', 'keytool'), `echo "Owner: CN=Bundle Shop, O=Shop"\necho "Certificate fingerprints:"\necho "	 SHA256: ${KEY_SHA}"`)
    mkdirSync(path.join(sdk, 'build-tools', '34.0.0', 'lib'), { recursive: true })
    mkdirSync(path.join(sdk, 'build-tools', '35.0.1', 'lib'), { recursive: true })
    writeFileSync(path.join(sdk, 'build-tools', '34.0.0', 'lib', 'apksigner.jar'), 'x')
    writeFileSync(path.join(sdk, 'build-tools', '35.0.1', 'lib', 'apksigner.jar'), 'x')
    twin = await startTwin({ CCM_DATA_DIR: dataRust, JAVA_HOME: jdk, ANDROID_HOME: sdk })
    const make = (ipc: ReturnType<typeof fakeIpc>, dataDir: string, withDesk: boolean) =>
      registerReleaseIpc({
        ipcMain: ipc.ipcMain,
        isAppFrame: () => true,
        resolveDir: async (d) => (d === proj ? proj : null),
        dataDir,
        pickArtifact: async () => picked,
        findTools: () => findSignTools({ jdkSetting: null, sdkSetting: null, env: { JAVA_HOME: jdk, ANDROID_HOME: sdk, PATH: '' }, platform: process.platform, home: tmp }),
        desk: withDesk ? twin.router : undefined,
      })
    make(node, dataNode, false)
    make(rust, dataRust, true)
  })

  afterAll(async () => {
    await twin.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, ...args: unknown[]) => {
    const a = norm(await node.call(channel, ev, ...args))
    const b = norm(await rust.call(channel, ev, ...args))
    expect(b).toEqual(a)
    return a as Record<string, unknown>
  }

  const appsOf = (r: Record<string, unknown>) => r.apps as { dir: string; kind: string; findings: { rule: string }[] }[]

  it('scans android and flutter apps with the same findings', async () => {
    const r = await both('ccm:release-check', proj)
    expect(r.ok).toBe(true)
    const apps = appsOf(r)
    expect(apps.map((a) => a.kind).sort()).toEqual(['android', 'flutter'])
    expect(apps.flatMap((a) => a.findings.map((f) => f.rule))).toEqual(expect.arrayContaining(['version-code', 'admob-test', 'target-sdk', 'signing', 'cert']))
  })

  it('refuses bad roots the same way', async () => {
    expect(await both('ccm:release-check', 'relative')).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:release-check', path.join(proj, '..'))).toEqual({ ok: false, error: 'denied' })
    expect(await both('ccm:release-check', 12)).toEqual({ ok: false, error: 'invalid' })
    const empty = path.join(proj, 'shop', 'nothing')
    mkdirSync(empty, { recursive: true })
    expect(await both('ccm:release-mark', { root: proj, app: path.join(proj, 'nope') })).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:release-mark', {})).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:release-verify', { root: proj, app: path.join(proj, 'nope') })).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:release-verify', { root: 'rel', app: '/x' })).toEqual({ ok: false, error: 'invalid' })
    expect(await both('ccm:release-verify', { root: path.join(tmp, 'else'), app: path.join(tmp, 'else', 'a') })).toEqual({ ok: false, error: 'denied' })
  })

  it('marks a release, verifies the signer, remembers the key', async () => {
    const first = appsOf(await both('ccm:release-check', proj))
    const shop = first.find((a) => a.kind === 'android')!.dir
    const flut = first.find((a) => a.kind === 'flutter')!.dir
    const marked = await both('ccm:release-mark', { root: proj, app: shop })
    expect(marked.ok).toBe(true)
    await both('ccm:release-mark', { root: proj, app: flut })
    expect(await both('ccm:release-remember', { root: proj, app: shop })).toEqual({ ok: false, error: 'invalid' })
    const v = await both('ccm:release-verify', { root: proj, app: shop })
    expect(v).toMatchObject({ ok: true, artifact: 'apk', debugKey: false, finding: { detail: 'notRemembered' } })
    const checked = await both('ccm:release-check', proj)
    expect(JSON.stringify(checked)).toContain('notRemembered')
    const remembered = await both('ccm:release-remember', { root: proj, app: shop })
    expect(remembered.ok).toBe(true)
    const after = await both('ccm:release-verify', { root: proj, app: shop })
    expect(after).toMatchObject({ ok: true, finding: { detail: 'match' } })
    const dbg = await both('ccm:release-verify', { root: proj, app: flut })
    expect(dbg).toMatchObject({ ok: true })
    picked = path.join(proj, 'flut', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk')
    const dbgPick = await both('ccm:release-verify', { root: proj, app: flut, pick: true })
    expect(dbgPick).toMatchObject({ ok: true, debugKey: true, finding: { detail: 'debugKey' } })
    expect(await both('ccm:release-remember', { root: proj, app: flut })).toEqual({ ok: false, error: 'invalid' })
    picked = path.join(proj, 'flut', 'build', 'app', 'outputs', 'bundle', 'release', 'app-release.aab')
    expect(await both('ccm:release-verify', { root: proj, app: flut, pick: true })).toMatchObject({ ok: true, artifact: 'aab', sha256: KEY_SHA })
    picked = null
    expect(await both('ccm:release-verify', { root: proj, app: flut, pick: true })).toEqual({ ok: false, error: 'cancelled' })
    picked = path.join(proj, 'flut', 'pubspec.yaml')
    expect(await both('ccm:release-verify', { root: proj, app: flut, pick: true })).toEqual({ ok: false, error: 'invalid' })
    const files = readdirSync(path.join(dataRust, 'release-check'))
    expect(files).toEqual(readdirSync(path.join(dataNode, 'release-check')))
    for (const f of files) expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'release-check', f), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'release-check', f), 'utf8'))))
  })

  it('stayed on the rust path', () => {
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
