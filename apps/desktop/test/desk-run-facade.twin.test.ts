import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { RunConfig, RunDevice, RunEvent, RunSession } from '@ccm/shared'
import { createAndroidRunner } from '../src/run/android/runner'
import { createCustomRunner } from '../src/run/custom'
import { runStandingHosts } from '../src/run/deskHosts'
import { RunFacade } from '../src/run/facade'
import { createFlutterRunner } from '../src/run/flutter/runner'
import { registerRunIpc } from '../src/run/ipc'
import { defaultSpawner } from '../src/run/proc'
import type { DeviceService } from '../src/run/types'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const posix = process.platform !== 'win32'
const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-run-facade-')))

function makeSide(name: string) {
  const root = path.join(tmp, name)
  const ws = path.join(root, 'ws')
  const data = path.join(root, 'data')
  const sdk = path.join(root, 'sdk')
  for (const d of [ws, data, sdk]) mkdirSync(d, { recursive: true })
  writeFileSync(path.join(ws, 'README.md'), 'x\n')
  return { root, ws, data, sdk }
}

type Side = ReturnType<typeof makeSide>

interface Seen {
  events: RunEvent[]
  states: { session: RunSession; config: RunConfig; device: RunDevice | null }[]
  logs: Map<string, string[]>
}

const seen = (): Seen => ({ events: [], states: [], logs: new Map() })

function stubDevices(): DeviceService {
  return { list: async () => [], onChange: () => () => undefined, emulators: async () => [], launchEmulator: async () => ({ ok: true }), stop: () => undefined }
}

function facadeFor(s: Side, rec: Seen, desk: () => Twin['router'] | null): RunFacade {
  return new RunFacade(
    {
      dataDir: s.data,
      env: process.env,
      platform: process.platform,
      home: os.homedir(),
      spawn: defaultSpawner,
      runners: { flutter: createFlutterRunner, android: createAndroidRunner, custom: createCustomRunner },
      devices: { flutter: stubDevices, adb: stubDevices },
      broadcast: (e) => {
        rec.events.push(e)
        if (e.type === 'log') rec.logs.set(e.sessionId, [...(rec.logs.get(e.sessionId) ?? []), ...e.lines.map((l) => l.text)].slice(-200))
      },
      confirm: async () => true,
    },
    desk,
  )
}

const norm = (v: unknown): unknown =>
  JSON.parse(JSON.stringify(v ?? null), (k, val: unknown) => {
    if ((k === 'id' || k === 'sessionId') && typeof val === 'string' && /^[0-9a-f-]{36}$/.test(val)) return 'S'
    if (['startedAt', 'endedAt', 'at'].includes(k) && typeof val === 'number') return 0
    return val
  }) as unknown

const side = { node: makeSide('node'), rust: makeSide('rust') }
const rec = { node: seen(), rust: seen() }
let twin: Twin
let nodeFacade: RunFacade
let rustFacade: RunFacade
let nodeCall: ReturnType<typeof fakeIpc>['call']
let rustCall: ReturnType<typeof fakeIpc>['call']
const event = { sender: { id: 1, once: () => undefined, on: () => undefined } }
const resolveFor = (s: Side) => async (p: unknown): Promise<string | null> => (typeof p === 'string' && p.startsWith(s.root) && existsSync(p) ? realpathSync.native(p) : null)
const fromRust = (v: unknown): unknown => norm(swapRoot(v, side.rust.root, side.node.root))

const waitFor = async (check: () => boolean): Promise<void> => {
  for (let i = 0; i < 400 && !check(); i++) await new Promise((r) => setTimeout(r, 25))
  expect(check()).toBe(true)
}

const endedCount = (f: RunFacade): number => f.list().filter((s) => ['stopped', 'failed', 'cancelled'].includes(s.state)).length
const logsByConfig = (f: RunFacade, r: Seen): Record<string, string[]> => Object.fromEntries(f.list().map((s) => [s.configId, r.logs.get(s.id) ?? []]))
const sorted = (v: unknown): string[] => (v as unknown[]).map((x) => JSON.stringify(x)).sort()
const transitions = (r: Seen): string[] => r.states.map((t) => `${t.config.id}:${t.session.state}:${t.config.type}:${t.device?.id ?? '-'}`)

