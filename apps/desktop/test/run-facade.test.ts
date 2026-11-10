import { mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import type { RunEvent, RunSession } from '@ccm/shared'
import { DeskRouter, type DeskClient } from '../src/desk/forward'
import type { HostHandlers } from '../src/desk/host'
import { RunFacade, RUN_SETTINGS_EVENT, RUN_STATE_EVENT } from '../src/run/facade'
import type { DeviceService } from '../src/run/types'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-run-facade-'))
afterAll(() => rmSync(tmp, { recursive: true, force: true }))

const stub = (): DeviceService => ({ list: async () => [], onChange: () => () => undefined, emulators: async () => [], launchEmulator: async () => ({ ok: true }), stop: () => undefined })

function setup(mode: 'rust' | 'node') {
  const calls: { channel: string; args: unknown[]; allowed: unknown }[] = []
  const replies: Record<string, unknown> = {}
  const client: DeskClient = {
    dead: null,
    call: async (channel: string, args: unknown[], hosts: HostHandlers) => {
      calls.push({ channel, args, allowed: await hosts['preview.allowed']?.({ path: 'x' }) })
      return replies[channel] ?? null
    },
  }
  const router = new DeskRouter({ client, env: { CCM_DESK_RUN: mode }, onFallback: () => undefined })
  const events: RunEvent[] = []
  const facade = new RunFacade(
    {
      dataDir: path.join(tmp, mode),
      env: {},
      platform: process.platform,
      home: os.homedir(),
      spawn: (() => {
        throw new Error('no spawn')
      }) as never,
      runners: {} as never,
      devices: { flutter: stub, adb: stub },
      broadcast: (e) => events.push(e),
      confirm: async () => false,
    },
    () => router,
  )
  return { facade, calls, replies, events }
}

const session = (id: string, state: RunSession['state'], endedAt: number | null = null): RunSession => ({
  id,
  configId: 'c',
  configName: 'C',
  type: 'custom',
  cwd: '/w',
  deviceId: null,
  deviceName: null,
  mode: null,
  state,
  startedAt: 1,
  endedAt,
  appId: null,
  vmServiceUri: null,
  devtoolsUri: null,
  failure: null,
  progress: null,
})

describe('RunFacade', () => {
  it('rust mode mirrors sessions from ccm:run-event and passes every event to the broadcast', () => {
    const { facade, events } = setup('rust')
    expect(facade.ingest('ccm:run-event', { type: 'session', session: session('a', 'starting') })).toBe(true)
    facade.ingest('ccm:run-event', { type: 'log', sessionId: 'a', lines: [{ stream: 'app', text: 'hi', at: 1 }] })
    facade.ingest('ccm:run-event', { type: 'session', session: session('a', 'running') })
    expect(facade.live().map((s) => s.state)).toEqual(['running'])
    facade.ingest('ccm:run-event', { type: 'session', session: session('a', 'stopped', 5) })
    expect(facade.live()).toEqual([])
    expect(facade.list().map((s) => s.state)).toEqual(['stopped'])
    expect(events.map((e) => e.type)).toEqual(['session', 'log', 'session', 'session'])
    expect(facade.ingest('ccm:other', {})).toBe(false)
  })

  it('keeps max 20 ended sessions, newest first, like the node manager', () => {
    const { facade } = setup('rust')
    for (let i = 0; i < 25; i++) facade.ingest('ccm:run-event', { type: 'session', session: session(`s${i}`, 'stopped', i) })
    facade.ingest('ccm:run-event', { type: 'session', session: session('live', 'running') })
    const ids = facade.list().map((s) => s.id)
    expect(ids).toHaveLength(21)
    expect(ids).toContain('live')
    expect(ids).not.toContain('s4')
    expect(ids).toContain('s5')
  })

  it('state events reach onSessionState listeners, a throwing listener does not stop the rest', () => {
    const { facade } = setup('rust')
    const seen: string[] = []
    facade.onSessionState(() => {
      throw new Error('boom')
    })
    const off = facade.onSessionState((s, cfg, device) => seen.push(`${s.state}:${cfg.id}:${device?.id ?? '-'}`))
    facade.ingest(RUN_STATE_EVENT, { session: session('a', 'running'), config: { id: 'c', type: 'android' }, device: { id: 'emu', platform: 'android-arm64' } })
    facade.ingest(RUN_STATE_EVENT, { session: session('a', 'stopped'), config: { id: 'c' }, device: null })
    facade.ingest(RUN_STATE_EVENT, { session: 'bad' })
    off()
    facade.ingest(RUN_STATE_EVENT, { session: session('a', 'failed'), config: { id: 'c' }, device: null })
    expect(seen).toEqual(['running:c:emu', 'stopped:c:-'])
  })

  it('settings come from the node load first, then from desk:run-settings', async () => {
    const { facade } = setup('rust')
    await facade.loadSettings()
    expect(facade.getSettings()).toEqual({ flutterSdk: null, androidSdk: null, jdk: null, flutterSdkProjects: {}, layoutAutoCompile: true })
    facade.ingest(RUN_SETTINGS_EVENT, { flutterSdk: '/f', androidSdk: '/a', jdk: null, flutterSdkProjects: { '/p': '/f2', bad: 3 }, layoutAutoCompile: false })
    const s = facade.getSettings()
    expect(s).toEqual({ flutterSdk: '/f', androidSdk: '/a', jdk: null, flutterSdkProjects: { '/p': '/f2' }, layoutAutoCompile: false })
    s.flutterSdkProjects['/x'] = '/y'
    expect(facade.getSettings().flutterSdkProjects).toEqual({ '/p': '/f2' })
    facade.ingest(RUN_SETTINGS_EVENT, 'junk')
    expect(facade.getSettings().androidSdk).toBe('/a')
  })

  it('rust mode sends configs, devices and start to the desk with the project as allowed root', async () => {
    const { facade, calls, replies } = setup('rust')
    replies['ccm:run-configs'] = [{ id: 'c' }]
    replies['ccm:run-start'] = { ok: true, sessionId: 's' }
    expect(await facade.configs('/proj')).toEqual([{ id: 'c' }])
    expect(await facade.devices()).toEqual([])
    expect(await facade.start({ root: '/proj', configId: 'c', deviceId: null })).toEqual({ ok: true, sessionId: 's' })
    replies['ccm:run-start'] = 'junk'
    expect(await facade.start({ root: '/proj', configId: 'c', deviceId: null })).toEqual({ ok: false, error: { code: 'spawn' } })
    facade.killAll()
    await new Promise((r) => setTimeout(r, 0))
    expect(calls.map((c) => c.channel)).toEqual(['ccm:run-configs', 'ccm:run-devices', 'ccm:run-start', 'ccm:run-start'])
    expect(calls[0]!.allowed).toEqual({ real: '/proj', roots: [] })
    expect(calls[2]!.args).toEqual([{ root: '/proj', configId: 'c', deviceId: null }])
  })

  it('node mode uses the node manager and never calls the desk', async () => {
    const { facade, calls } = setup('node')
    facade.ingest(RUN_SETTINGS_EVENT, { flutterSdk: '/f' })
    facade.ingest('ccm:run-event', { type: 'session', session: session('a', 'running') })
    expect(facade.getSettings().flutterSdk).toBeNull()
    expect(facade.list()).toEqual([])
    expect(await facade.configs(path.join(tmp, 'none'))).toEqual([])
    facade.killAll()
    expect(calls).toEqual([])
  })
})
