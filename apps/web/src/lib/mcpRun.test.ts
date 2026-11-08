import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { RunConfig, RunDevice, RunSession } from '@ccm/shared'

const bridge = vi.hoisted(() => ({
  runSessions: vi.fn(),
  runStart: vi.fn(),
  runStop: vi.fn(),
  runReload: vi.fn(),
  runConfigs: vi.fn(),
  runDevices: vi.fn(),
}))

vi.mock('../lib/devices', () => ({ devSnapshot: () => Promise.resolve(null), devConnectMany: () => Promise.resolve([]) }))
vi.mock('../lib/desktop', () => ({
  desktop: {
    platform: 'win32',
    onRunEvent: () => () => {},
    runSessions: () => bridge.runSessions(),
    runStart: (req: unknown) => bridge.runStart(req),
    runStop: (id: string) => bridge.runStop(id),
    runReload: (id: string, full: boolean) => bridge.runReload(id, full),
    runConfigs: (root: string) => bridge.runConfigs(root),
    runDevices: () => bridge.runDevices(),
  },
  hasRun: true,
}))

import { useRunStore } from '../stores/run'
import { createRunCalls, isErrorLine, RUN_REPLY_MS } from './mcpRun'

const ROOT = 'C:\\work\\app'
const cfg = (id: string, device = 'selected'): RunConfig => ({ id, name: `cfg ${id}`, cwd: ROOT, shared: false, env: {}, beforeLaunch: [], type: 'android', module: ':app', variant: 'debug', device, args: '' })
const dev = (id: string, state: RunDevice['state'] = 'ready'): RunDevice => ({ id, name: `Phone ${id}`, platform: 'android', emulator: false, state, source: 'adb' })
const session = (over: Partial<RunSession> = {}): RunSession => ({
  id: 's1', configId: 'a', configName: 'cfg a', type: 'android', cwd: ROOT, deviceId: 'P1', deviceName: 'Phone P1', mode: null, state: 'running',
  startedAt: 10, endedAt: null, appId: 'com.x', vmServiceUri: null, devtoolsUri: null, failure: null, progress: null, ...over,
})
const base = { id: 'q', from: 'pane', cwd: ROOT }

