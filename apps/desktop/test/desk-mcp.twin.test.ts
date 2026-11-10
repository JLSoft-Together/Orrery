import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { handleMcp } from '../src/pty/mcp'
import { mcpToolsFor } from '../src/pty/mcpTools'
import type { DeskAsk, OpenArgs, PageAsk, ToolHost } from '../src/pty/mcpTools/types'
import { rustMcpHandle } from '../src/pty/mcpRust'
import { HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-mcp-rpc-')))
const project = path.join(tmp, 'app')
let twin: Twin

const SESSION = { id: 'run-1', configId: 'app', configName: 'App ghp_abcdefghijklmnop1234', type: 'flutter', deviceId: 'emu-5554', deviceName: 'Pixel', state: 'running', rawState: 'running', startedAt: 1_700_000_000_000, endedAt: null, appId: 'com.x', lastError: null }

function pageAnswer(call: PageAsk): unknown {
  switch (call.tool) {
    case 'list_sessions':
      return { ok: true, tool: 'list_sessions', sessions: [{ sessionId: 'p2', name: 'Other "one"', agent: 'codex', status: 'idle', self: false, openedByYou: true, idleSince: '2026-11-10T01:00:00.000Z', limit: { window: 'five_hour', resetsAt: '2026-11-10T05:00:00.000Z' } }, { sessionId: 'p1', name: 'Me', agent: 'claude', status: 'busy', self: true, openedByYou: false }] }
    case 'send_to_session':
      return { ok: true, tool: 'send_to_session', name: 'Other' }
    case 'close_session':
      return call.session === 'p2' ? { ok: true, tool: 'close_session', name: 'Other', confirm: true } : { ok: false, error: 'target' }
    case 'open_in_workspace':
      return { ok: true, tool: 'open_in_workspace', paneKey: 'k', reused: true }
    case 'who_is_editing':
      return { ok: true, tool: 'who_is_editing', repo: true, editors: [{ name: 'Wt', agent: 'claude', branch: 'feat/x', root: '/elsewhere', sameCheckout: false, file: '/elsewhere/a.ts', status: 'busy' }, { name: 'Same', agent: 'codex', branch: null, root: project, sameCheckout: true, file: path.join(project, 'a.ts') }] }
    case 'run_status':
      return { ok: true, tool: 'run_status', configs: [{ id: 'app', name: 'App', type: 'flutter', selected: true }], devices: [{ serial: 'emu-5554', name: 'Pixel', platform: 'android', state: 'ready', selected: true }], sessions: [SESSION], focused: 'run-1' }
    case 'run_app':
      return call.action === 'start' ? { ok: true, tool: 'run_app', action: 'start', refused: { code: 'running', detail: 'token=abcdef123' }, session: SESSION } : { ok: true, tool: 'run_app', action: call.action, session: SESSION }
    case 'run_log':
      return { ok: true, tool: 'run_log', session: SESSION, lines: [{ stream: 'app', level: 'E', text: 'boom password=hunter22' }, { stream: 'build', text: 'ok' }], errors: [{ file: 'lib/a.dart', line: 3, col: 4, message: 'bad', severity: 'error' }], partial: true, cursor: 'run-1.0.12', truncated: true }
    case 'device_screenshot':
      return call.serial === 'none' ? { ok: true, tool: 'device_screenshot', refused: { code: 'several' }, devices: [{ serial: 'a', name: 'A' }] } : { ok: true, tool: 'device_screenshot', shot: { path: '/tmp/s.png', width: 10, height: 20, serial: 'emu-5554', name: 'Pixel', saved: 'temp' } }
    default:
      return { ok: false, error: 'unavailable' }
  }
}

function deskAnswer(call: DeskAsk): unknown {
  if (call.tool === 'list_servers') return { ok: true, tool: 'list_servers', at: '2026-11-10T00:00:00.000Z', servers: [{ pid: 9, name: 'vite', kind: 'dev-server', ports: [5173], idleSec: 4500, mine: true, orphan: false, cmd: 'vite' }], outside: [{ port: 22, name: 'sshd' }] }
  if (call.tool === 'free_port') return { ok: false, error: 'not-mine' }
  if (call.tool === 'telegram_notify') return { ok: true, tool: 'telegram_notify', enabled: true, configured: true, projects: ['app'], routed: false }
  if (call.tool === 'telegram_send') return { ok: false, error: 'not-paired', detail: 'not-paired' }
  if (call.tool === 'preview_layout') return { ok: false, error: 'starting', retryInSec: 5 }
  return { ok: false, error: 'unavailable' }
}

function fakeHost(log: unknown[], opts: { preset?: ToolHost['preset']; desk?: boolean } = {}): ToolHost {
  const rates = new Map<string, number>()
  return {
    root: project,
    preset: opts.preset ?? 'claude',
    take: (key, max) => {
      log.push(['take', key, max])
      const n = (rates.get(key) ?? 0) + 1
      rates.set(key, n)
      return n <= Math.min(max, 2)
    },
    openSession: async (args: OpenArgs) => (log.push(['open', args]), args.agent === 'shell' ? { ok: false, error: 'limit' } : { ok: true, preset: 'codex', cwd: project, paneId: 'np', prompt: true, submitted: true }),
    notify: async (text, level) => (log.push(['notify', text, level]), text.includes('held') ? { ok: true, sent: false, inMs: 1500 } : { ok: true, sent: true, name: 'app', inbox: true }),
    checkoutRoots: async () => (log.push(['roots']), ['/elsewhere', project]),
    page: async (call) => (log.push(['page', call]), pageAnswer(call) as never),
    ...(opts.desk === false ? {} : { desk: async (call: DeskAsk) => (log.push(['desk', call]), deskAnswer(call) as never) }),
  }
}

const rpc = (id: number, name: string, args?: unknown) => ({ jsonrpc: '2.0', id, method: 'tools/call', params: { name, ...(args === undefined ? {} : { arguments: args }) } })

const BODIES: unknown[] = [
  { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-03-26' } },
  { jsonrpc: '2.0', id: 2, method: 'initialize', params: { protocolVersion: 'x' } },
  { jsonrpc: '2.0', id: 3, method: 'tools/list' },
  { jsonrpc: '2.0', method: 'notifications/initialized' },
  { jsonrpc: '2.0', id: 4, method: 'nope' },
  { jsonrpc: '1.0', id: 5, method: 'ping' },
  [],
  [{ jsonrpc: '2.0', id: 6, method: 'ping' }, { jsonrpc: '2.0', method: 'x' }],
  Array.from({ length: 17 }, (_, i) => ({ jsonrpc: '2.0', id: i, method: 'ping' })),
  rpc(7, 'hand_back', { report: 'r.md', summary: 's' }),
  rpc(8, 'list_sessions', [1]),
  rpc(9, 'open_session', { agent: 'codex', prompt: 'go', submit: true }),
  rpc(10, 'open_session', { agent: 'shell' }),
  rpc(11, 'notify_owner', { text: 'done   now‮', level: 'done' }),
  rpc(12, 'notify_owner', { text: 'held one' }),
  rpc(13, 'notify_owner', { text: '', level: 'loud' }),
  rpc(14, 'notify_owner', { text: 'x'.repeat(201) }),
  rpc(15, 'open_in_workspace', { path: 'a.ts', line: 3 }),
  rpc(16, 'open_in_workspace', { path: 'missing.ts' }),
  rpc(17, 'open_in_workspace', { path: 'missing.ts', diff: true }),
  rpc(18, 'open_in_workspace', { path: '../out.ts' }),
  rpc(19, 'open_in_workspace', { path: 'a.ts', line: 0 }),
  rpc(20, 'open_in_workspace', { path: 'a.ts', diff: 'yes' }),
  rpc(21, 'open_in_workspace', { path: 'src' }),
  rpc(22, 'who_is_editing', { path: 'a.ts' }),
  rpc(23, 'list_sessions'),
  rpc(24, 'send_to_session', { session: 'p2', text: '  hi\r\nthere ' }),
  rpc(25, 'send_to_session', { session: '', text: 'x' }),
  rpc(26, 'close_session', { session: 'p2' }),
  rpc(27, 'close_session', { session: 'zz' }),
  rpc(28, 'wait_session', { session: 'p2' }),
  rpc(29, 'wait_session', { session: 'p1', until: 'exited' }),
  rpc(30, 'wait_session', { session: 'p2', until: 'later' }),
  rpc(31, 'wait_session', { session: 'p2', timeoutMs: 5 }),
  rpc(32, 'run_status'),
  rpc(33, 'run_app', { action: 'start' }),
  rpc(34, 'run_app', { action: 'reload', full: true }),
  rpc(35, 'run_app', { action: 'stop', full: true }),
  rpc(36, 'run_app', { action: 'fly' }),
  rpc(37, 'run_log', { lines: 1, errorsOnly: true }),
  rpc(38, 'run_log', { match: '(', ignoreCase: true }),
  rpc(39, 'run_log', { since: 'run-1.0.3', match: 'bo+m', ignoreCase: true }),
  rpc(40, 'run_log', { lines: 999 }),
  rpc(41, 'device_screenshot'),
  rpc(42, 'device_screenshot', { serial: 'none' }),
  rpc(43, 'device_screenshot', { serial: '-x' }),
  rpc(44, 'list_servers'),
  rpc(45, 'free_port', { port: 5173 }),
  rpc(46, 'free_port', { port: 80 }),
  rpc(47, 'telegram_notify', { state: 'status' }),
  rpc(48, 'telegram_send', { text: 'done', buttons: [{ label: 'Yes', value: 'yes' }] }),
  rpc(49, 'telegram_send', { text: 'done', file: '.env' }),
  rpc(50, 'telegram_receive', { state: 'maybe' }),
  rpc(51, 'preview_layout', { path: 'res/layout/main.xml' }),
  rpc(52, 'preview_layout', { path: 'a.ts' }),
  rpc(53, 'preview_layout', { path: 'res/layout/main.xml', device: 'watch' }),
  rpc(54, 'run_status'),
  rpc(55, 'run_status'),
]

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"waitedMs":\d+/g, '"waitedMs":0').replace(/waited \d+ s/g, 'waited 0 s'))

