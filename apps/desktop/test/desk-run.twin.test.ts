import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { RunEvent } from '@ccm/shared'
import { runStandingHosts } from '../src/run/deskHosts'
import { createAndroidRunner } from '../src/run/android/runner'
import { createCustomRunner } from '../src/run/custom'
import { createFlutterRunner } from '../src/run/flutter/runner'
import { registerRunIpc } from '../src/run/ipc'
import { defaultSpawner } from '../src/run/proc'
import { RunManager } from '../src/run/sessions'
import type { DeviceService } from '../src/run/types'
import { fakeIpc, HAVE_DESK, listTree, startTwin, swapRoot, type Twin } from './desk-twin'

const posix = process.platform !== 'win32'
const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-run-')))
const write = (file: string, text: string): void => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}
const script = (file: string, body: string): void => {
  write(file, `#!/bin/sh\n${body}\n`)
  chmodSync(file, 0o755)
}

const FLUTTER = `case "$1" in
  --version) echo '{"frameworkVersion":"3.99.0","channel":"stable","dartSdkVersion":"3.9.0 (build x)"}';;
  doctor) case "$2" in --machine) echo '[{"name":"Flutter","type":"installed","statusInfo":"ok","messages":[{"message":"hi","type":"information"}]}]';; esac;;
  pub) if [ -f "$PWD/fail" ]; then echo "Error: resolving failed" >&2; exit 1; fi; echo got;;
esac`

function makeSide(name: string) {
  const root = path.join(tmp, name)
  const ws = path.join(root, 'ws')
  const app = path.join(ws, 'app')
  const sdk = path.join(root, 'sdk')
  const data = path.join(root, 'data')
  const other = path.join(root, 'other')
  mkdirSync(data, { recursive: true })
  mkdirSync(other, { recursive: true })
  write(path.join(app, 'pubspec.yaml'), 'name: demo\ndependencies:\n  flutter:\n    sdk: flutter\n')
  write(path.join(app, '.metadata'), 'project_type: app\n')
  write(path.join(app, 'lib', 'main.dart'), 'void main() {}\n')
  write(path.join(app, 'lib', 'main_dev.dart'), 'void main() {}\n')
  write(path.join(app, 'android', 'app', 'build.gradle'), 'android {\n  productFlavors {\n    dev { }\n    prod { }\n  }\n}\n')
  write(path.join(ws, '.run', 'Dev.run.xml'), '<component><configuration name="Dev &amp; Co" type="FlutterRunConfigurationType"><option name="filePath" value="$PROJECT_DIR$/app/lib/main_dev.dart" /><option name="buildFlavor" value="dev" /><env name="A" value="1" /><env name="PATH" value="x" /></configuration></component>')
  write(path.join(ws, '.vscode', 'launch.json'), '{"configurations": [{"type": "dart", "name": "go", "program": "app/lib/main.dart", "args": ["--a b"], "env": {"N": 3},}]}')
  script(path.join(sdk, 'bin', 'flutter'), FLUTTER)
  return { root, ws, app, sdk, data, other }
}

type Side = ReturnType<typeof makeSide>

function stubDevices(): DeviceService {
  return { list: async () => [], onChange: () => () => undefined, emulators: async () => [], launchEmulator: async () => ({ ok: true }), stop: () => undefined }
}

const norm = (v: unknown): unknown =>
  JSON.parse(JSON.stringify(v ?? null), (k, val: unknown) => {
    if (k === 'id' && typeof val === 'string' && /^[0-9a-f-]{36}$/.test(val)) return 'S'
    if (k === 'sessionId') return 'S'
    if (['startedAt', 'endedAt', 'at'].includes(k) && typeof val === 'number') return 0
    return val
  }) as unknown

const side = { node: makeSide('node'), rust: makeSide('rust') }
const sides = [side.node, side.rust]
let twin: Twin
let nodeEvents: RunEvent[] = []
const confirms: { node: string[]; rust: string[] } = { node: [], rust: [] }
let nodeCall: ReturnType<typeof fakeIpc>['call']
let rustCall: ReturnType<typeof fakeIpc>['call']
let manager: RunManager