describe('MCP run calls (page side)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    bridge.runSessions.mockReset().mockResolvedValue([])
    bridge.runStart.mockReset().mockResolvedValue({ ok: true, sessionId: 'n1' })
    bridge.runStop.mockReset().mockResolvedValue({ ok: true })
    bridge.runReload.mockReset().mockResolvedValue({ ok: true })
    bridge.runConfigs.mockReset().mockResolvedValue([cfg('a'), cfg('b', 'P2')])
    bridge.runDevices.mockReset().mockResolvedValue([dev('P1'), dev('P2')])
  })

  it('starts the selected config on the selected device, or an asked config / device from the lists only', async () => {
    const calls = createRunCalls(useRunStore())
    await calls.app({ ...base, tool: 'run_app', action: 'start' })
    expect(bridge.runStart).toHaveBeenLastCalledWith({ root: ROOT, configId: 'a', deviceId: 'P1' })
    await calls.app({ ...base, tool: 'run_app', action: 'start', config: 'a', device: 'P2' })
    expect(bridge.runStart).toHaveBeenLastCalledWith({ root: ROOT, configId: 'a', deviceId: 'P2' })
    expect(await calls.app({ ...base, tool: 'run_app', action: 'start', config: 'zzz' })).toMatchObject({ ok: true, refused: { code: 'config' } })
    expect(await calls.app({ ...base, tool: 'run_app', action: 'start', config: 'a', device: 'evil' })).toMatchObject({ refused: { code: 'device' } })
    // b pins P2: another device is refused
    expect(await calls.app({ ...base, tool: 'run_app', action: 'start', config: 'b', device: 'P1' })).toMatchObject({ refused: { code: 'device' } })
    expect(bridge.runStart).toHaveBeenCalledTimes(2)
  })

  it('does not change the Run bar pick and refuses start of a pair already running', async () => {
    const run = useRunStore()
    const calls = createRunCalls(run)
    run.handleEvent({ type: 'session', session: session() })
    expect(await calls.app({ ...base, tool: 'run_app', action: 'start' })).toMatchObject({ refused: { code: 'running' }, session: { id: 's1', state: 'running' } })
    await calls.app({ ...base, tool: 'run_app', action: 'restart', config: 'a', device: 'P2' })
    expect(run.selected).toEqual({})
  })

  it('stop / reload act on the live run of the project; nothing running is a refusal', async () => {
    const run = useRunStore()
    const calls = createRunCalls(run)
    expect(await calls.app({ ...base, tool: 'run_app', action: 'stop' })).toMatchObject({ refused: { code: 'no-session' } })
    run.handleEvent({ type: 'session', session: session() })
    await calls.app({ ...base, tool: 'run_app', action: 'stop' })
    expect(bridge.runStop).toHaveBeenCalledWith('s1')
    await calls.app({ ...base, tool: 'run_app', action: 'reload', full: true })
    expect(bridge.runReload).toHaveBeenCalledWith('s1', true)
    run.handleEvent({ type: 'session', session: session({ id: 's2', state: 'building', startedAt: 20 }) })
    expect(await calls.app({ ...base, tool: 'run_app', action: 'reload' })).toMatchObject({ refused: { code: 'not-running' } })
    bridge.runStop.mockResolvedValue({ ok: false, error: { code: 'session-missing' } })
    expect(await calls.app({ ...base, tool: 'run_app', action: 'stop' })).toMatchObject({ refused: { code: 'session-missing' } })
  })

  it('answers waiting when the start takes longer than main lets the page reply', async () => {
    vi.useFakeTimers()
    try {
      const calls = createRunCalls(useRunStore())
      bridge.runStart.mockReturnValue(new Promise(() => {}))
      const pending = calls.app({ ...base, tool: 'run_app', action: 'start' })
      await vi.advanceTimersByTimeAsync(RUN_REPLY_MS + 10)
      expect(await pending).toMatchObject({ refused: { code: 'waiting' } })
    } finally {
      vi.useRealTimers()
    }
  })

  it('status lists configs, devices and runs of this project only, with a one-line last error', async () => {
    const run = useRunStore()
    const calls = createRunCalls(run)
    run.handleEvent({ type: 'session', session: session({ state: 'failed', failure: { code: 'build', detail: 'Gradle failed' }, endedAt: 30 }) })
    run.handleEvent({ type: 'session', session: session({ id: 'other', cwd: 'C:\\elsewhere' }) })
    const res = await calls.status({ ...base, tool: 'run_status' })
    expect(res).toMatchObject({
      ok: true,
      configs: [{ id: 'a', selected: true }, { id: 'b', device: 'P2', selected: false }],
      devices: [{ serial: 'P1', selected: true }, { serial: 'P2' }],
      sessions: [{ id: 's1', state: 'crashed', lastError: 'build: Gradle failed' }],
    })
  })

  it('log returns the last N lines (errors only on request) and marks output lost to a reload', async () => {
    const run = useRunStore()
    const calls = createRunCalls(run)
    expect(await calls.log({ ...base, tool: 'run_log', lines: 10, errorsOnly: false })).toMatchObject({ session: null, lines: [] })
    run.handleEvent({ type: 'session', session: session() })
    run.handleEvent({
      type: 'log',
      sessionId: 's1',
      lines: [
        { stream: 'build', text: 'compiling', at: 1 },
        { stream: 'logcat', text: 'boom', level: 'E', at: 2 },
        { stream: 'app', text: 'Build failed with an exception', at: 3 },
        { stream: 'app', text: 'ok', at: 4 },
      ],
    })
    const all = await calls.log({ ...base, tool: 'run_log', lines: 2, errorsOnly: false })
    expect(all).toMatchObject({ lines: [{ text: 'Build failed with an exception' }, { text: 'ok' }], partial: false })
    const errs = await calls.log({ ...base, tool: 'run_log', lines: 50, errorsOnly: true })
    expect(errs).toMatchObject({ lines: [{ text: 'boom', level: 'E' }, { text: 'Build failed with an exception' }] })
    expect(isErrorLine({ stream: 'app', text: 'terror is fine', at: 0 })).toBe(false)
  })

  describe('log match / since', () => {
    type Out = { ok?: boolean; lines: { text: string }[]; cursor?: string; reset?: boolean; truncated?: boolean }
    const feed = (run: ReturnType<typeof useRunStore>, texts: string[], sessionId = 's1') =>
      run.handleEvent({ type: 'log', sessionId, lines: texts.map((text, i) => ({ stream: 'app' as const, text, at: i })) })
    const logOf = (calls: ReturnType<typeof createRunCalls>, over: Record<string, unknown> = {}) =>
      calls.log({ ...base, tool: 'run_log', lines: 50, errorsOnly: false, ...over } as never) as unknown as Promise<Out>
    const texts = (o: Out) => o.lines.map((l) => l.text)
    const setup = () => {
      const run = useRunStore()
      run.handleEvent({ type: 'session', session: session() })
      return { run, calls: createRunCalls(run) }
    }

    it('match filters per line, ignoreCase widens it, errorsOnly is ANDed', async () => {
      const { run, calls } = setup()
      feed(run, ['Alpha one', 'beta two', 'alpha three'])
      run.handleEvent({ type: 'log', sessionId: 's1', lines: [{ stream: 'logcat', level: 'E', text: 'alpha boom', at: 9 }] })
      expect(texts(await logOf(calls, { match: '^alpha' }))).toEqual(['alpha three', 'alpha boom'])
      expect(texts(await logOf(calls, { match: '^alpha', ignoreCase: true }))).toEqual(['Alpha one', 'alpha three', 'alpha boom'])
      expect(texts(await logOf(calls, { match: 'alpha', errorsOnly: true }))).toEqual(['alpha boom'])
    })

    it('an invalid regex is refused', async () => {
      const { calls } = setup()
      expect(await logOf(calls, { match: '(' })).toMatchObject({ ok: false })
    })

    it('since pages the oldest lines first without loss', async () => {
      const { run, calls } = setup()
      feed(run, ['l1', 'l2', 'l3', 'l4', 'l5'])
      expect(texts(await logOf(calls, { lines: 2 }))).toEqual(['l4', 'l5'])
      const p1 = await logOf(calls, { lines: 2, since: 's1.0.0' })
      expect(texts(p1)).toEqual(['l1', 'l2'])
      expect(p1.cursor).toBe('s1.0.2')
      const p2 = await logOf(calls, { lines: 2, since: p1.cursor })
      expect(texts(p2)).toEqual(['l3', 'l4'])
      const p3 = await logOf(calls, { lines: 2, since: p2.cursor })
      expect(texts(p3)).toEqual(['l5'])
      expect(p3.cursor).toBe('s1.0.5')
      expect((await logOf(calls, { since: p3.cursor })).lines).toEqual([])
      feed(run, ['l6'])
      expect(texts(await logOf(calls, { since: p3.cursor }))).toEqual(['l6'])
    })

    it('since with match skips non-matching lines and still advances', async () => {
      const { run, calls } = setup()
      feed(run, ['x1', 'y', 'x2', 'y', 'x3'])
      const a = await logOf(calls, { match: '^x', lines: 2, since: 's1.0.0' })
      expect(texts(a)).toEqual(['x1', 'x2'])
      const b = await logOf(calls, { match: '^x', lines: 2, since: a.cursor })
      expect(texts(b)).toEqual(['x3'])
      expect(b.cursor).toBe('s1.0.5')
    })

    it('a cursor from another run, a bad position or a cleared log resets to the start of the current log', async () => {
      const { run, calls } = setup()
      feed(run, ['a', 'b'])
      const old = (await logOf(calls)).cursor
      run.handleEvent({ type: 'session', session: session({ id: 's2', startedAt: 20 }) })
      feed(run, ['c', 'd'], 's2')
      const res = await logOf(calls, { since: old })
      expect(res).toMatchObject({ reset: true, cursor: 's2.0.2' })
      expect(texts(res)).toEqual(['c', 'd'])
      expect(await logOf(calls, { since: 's2.0.99' })).toMatchObject({ reset: true })
      run.clearLog('s2', ['app'])
      expect(await logOf(calls, { since: 's2.0.2' })).toMatchObject({ reset: true })
    })

    it('reports truncated when the ring buffer dropped lines after the cursor', async () => {
      const { run, calls } = setup()
      feed(run, ['first'])
      const cur = (await logOf(calls)).cursor
      feed(run, Array.from({ length: 5200 }, (_, i) => `n${i}`))
      const res = await logOf(calls, { since: cur, lines: 3 })
      expect(res.truncated).toBe(true)
      expect(res.reset).toBeUndefined()
      expect(texts(res)).toHaveLength(3)
    })
  })
})
