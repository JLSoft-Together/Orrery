import { existsSync, mkdirSync, mkdtempSync, readdirSync, realpathSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { DevCapture, DevSnapshot, ResKillResult, ResourceReport, RunSession } from '@ccm/shared'
import type { McpDeskCall } from '../../../packages/shared/src/mcpDesk'
import { mcpScreenshot, type McpShotDeps } from '../src/devices/mcpShot'
import type { HostHandlers } from '../src/desk/host'
import { createDesk, type DeskDeps } from '../src/mcpDesk'
import { mcpDeskHosts } from '../src/mcpDeskHosts'
import type { ProcRow } from '../src/run/proc'
import type { TelegramApi } from '../src/telegram/facade'
import { HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-mcp-')))
const home = path.join(tmp, 'home')
const project = path.join(home, 'app')
const layout = path.join(project, 'res', 'layout', 'main.xml')
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwQ2PLwAAAABJRU5ErkJggg==', 'base64')
let twin: Twin

const base = (tool: string, extra: Record<string, unknown> = {}): McpDeskCall => ({ id: 'c1', from: 'pane-1', cwd: project, pid: 100, name: 'app', tool, ...extra }) as McpDeskCall

const ROWS: ProcRow[] = [
  { pid: 100, ppid: 1, cmd: '' },
  { pid: 101, ppid: 100, cmd: '' },
  { pid: 102, ppid: 101, cmd: '' },
  { pid: 900, ppid: 1, cmd: '' },
]

function report(): ResourceReport {
  const now = Date.now()
  return {
    at: '2026-11-10T00:00:00.000Z',
    sessions: [{ pid: 101, start: '1', name: 'claude', ramBytes: 1, sessionId: 's1' }],
    servers: [
      { pid: 102, start: '5', name: 'vite', ramBytes: 1, cmd: 'node vite --port 5173', kind: 'dev-server', lastBusyAt: null, idleSinceMs: now - 100_300, orphan: false, ports: [{ port: 5173 }], cwd: project, sessionId: null },
      { pid: 300, start: '6', name: 'worker', ramBytes: 1, cmd: 'secret --token x', kind: 'dev-server', lastBusyAt: null, idleSinceMs: null, orphan: true, ports: [{ port: 3000 }, { port: 3000 }], cwd: '/elsewhere', sessionId: null },
      { pid: 301, start: '7', name: 'qemu', ramBytes: 1, cmd: 'emulator', kind: 'emulator', lastBusyAt: null, ports: [{ port: 5554 }], cwd: null, sessionId: null },
      { pid: 302, start: '', name: 'blank', ramBytes: 1, cmd: 'x', kind: 'dev-server', lastBusyAt: null, ports: [{ port: 4000 }], cwd: project, sessionId: null },
      { pid: 100, start: '8', name: 'zsh', ramBytes: 1, cmd: 'zsh', kind: 'dev-server', lastBusyAt: null, ports: [{ port: 6000 }], cwd: project, sessionId: null },
      { pid: 303, start: '9', name: 'idle', ramBytes: 1, cmd: 'node', kind: 'dev-server', lastBusyAt: null, idleSinceMs: null, orphan: false, ports: [], cwd: project, sessionId: 's1' },
    ],
    outside: [{ name: 'postgres', ports: [{ port: 5432 }, { port: 5432 }] }],
  } as unknown as ResourceReport
}

interface Rec {
  asks: { targets: unknown; message: string; detail: string; confirm: string }[]
  confirms: { message: string; detail: string; buttons: string[] }[]
  shots: string[]
  tgSet: unknown[]
  tgReports: unknown[][]
}

class FakeTg implements TelegramApi {
  constructor(
    public s: Record<string, unknown>,
    private rec: Rec,
    private sendResult: unknown,
  ) {}
  state(): never {
    return JSON.parse(JSON.stringify(this.s)) as never
  }
  set(patch: Record<string, unknown>): never {
    this.rec.tgSet.push(patch)
    Object.assign(this.s, patch)
    return { ok: true, state: this.s } as never
  }
  routeFor(cwd: string): never {
    return (cwd.endsWith('routed') ? { label: 'R', project: cwd } : null) as never
  }
  async report(...a: unknown[]): Promise<never> {
    this.rec.tgReports.push(a)
    return this.sendResult as never
  }
}

interface Opt {
  report?: ResourceReport | null
  render?: DeskDeps['render']
  render_n?: boolean
  writeShot?: string | null
  kill?: (t: unknown) => Promise<ResKillResult>
  tg?: Record<string, unknown> | null
  send?: unknown
  confirm?: number | 'throw'
  fallback: string
}

function mk(o: Opt): { deps: DeskDeps; rec: Rec } {
  const rec: Rec = { asks: [], confirms: [], shots: [], tgSet: [], tgReports: [] }
  const tg = o.tg ? new FakeTg({ ...o.tg }, rec, o.send ?? { ok: true, topic: 'T' }) : null
  const deps: DeskDeps = {
    render: o.render_n ? null : (o.render ?? (async () => ({ ok: false, code: 'unavailable' }))),
    writeShot: async (_png, name) => {
      rec.shots.push(name)
      return o.writeShot ?? null
    },
    fallbackDir: o.fallback,
    report: () => (o.report === undefined ? report() : o.report),
    procRows: async () => ROWS,
    home,
    locale: () => 'vi',
    kill: async (targets, ask) => {
      rec.asks.push({ targets, message: ask.message, detail: ask.detail(['• x']), confirm: ask.confirm })
      return o.kill ? o.kill(targets) : { ok: true, killed: 1 }
    },
    telegram: () => tg,
    confirm: async (spec) => {
      rec.confirms.push(spec)
      if (o.confirm === 'throw') throw new Error('boom')
      return o.confirm ?? 0
    },
  }
  return { deps, rec }
}

const norm = (v: unknown, a: string, b: string): unknown => JSON.parse(JSON.stringify(swapRoot(v, a, b)).replace(/-\d{13}\.png/g, '-N.png').replace(/\d{8}-\d{6}/g, 'STAMP'))

async function both(call: McpDeskCall, o: Omit<Opt, 'fallback'>, mid?: () => Promise<void>): Promise<unknown> {
  const fb = { node: path.join(tmp, `fb-n-${Math.random().toString(36).slice(2)}`), rust: path.join(tmp, `fb-r-${Math.random().toString(36).slice(2)}`) }
  const a = mk({ ...o, fallback: fb.node })
  const b = mk({ ...o, fallback: fb.rust })
  const hosts: HostHandlers = mcpDeskHosts({ desk: b.deps })
  const ts = await createDesk(a.deps)(call)
  await mid?.()
  const rs = await twin.host.call('desk:mcp-desk', [call, { deps: 'host', home, fallbackDir: fb.rust }], hosts)
  expect(norm(rs, fb.rust, fb.node)).toEqual(norm(ts, fb.node, fb.node))
  expect(b.rec).toEqual(a.rec)
  return ts
}

beforeAll(async () => {
  if (!HAVE_DESK) return
  mkdirSync(path.dirname(layout), { recursive: true })
  writeFileSync(layout, '<LinearLayout/>')
  twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
}, 30_000)

afterAll(async () => {
  await twin?.host.stop()
  rmSync(tmp, { recursive: true, force: true })
})


describe.skipIf(!HAVE_DESK)('mcp desk: TS createDesk and orrery-core desk (host deps) answer the same', () => {
  it('list_servers: no report, then mine first with commands only for mine', async () => {
    expect(await both(base('list_servers'), { report: null })).toEqual({ ok: false, error: 'no-report' })
    const r = (await both(base('list_servers'), {})) as { servers: { name: string; mine: boolean; cmd?: string }[] }
    expect(r.servers.map((s) => s.name)).toEqual(['blank', 'vite', 'zsh', 'idle', 'worker', 'qemu'])
    expect(r.servers.find((s) => s.name === 'worker')?.cmd).toBeUndefined()
    await both(base('list_servers', { pid: null }), {})
  })

  it('free_port: every plan refusal, the stop, and the error mapping', async () => {
    await both(base('free_port', { port: 5173 }), { report: null })
    for (const port of [9999, 5432, 5554, 3000, 4000, 6000, 5173]) await both(base('free_port', { port }), {})
    const ok = (await both(base('free_port', { port: 5173 }), {})) as { ok: boolean; stopped: unknown[] }
    expect(ok.ok).toBe(true)
    for (const error of ['cancelled', 'busy', 'gone', 'invalid', 'unavailable', 'failed', 'weird']) await both(base('free_port', { port: 5173 }), { kill: async () => ({ ok: false, error }) as ResKillResult })
    await both(base('free_port', { port: 5173, name: 'a\u0001b'.repeat(50) }), {})
  })

  it('free_port: a second call while the first confirm is open gets confirm-open', async () => {
    let release: () => void = () => undefined
    const gate = new Promise<void>((r) => (release = r))
    const nodeDeps = mk({ fallback: path.join(tmp, 'fb-c1'), kill: async () => (await gate, { ok: true, killed: 1 }) })
    const rustDeps = mk({ fallback: path.join(tmp, 'fb-c2'), kill: async () => (await gate, { ok: true, killed: 1 }) })
    const desk = createDesk(nodeDeps.deps)
    const first = desk(base('free_port', { port: 5173 }))
    await new Promise((r) => setTimeout(r, 30))
    const second = await desk(base('free_port', { port: 5173 }))
    const hosts = mcpDeskHosts({ desk: rustDeps.deps })
    const call = (): Promise<unknown> => twin.host.call('desk:mcp-desk', [base('free_port', { port: 5173 }), { deps: 'host', home }], hosts)
    const pending = call()
    for (let i = 0; i < 100 && rustDeps.rec.asks.length === 0; i++) await new Promise((r) => setTimeout(r, 20))
    const rustSecond = await call()
    release()
    expect(rustSecond).toEqual(second)
    expect(second).toEqual({ ok: false, error: 'confirm-open' })
    expect(await pending).toEqual(await first)
  })

  it('preview_layout: outside, unavailable, render refusals, saved to the capture folder or the fallback', async () => {
    await both(base('preview_layout', { path: path.join(tmp, 'other.xml'), device: 'phone', night: false }), {})
    await both(base('preview_layout', { path: layout, device: 'phone', night: false }), { render_n: true })
    for (const out of [{ ok: false, code: 'starting', retryInSec: 8 }, { ok: false, code: 'render-failed', detail: 'bad xml' }, { ok: false, code: 'weird', detail: 'x' }, { ok: false, code: 'busy' }, { ok: false, code: 'jdk-old' }]) {
      await both(base('preview_layout', { path: layout, device: 'tablet', night: true }), { render: async () => out as never })
    }
    const png = path.join(tmp, 'out.png')
    writeFileSync(png, PNG)
    const diagnostics = Array.from({ length: 15 }, (_, i) => ({ severity: 'warning' as const, message: `m${i}`, line: i + 1 }))
    const ok = async () => ({ ok: true as const, png, width: 1080, height: 2340, renderMs: 42, frameworkOnly: true, diagnostics })
    const viaCapture = (await both(base('preview_layout', { path: layout, device: 'phone', night: true }), { render: ok, writeShot: '/cap/shots/layout-main-dark-1.png' })) as { diagnostics: unknown[]; png: string }
    expect(viaCapture.diagnostics).toHaveLength(12)
    const viaTemp = (await both(base('preview_layout', { path: layout, device: 'phone', night: false }), { render: ok })) as { png: string }
    expect(viaTemp.png).toMatch(/layout-main-\d+\.png$/)
    await both(base('preview_layout', { path: layout, device: 'phone', night: false }), { render: async () => ({ ok: true as const, png: path.join(tmp, 'gone.png'), width: 1, height: 1, renderMs: 1, frameworkOnly: false, diagnostics: [] }) })
  })

  it('preview_layout: the fallback folder keeps the 20 newest layout files', async () => {
    const png = path.join(tmp, 'keep.png')
    writeFileSync(png, PNG)
    const dirs = { node: path.join(tmp, 'keep-n'), rust: path.join(tmp, 'keep-r') }
    for (const d of Object.values(dirs)) {
      mkdirSync(d, { recursive: true })
      for (let i = 0; i < 24; i++) {
        const f = path.join(d, `layout-old-${i}.png`)
        writeFileSync(f, 'x')
        utimesSync(f, 1_700_000_000 + i, 1_700_000_000 + i)
      }
      writeFileSync(path.join(d, 'keep-me.txt'), 'x')
    }
    const ok = async () => ({ ok: true as const, png, width: 1, height: 1, renderMs: 1, frameworkOnly: false, diagnostics: [] })
    const a = mk({ fallback: dirs.node, render: ok })
    const b = mk({ fallback: dirs.rust, render: ok })
    await createDesk(a.deps)(base('preview_layout', { path: layout, device: 'phone', night: false }))
    await twin.host.call('desk:mcp-desk', [base('preview_layout', { path: layout, device: 'phone', night: false }), { deps: 'host', home, fallbackDir: dirs.rust }], mcpDeskHosts({ desk: b.deps }))
    const names = (d: string): string[] => readdirSync(d).sort().map((n) => n.replace(/-\d{13}\.png$/, '-N.png'))
    expect(names(dirs.rust)).toEqual(names(dirs.node))
    expect(names(dirs.node).filter((n) => n.startsWith('layout-'))).toHaveLength(20)
    expect(existsSync(path.join(dirs.rust, 'keep-me.txt'))).toBe(true)
  })

  const TG = { configured: true, enabled: false, receiving: false, owner: { name: 'Jin' }, routes: [{ label: 'A' }, { label: 'B' }] }

  it('telegram_notify / telegram_send / telegram_receive flows', async () => {
    const routed = path.join(tmp, 'routed')
    for (const tool of ['telegram_notify', 'telegram_send', 'telegram_receive']) await both(base(tool, { state: 'status', text: 't' }), { tg: null })
    for (const state of ['status', 'on', 'off']) {
      await both(base('telegram_notify', { state }), { tg: TG })
      await both(base('telegram_notify', { state, cwd: routed }), { tg: TG })
      await both(base('telegram_notify', { state }), { tg: { ...TG, configured: false } })
    }
    await both(base('telegram_send', { text: 'hello' }), { tg: TG })
    await both(base('telegram_send', { text: 'hello', file: path.join(project, 'a.txt'), from: 'p' }), { tg: TG, send: { ok: true, topic: 'T', file: 'a.txt', linkOnly: true, uploading: true } })
    await both(base('telegram_send', { text: 'hello', buttons: [{ label: 'x', value: 'y' }] }), { tg: TG, send: { ok: true, topic: 'T', buttons: 1 } })
    await both(base('telegram_send', { text: 'hello', buttons: [{ label: 'x', value: 'y' }], file: '/a' }), { tg: TG })
    for (const error of ['rate', 'no-topic', 'outside', 'secret', 'network']) await both(base('telegram_send', { text: 'hello' }), { tg: TG, send: { ok: false, error } })
    for (const state of ['status', 'off']) await both(base('telegram_receive', { state }), { tg: TG })
    await both(base('telegram_receive', { state: 'on' }), { tg: { ...TG, configured: false } })
    await both(base('telegram_receive', { state: 'on' }), { tg: { ...TG, owner: null } })
    await both(base('telegram_receive', { state: 'on' }), { tg: TG, confirm: 0 })
    await both(base('telegram_receive', { state: 'on' }), { tg: TG, confirm: 1 })
    await both(base('telegram_receive', { state: 'on' }), { tg: TG, confirm: 'throw' })
    await both(base('telegram_receive', { state: 'on' }), { tg: { ...TG, receiving: true } })
  })

  it('unknown tool', async () => {
    expect(await both(base('nope'), {})).toEqual({ ok: false, error: 'unavailable' })
  })
})

const DEVICES: DevSnapshot = {
  adb: '/sdk/adb',
  autoConnect: false,
  devices: [
    { id: 'A1', name: 'Pixel 7', online: true, state: 'ready' },
    { id: 'B2', name: '', online: true, state: 'ready' },
    { id: 'C3', name: 'Offline', online: false, state: 'ready' },
  ] as never,
}

function mkShot(o: { snap?: DevSnapshot; runs?: RunSession[]; cap?: DevCapture; active?: boolean; written?: string | null }, calls: string[]): McpShotDeps & { tmpRoot: string } {
  return {
    snapshot: async () => o.snap ?? DEVICES,
    screencap: async (serial) => {
      calls.push(serial)
      return o.cap ?? { ok: true, png: `data:image/png;base64,${PNG.toString('base64')}`, width: 1, height: 1 }
    },
    runs: () => o.runs ?? [],
    writeShot: async (png, device) => {
      calls.push(`write:${device}:${png.length}`)
      return o.written === undefined ? '/cap/shots/x.png' : o.written
    },
    captureActive: () => o.active === true,
    tmpRoot: '',
  }
}

describe.skipIf(!HAVE_DESK)('mcp screenshot: TS mcpScreenshot and orrery-core desk answer the same', () => {
  const shotTwin = async (call: { cwd: string; serial?: string }, o: Parameters<typeof mkShot>[0], badRoot = false): Promise<unknown> => {
    const roots = { node: path.join(tmp, `sn-${Math.random().toString(36).slice(2)}`), rust: path.join(tmp, `sr-${Math.random().toString(36).slice(2)}`) }
    if (badRoot) for (const r of Object.values(roots)) writeFileSync(r, 'file')
    const ca: string[] = []
    const cb: string[] = []
    const ts = await mcpScreenshot(call, { ...mkShot(o, ca), tmpRoot: roots.node })
    const hosts = mcpDeskHosts({ shot: () => mkShot(o, cb) })
    const rs = await twin.host.call('desk:mcp-screenshot', [call, { deps: 'host', tmpRoot: roots.rust }], hosts)
    expect(norm(rs, roots.rust, roots.node)).toEqual(norm(ts, roots.node, roots.node))
    expect(cb).toEqual(ca)
    return ts
  }

  it('refusals and device choice', async () => {
    const cwd = project
    await shotTwin({ cwd }, { snap: { adb: null, devices: [], autoConnect: false } })
    await shotTwin({ cwd }, { snap: { adb: '/adb', devices: [], autoConnect: false } })
    await shotTwin({ cwd }, {})
    await shotTwin({ cwd, serial: 'C3' }, {})
    await shotTwin({ cwd, serial: 'zzz' }, {})
    const run = (deviceId: string, startedAt: number, state: string, at = project): RunSession => ({ id: deviceId, deviceId, cwd: at, startedAt, state }) as never
    const r = (await shotTwin({ cwd }, { runs: [run('A1', 1, 'running'), run('B2', 5, 'building'), run('A1', 9, 'stopped'), run('A1', 20, 'running', '/other')] })) as { shot: { serial: string; name: string } }
    expect(r.shot.serial).toBe('B2')
    expect(r.shot.name).toBe('B2')
    await shotTwin({ cwd }, { snap: { ...DEVICES, devices: [DEVICES.devices[0]!] } })
  })

  it('capture failures, capture folder, temp folder and write failure', async () => {
    const cwd = project
    for (const error of ['secure', 'timeout', 'adb-missing', 'not-connected', 'weird']) await shotTwin({ cwd, serial: 'A1' }, { cap: { ok: false, error } as never })
    await shotTwin({ cwd, serial: 'A1' }, { cap: { ok: true, png: 'data:image/png;base64,', width: 1, height: 1 } })
    const viaCapture = (await shotTwin({ cwd, serial: 'A1' }, { active: true })) as { shot: { saved: string } }
    expect(viaCapture.shot.saved).toBe('capture')
    await shotTwin({ cwd, serial: 'A1' }, { active: true, written: null })
    const temp = (await shotTwin({ cwd, serial: 'A1' }, {})) as { shot: { saved: string; path: string } }
    expect(temp.shot.saved).toBe('temp')
    expect(temp.shot.path).toContain('orrery-shots')
    await shotTwin({ cwd, serial: 'A1' }, {}, true)
  })
})