describe.skipIf(!HAVE_DESK || !posix)('run facade: consumers see the same state in node and rust mode', () => {
  beforeAll(async () => {
    twin = await startTwin({ CCM_DATA_DIR: side.rust.data }, runStandingHosts({ confirm: () => async () => true, onAdbUse: () => undefined }), (e, p) => rustFacade?.ingest(e, p))
    nodeFacade = facadeFor(side.node, rec.node, () => null)
    rustFacade = facadeFor(side.rust, rec.rust, () => twin.router)
    for (const [f, r] of [[nodeFacade, rec.node], [rustFacade, rec.rust]] as const) {
      await f.loadSettings()
      f.onSessionState((session, config, device) => r.states.push({ session, config, device }))
    }
    const n = fakeIpc()
    const r = fakeIpc()
    nodeCall = n.call
    rustCall = r.call
    registerRunIpc({ ipcMain: n.ipcMain, isAppFrame: () => true, manager: nodeFacade.node, resolveDir: resolveFor(side.node) })
    registerRunIpc({ ipcMain: r.ipcMain, isAppFrame: () => true, manager: rustFacade.node, resolveDir: resolveFor(side.rust), desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    nodeFacade?.killAll()
    rustFacade?.killAll()
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('settings changed through the page reach getSettings', async () => {
    expect(fromRust(rustFacade.getSettings())).toEqual(norm(nodeFacade.getSettings()))
    await nodeCall('ccm:run-settings-set', event, { flutterSdk: side.node.sdk, jdk: '/jdk/home', layoutAutoCompile: false })
    await rustCall('ccm:run-settings-set', event, { flutterSdk: side.rust.sdk, jdk: '/jdk/home', layoutAutoCompile: false })
    await waitFor(() => rustFacade.getSettings().jdk === '/jdk/home')
    expect(fromRust(rustFacade.getSettings())).toEqual(norm(nodeFacade.getSettings()))
    await nodeCall('ccm:run-project-sdk-set', event, side.node.ws, side.node.sdk)
    await rustCall('ccm:run-project-sdk-set', event, side.rust.ws, side.rust.sdk)
    await waitFor(() => Object.keys(rustFacade.getSettings().flutterSdkProjects).length === 1)
    expect(fromRust(rustFacade.getSettings())).toEqual(norm(nodeFacade.getSettings()))
    await nodeCall('ccm:run-settings-set', event, { androidSdk: '/android/sdk', jdk: null })
    await rustCall('ccm:run-settings-set', event, { androidSdk: '/android/sdk', jdk: null })
    await waitFor(() => rustFacade.getSettings().androidSdk === '/android/sdk')
    expect(fromRust(rustFacade.getSettings())).toEqual(norm(nodeFacade.getSettings()))
    expect(nodeFacade.getSettings()).toMatchObject({ androidSdk: '/android/sdk', jdk: null, layoutAutoCompile: false })
  })

  it('configs, start, output, exit and stop reach list, live, onSessionState and the broadcast', async () => {
    const cfg = (s: Side, id: string, command: string) => ({ id, name: id, cwd: s.ws, type: 'custom', command, env: { K: 'v' }, beforeLaunch: [] })
    for (const [id, command] of [['ok', 'sh -c "echo hello; echo world"'], ['bad', 'sh -c "echo nope; exit 3"'], ['slow', 'sh -c "echo up; sleep 30"']]) {
      await nodeCall('ccm:run-config-save', event, side.node.ws, cfg(side.node, id!, command!))
      await rustCall('ccm:run-config-save', event, side.rust.ws, cfg(side.rust, id!, command!))
    }
    const nodeConfigs = await nodeFacade.configs(side.node.ws)
    expect(nodeConfigs.map((c) => c.id).sort()).toEqual(['bad', 'ok', 'slow'])
    expect(fromRust(await rustFacade.configs(side.rust.ws))).toEqual(norm(nodeConfigs))

    for (const id of ['ok', 'bad']) {
      const n = await nodeFacade.start({ root: side.node.ws, configId: id, deviceId: null })
      const r = await rustFacade.start({ root: side.rust.ws, configId: id, deviceId: null })
      expect(n.ok).toBe(true)
      expect(r.ok).toBe(true)
    }
    expect(await rustFacade.start({ root: side.rust.ws, configId: 'ghost', deviceId: null })).toEqual(await nodeFacade.start({ root: side.node.ws, configId: 'ghost', deviceId: null }))
    await waitFor(() => endedCount(nodeFacade) === 2 && endedCount(rustFacade) === 2)
    await waitFor(() => transitions(rec.rust).length === transitions(rec.node).length)
    expect(sorted(fromRust(rustFacade.list()))).toEqual(sorted(norm(nodeFacade.list())))
    expect(rustFacade.live()).toEqual([])
    expect(logsByConfig(rustFacade, rec.rust)).toEqual(logsByConfig(nodeFacade, rec.node))
    expect(logsByConfig(nodeFacade, rec.node).ok).toEqual(['hello', 'world'])
    expect(sorted(fromRust(rec.rust.states))).toEqual(sorted(norm(rec.node.states)))
    expect([...transitions(rec.node)].sort()).toEqual(['bad:failed:custom:-', 'bad:running:custom:-', 'ok:running:custom:-', 'ok:stopped:custom:-'])

    const startSlow = async (call: typeof nodeCall, s: Side): Promise<string> => ((await call('ccm:run-start', event, { root: s.ws, configId: 'slow' })) as { sessionId: string }).sessionId
    const nid = await startSlow(nodeCall, side.node)
    const rid = await startSlow(rustCall, side.rust)
    await waitFor(() => nodeFacade.live().some((s) => s.state === 'running') && rustFacade.live().some((s) => s.state === 'running'))
    expect(fromRust(rustFacade.live())).toEqual(norm(nodeFacade.live()))
    expect(rustFacade.live().map((s) => s.configName)).toEqual(['slow'])
    expect(await nodeCall('ccm:run-stop', event, nid)).toEqual({ ok: true })
    expect(await rustCall('ccm:run-stop', event, rid)).toEqual({ ok: true })
    await waitFor(() => endedCount(nodeFacade) === 3 && endedCount(rustFacade) === 3)
    await waitFor(() => transitions(rec.rust).length === transitions(rec.node).length)
    expect(sorted(fromRust(rec.rust.states))).toEqual(sorted(norm(rec.node.states)))
    expect(transitions(rec.node).filter((t) => t.startsWith('slow:'))).toEqual(['slow:running:custom:-', 'slow:stopping:custom:-', 'slow:stopped:custom:-'])
    expect(sorted(fromRust(rustFacade.list()))).toEqual(sorted(norm(nodeFacade.list())))
    expect(rustFacade.live()).toEqual([])

    const sessionEvents = (r: Seen, id: string): string[] => r.events.flatMap((e) => (e.type === 'session' && e.session.configId === id ? [e.session.state] : []))
    for (const id of ['ok', 'bad', 'slow']) expect(sessionEvents(rec.rust, id)).toEqual(sessionEvents(rec.node, id))
  }, 60_000)

  it('did not fall back to node', () => {
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