const resolveFor = (s: Side) => async (p: unknown): Promise<string | null> => (typeof p === 'string' && p.startsWith(s.root) && existsSync(p) ? realpathSync.native(p) : null)
const event = { sender: { id: 1, once: () => undefined, on: () => undefined } }

async function both(channel: string, ...argsFor: ((s: Side) => unknown)[]): Promise<[unknown, unknown]> {
  const n = await nodeCall(channel, event, ...argsFor.map((f) => f(side.node)))
  const r = await rustCall(channel, event, ...argsFor.map((f) => f(side.rust)))
  return [n, swapRoot(r, side.rust.root, side.node.root)]
}

async function same(channel: string, ...argsFor: ((s: Side) => unknown)[]): Promise<unknown> {
  const [n, r] = await both(channel, ...argsFor)
  expect(norm(r)).toEqual(norm(n))
  return n
}

const waitEnded = async (events: () => unknown[], count: number): Promise<void> => {
  const ended = (): number => events().filter((e) => (e as { type?: string; session?: { state?: string } }).type === 'session' && ['stopped', 'failed', 'cancelled'].includes((e as { session: { state: string } }).session.state)).length
  for (let i = 0; i < 400 && ended() < count; i++) await new Promise((r) => setTimeout(r, 25))
  expect(ended()).toBeGreaterThanOrEqual(count)
}

const rustEvents = (): unknown[] => twin.events.filter((e) => e.event === 'ccm:run-event').map((e) => swapRoot(e.payload, side.rust.root, side.node.root))