beforeAll(async () => {
  mkdirSync(path.join(project, 'src'), { recursive: true })
  mkdirSync(path.join(project, 'res', 'layout'), { recursive: true })
  writeFileSync(path.join(project, 'a.ts'), 'x')
  writeFileSync(path.join(project, '.env'), 'K=v')
  writeFileSync(path.join(project, 'res', 'layout', 'main.xml'), '<LinearLayout/>')
  if (HAVE_DESK) twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
}, 30_000)

afterAll(async () => {
  await twin?.host.stop()
  rmSync(tmp, { recursive: true, force: true })
})

async function both(bodies: unknown[], opts: { preset?: ToolHost['preset']; desk?: boolean; deepseek?: boolean } = {}): Promise<void> {
  const tsLog: unknown[] = []
  const rsLog: unknown[] = []
  const tsHost = fakeHost(tsLog, opts)
  const rsHost = fakeHost(rsLog, opts)
  const rust = rustMcpHandle((c, a, h) => twin.router.invoke(c, a, h), '9.9.9', opts.deepseek === true)
  for (const body of bodies) {
    const ts = await handleMcp(body, tsHost, '9.9.9', mcpToolsFor(opts.deepseek === true))
    const rs = await rust(body, rsHost)
    expect(norm(rs), JSON.stringify(body).slice(0, 120)).toEqual(norm(ts))
  }
  expect(norm(rsLog)).toEqual(norm(tsLog))
  expect(twin.fellBack).toEqual([])
}

describe.skipIf(!HAVE_DESK)('mcp json-rpc: TS handleMcp and orrery-core desk:mcp-handle answer the same', () => {
  it('every tool, error branch and the host calls they make', async () => {
    await both(BODIES)
  }, 60_000)

  it('without a desk, and with DeepSeek on (hand_back for a DeepSeek pane)', async () => {
    await both([rpc(1, 'list_servers'), rpc(2, 'telegram_notify', { state: 'on' }), rpc(3, 'hand_back', { report: 'a.ts', summary: 'x' })], { desk: false })
    writeFileSync(path.join(project, 'r.md'), '# done')
    await both([{ jsonrpc: '2.0', id: 1, method: 'initialize' }, { jsonrpc: '2.0', id: 2, method: 'tools/list' }, rpc(3, 'hand_back', { report: 'r.md', summary: 'All  done' }), rpc(4, 'hand_back', { report: 'a.ts', summary: 'x' }), rpc(5, 'open_session', { agent: 'deepseek' })], { preset: 'deepseek', deepseek: true })
    await both([rpc(1, 'hand_back', { report: 'r.md', summary: 'x' })], { preset: 'claude', deepseek: true })
  }, 30_000)
})