describe.skipIf(process.env.CCM_DESK_RUN?.trim() === 'node' || !HAVE_DESK || !posix)('desk run twin', () => {
  beforeAll(async () => {
    twin = await startTwin(
      { CCM_DATA_DIR: side.rust.data },
      runStandingHosts({
        confirm: () => async (code, detail) => {
          confirms.rust.push(`${code}:${detail ?? ''}`)
          return true
        },
        onAdbUse: () => undefined,
      }),
    )
    manager = new RunManager({
      dataDir: side.node.data,
      env: process.env,
      platform: process.platform,
      home: os.homedir(),
      spawn: defaultSpawner,
      runners: { flutter: createFlutterRunner, android: createAndroidRunner, custom: createCustomRunner },
      devices: { flutter: stubDevices, adb: stubDevices },
      broadcast: (e) => nodeEvents.push(e),
      confirm: async (code, detail) => {
        confirms.node.push(`${code}:${detail ?? ''}`)
        return true
      },
    })
    await manager.loadSettings()
    const nodeIpc = fakeIpc()
    const rustIpc = fakeIpc()
    nodeCall = nodeIpc.call
    rustCall = rustIpc.call
    registerRunIpc({ ipcMain: nodeIpc.ipcMain, isAppFrame: () => true, manager, resolveDir: resolveFor(side.node) })
    registerRunIpc({ ipcMain: rustIpc.ipcMain, isAppFrame: () => true, manager, resolveDir: resolveFor(side.rust), desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('settings, templates and project sdk', async () => {
    expect(await same('ccm:run-settings')).toMatchObject({ layoutAutoCompile: true })
    const set = await both('ccm:run-settings-set', (s) => ({ flutterSdk: s.sdk, jdk: 'relative', layoutAutoCompile: false }))
    expect(norm(set[1])).toEqual(norm(set[0]))
    await same('ccm:run-settings-set', () => ({ androidSdk: null }))
    await same('ccm:run-project-sdk-set', (s) => s.app, (s) => s.sdk)
    await same('ccm:run-project-sdk-set', (s) => s.app, () => 'rel')
    await same('ccm:run-project-sdk-set', (s) => s.other, (s) => s.sdk)
    await same('ccm:run-settings')
    await same('ccm:run-templates')
    await same('ccm:run-templates-set', () => ({ flutter: { mode: 'profile', flavor: 'dev', args: '--x' }, android: { variant: 'bad variant' } }))
    await same('ccm:run-templates-set', () => 'nope')
    const read = (s: Side, f: string): unknown => JSON.parse(readFileSync(path.join(s.data, f), 'utf8'))
    expect(swapRoot(read(side.rust, 'run-settings.json'), side.rust.root, side.node.root)).toEqual(read(side.node, 'run-settings.json'))
    expect(JSON.parse(readFileSync(path.join(side.rust.data, 'run-templates.json'), 'utf8'))).toEqual(JSON.parse(readFileSync(path.join(side.node.data, 'run-templates.json'), 'utf8')))
  })

  it('detect, configs, save and delete', async () => {
    const det = (await same('ccm:run-detect', (s) => s.ws)) as { projects: { kind: string }[] }
    expect(det.projects[0]?.kind).toBe('flutter')
    const list = (await same('ccm:run-configs', (s) => s.ws)) as { id: string; name: string }[]
    expect(list.length).toBeGreaterThanOrEqual(3)
    await same('ccm:run-configs', (s) => s.other)
    const personal = (s: Side) => ({ id: 'mine', name: 'Mine', cwd: s.app, type: 'flutter', entrypoint: 'lib/main.dart', device: 'selected', mode: 'release', flavor: 'dev', args: '--a', env: { K: 'v' }, beforeLaunch: [' echo hi ', ''] })
    const shared = (s: Side) => ({ id: 'team', name: 'Team', cwd: s.app, shared: true, type: 'custom', command: 'echo hi', env: { PATH: 'x', K: 'v' } })
    await same('ccm:run-config-save', (s) => s.ws, personal)
    await same('ccm:run-config-save', (s) => s.ws, shared)
    expect(await same('ccm:run-config-save', (s) => s.ws, (s) => ({ ...personal(s), cwd: '/elsewhere' }))).toBeNull()
    await same('ccm:run-config-save', (s) => s.other, personal)
    await same('ccm:run-configs', (s) => s.ws)
    const treeN = listTree(path.join(side.node.ws, '.agent-monitor'))
    const treeR = listTree(path.join(side.rust.ws, '.agent-monitor'))
    expect(JSON.parse(JSON.stringify(swapRoot(treeR, side.rust.root, side.node.root)))).toEqual(treeN)
    const fileOf = (s: Side) => readdirSync(path.join(s.data, 'run-configs')).map((f) => JSON.parse(readFileSync(path.join(s.data, 'run-configs', f), 'utf8')) as unknown)
    expect(swapRoot(fileOf(side.rust), side.rust.root, side.node.root)).toEqual(fileOf(side.node))
    expect(await same('ccm:run-config-delete', (s) => s.ws, () => 'mine')).toBe(true)
    expect(await same('ccm:run-config-delete', (s) => s.ws, () => 'mine')).toBe(false)
    expect(await same('ccm:run-config-delete', (s) => s.ws, () => 'bad id')).toBe(false)
    await same('ccm:run-configs', (s) => s.ws)
  })

  it('import, auto import and denied roots', async () => {
    const imported = (await same('ccm:run-config-import', (s) => s.ws)) as unknown[]
    expect(imported.length).toBe(2)
    expect(await same('ccm:run-config-import', (s) => s.ws)).toEqual([])
    expect(await same('ccm:run-config-auto-import', (s) => s.ws)).toEqual([])
    expect(await same('ccm:run-configs', () => '/nowhere')).toEqual([])
    expect(await same('ccm:run-detect', () => '/nowhere')).toBeNull()
    expect(await same('ccm:run-env', () => 'relative', () => 'flutter')).toBeNull()
    expect(await same('ccm:run-launch-emulator', () => 'bad id', () => 'adb')).toMatchObject({ ok: false, error: { code: 'denied' } })
    expect(await same('ccm:run-launch-emulator', () => '-bad', () => 'adb')).toMatchObject({ ok: false, error: { code: 'emulator-missing' } })
    expect(await same('ccm:run-stop', () => 'bad id')).toMatchObject({ ok: false, error: { code: 'denied' } })
    expect(await same('ccm:run-stop', () => 'nope')).toMatchObject({ ok: false, error: { code: 'session-missing' } })
    expect(await same('ccm:run-reload', () => 'nope', () => true)).toMatchObject({ ok: false, error: { code: 'reload-unsupported' } })
    expect(await same('ccm:run-hold', (s) => s.ws, () => 'k', () => true)).toBe(true)
    expect(await same('ccm:run-hold', () => '/nowhere', () => 'k', () => true)).toBe(false)
    expect(await same('ccm:run-hold', (s) => s.ws, () => 'k', () => false)).toBe(true)
    expect(await same('ccm:run-sessions')).toEqual([])
  })

  it('preview and env', async () => {
    const cfg = (s: Side) => ({ id: 'p1', name: 'P', cwd: s.app, type: 'flutter', entrypoint: 'lib/missing.dart', device: 'selected', mode: 'debug', flavor: 'zzz', args: '--dart-define=K=${env:HOME} --dart-define-from-file=none.json', env: {}, beforeLaunch: [] })
    const p = (await same('ccm:run-preview', (s) => s.ws, cfg, () => null)) as { errors: { code: string }[] }
    expect(p.errors.map((e) => e.code)).toContain('entrypoint-missing')
    await same('ccm:run-preview', (s) => s.ws, cfg, () => 'R58')
    await same('ccm:run-preview', (s) => s.ws, (s) => ({ id: 'c1', name: 'C', cwd: s.ws, type: 'custom', command: 'echo "a b" ${projectDir}', env: {}, beforeLaunch: [] }), () => null)
    await same('ccm:run-preview', (s) => s.ws, (s) => ({ id: 'a1', name: 'A', cwd: s.ws, type: 'android', module: ':app', variant: 'freeDebug', device: 'selected', args: '-Px=1', env: {}, beforeLaunch: [] }), () => 'emu')
    await same('ccm:run-preview', (s) => s.ws, () => ({ nope: 1 }), () => null)
    const env = (await same('ccm:run-env', (s) => s.app, () => 'flutter')) as { version: string | null }
    expect(env.version).toBe('3.99.0')
  })

  it('toolchain and pub get', async () => {
    const tc = (await both('ccm:run-toolchain', (s) => s.app, () => 'flutter')) as [{ checks: unknown[]; at: number }, { checks: unknown[]; at: number }]
    expect(tc[1].checks).toEqual(tc[0].checks)
    expect(tc[0].checks.length).toBe(1)
    expect(await same('ccm:run-pub-get', (s) => s.app)).toEqual({ ok: true })
    for (const s of sides) write(path.join(s.app, 'fail'), '1')
    expect(await same('ccm:run-pub-get', (s) => s.app)).toMatchObject({ ok: false, error: { code: 'pub-get', detail: 'Error: resolving failed' } })
    expect(await same('ccm:run-pub-get', () => '/nowhere')).toMatchObject({ ok: false, error: { code: 'denied' } })
    for (const s of sides) rmSync(path.join(s.app, 'fail'))
  })

  it('runs a custom config: success, failure, stop and trust', async () => {
    const cfg = (s: Side, id: string, command: string, extra: object = {}) => ({ id, name: id, cwd: s.ws, type: 'custom', command, env: {}, beforeLaunch: [], ...extra })
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'ok', 'sh -c "echo hello; echo world"'))
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'bad', 'sh -c "echo nope; exit 3"'))
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'slow', 'sh -c "echo up; sleep 30"'))
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'pre', 'sh -c "echo main"', { beforeLaunch: ['sh -c "echo before"'] }))
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'missing', 'no-such-binary-xyz'))
    await both('ccm:run-config-save', (s) => s.ws, (s) => cfg(s, 'trust', 'sh -c "echo trusted"', { shared: true, env: { PATH: 'x', K: 'v' } }))
    nodeEvents = []
    twin.events.length = 0
    for (const id of ['ok', 'bad', 'pre', 'missing']) {
      const n = (await nodeCall('ccm:run-start', event, { root: side.node.ws, configId: id })) as { ok: boolean }
      const r = (await rustCall('ccm:run-start', event, { root: side.rust.ws, configId: id })) as { ok: boolean }
      expect(r.ok).toBe(n.ok)
    }
    await waitEnded(() => nodeEvents, 4)
    await waitEnded(rustEvents, 4)
    const group = (events: unknown[]): Record<string, unknown[]> => {
      const by: Record<string, unknown[]> = {}
      for (const e of events as { type: string; session?: { configId: string }; lines?: unknown[]; sessionId?: string }[]) {
        if (e.type === 'session') (by[e.session!.configId] ??= []).push(norm(e))
      }
      return by
    }
    const sessionNames = (events: unknown[]): Record<string, string> => {
      const m: Record<string, string> = {}
      for (const e of events as { type: string; session?: { id: string; configId: string } }[]) if (e.type === 'session') m[e.session!.id] = e.session!.configId
      return m
    }
    const logs = (events: unknown[]): Record<string, string[]> => {
      const names = sessionNames(events)
      const by: Record<string, string[]> = {}
      for (const e of events as { type: string; sessionId?: string; lines?: { text: string; stream: string }[] }[]) if (e.type === 'log') (by[names[e.sessionId!]!] ??= []).push(...e.lines!.map((l) => `${l.stream}:${l.text}`))
      return by
    }
    expect(group(rustEvents())).toEqual(group(nodeEvents))
    expect(logs(rustEvents())).toEqual(logs(nodeEvents))
    expect(logs(nodeEvents).ok).toEqual(['app:hello', 'app:world'])
    expect(group(nodeEvents).bad?.at(-1)).toMatchObject({ session: { state: 'failed', failure: { code: 'exit' } } })
    expect(group(nodeEvents).missing?.at(-1)).toMatchObject({ session: { failure: { code: 'spawn', detail: 'no-such-binary-xyz' } } })

    nodeEvents = []
    twin.events.length = 0
    const startSlow = async (call: typeof nodeCall, s: Side) => ((await call('ccm:run-start', event, { root: s.ws, configId: 'slow' })) as { sessionId: string }).sessionId
    const [nid, rid] = [await startSlow(nodeCall, side.node), await startSlow(rustCall, side.rust)]
    for (let i = 0; i < 200 && !logs(nodeEvents).slow; i++) await new Promise((r) => setTimeout(r, 25))
    for (let i = 0; i < 200 && !logs(rustEvents()).slow; i++) await new Promise((r) => setTimeout(r, 25))
    expect(await nodeCall('ccm:run-sessions', event)).toHaveLength(5)
    expect(((await rustCall('ccm:run-sessions', event)) as unknown[]).length).toBe(5)
    expect(await nodeCall('ccm:run-stop', event, nid)).toEqual({ ok: true })
    expect(await rustCall('ccm:run-stop', event, rid)).toEqual({ ok: true })
    await waitEnded(() => nodeEvents, 1)
    await waitEnded(rustEvents, 1)
    expect(group(rustEvents())).toEqual(group(nodeEvents))
    expect((group(nodeEvents).slow ?? []).map((e) => (e as { session: { state: string } }).session.state)).toEqual(['starting', 'running', 'stopping', 'stopped'])

    nodeEvents = []
    twin.events.length = 0
    expect(((await nodeCall('ccm:run-start', event, { root: side.node.ws, configId: 'trust' })) as { ok: boolean }).ok).toBe(true)
    expect(((await rustCall('ccm:run-start', event, { root: side.rust.ws, configId: 'trust' })) as { ok: boolean }).ok).toBe(true)
    await waitEnded(() => nodeEvents, 1)
    await waitEnded(rustEvents, 1)
    expect(group(rustEvents())).toEqual(group(nodeEvents))
    const plan = (list: string[]): unknown => JSON.parse(list.find((c) => c.startsWith('trust-config:'))!.slice('trust-config:'.length))
    expect(swapRoot(plan(confirms.rust), side.rust.root, side.node.root)).toEqual(plan(confirms.node))
    expect((plan(confirms.node) as { envKeys: string[] }).envKeys).toEqual(['K'])
    await nodeCall('ccm:run-start', event, { root: side.node.ws, configId: 'trust' })
    expect(confirms.node.filter((c) => c.startsWith('trust-config:'))).toHaveLength(1)
    expect(await nodeCall('ccm:run-start', event, { root: '/nowhere', configId: 'trust' })).toMatchObject({ ok: false, error: { code: 'denied' } })
    expect(await rustCall('ccm:run-start', event, { root: '/nowhere', configId: 'trust' })).toMatchObject({ ok: false, error: { code: 'denied' } })
    expect(await rustCall('ccm:run-start', event, { root: side.rust.ws, configId: 'ghost' })).toMatchObject({ ok: false, error: { code: 'config-missing' } })
    expect(await rustCall('ccm:run-start', event, { root: side.rust.ws, configId: 'ok-device' })).toMatchObject({ ok: false })
  }, 60_000)

  it('did not fall back to node', () => {
    expect(twin.host.channels).toContain('ccm:run-start')
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
