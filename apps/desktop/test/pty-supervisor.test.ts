import { spawn, type ChildProcess } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import WebSocket from 'ws'
import type { PaneOpenRequest, PtyEndpoint, PtyServerMessage } from '../../../packages/shared/src/pty'
import { encodeControl, NdjsonReader } from '../../../packages/shared/src/ptyControl'

const ORIGIN = 'http://127.0.0.1:4317'
const hosts: ChildProcess[] = []
let bundle = ''
let stderr = ''

class FakeUtility extends EventEmitter {
  readonly stdout = null
  readonly stderr: NodeJS.ReadableStream | null
  constructor(private readonly proc: ChildProcess) {
    super()
    this.stderr = proc.stderr
    const reader = new NdjsonReader((v) => this.emit('message', v))
    proc.stdout!.setEncoding('utf8')
    proc.stdout!.on('data', (d: string) => reader.push(d))
    proc.stderr!.on('data', (d: Buffer) => (stderr += d.toString('utf8')))
    proc.once('exit', (code) => this.emit('exit', code ?? 0))
  }
  postMessage(msg: unknown): void {
    if (this.proc.stdin!.writable) this.proc.stdin!.write(encodeControl(msg as { t: string }))
  }
  kill(): boolean {
    return this.proc.kill()
  }
}

vi.mock('electron', () => ({
  app: { getVersion: () => '9.9.9' },
  utilityProcess: {
    fork: (script: string, _args: string[], opts: { env: NodeJS.ProcessEnv }) => {
      const proc = spawn(process.execPath, [script], { env: { ...opts.env, CCM_PTY_CONTROL: 'stdio' }, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
      hosts.push(proc)
      return new FakeUtility(proc)
    },
  },
}))

const { PtySupervisor } = await import('../src/pty/supervisor')

let tmp: string
let sup: InstanceType<typeof PtySupervisor>
let ep: PtyEndpoint
const asked: PaneOpenRequest[] = []
const actions: { action: string; caller: unknown }[] = []
let onOpen: (req: PaneOpenRequest) => void = () => undefined

class Client {
  readonly ws: WebSocket
  readonly texts: PtyServerMessage[] = []
  constructor() {
    this.ws = new WebSocket(ep.url, [ep.protocol], { origin: ORIGIN })
    this.ws.on('message', (data: Buffer, binary: boolean) => {
      if (!binary) this.texts.push(JSON.parse(data.toString('utf8')) as PtyServerMessage)
    })
  }
  opened(): Promise<void> {
    return new Promise((resolve, reject) => (this.ws.once('open', () => resolve()), this.ws.once('error', reject)))
  }
  send(msg: unknown): void {
    this.ws.send(JSON.stringify(msg))
  }
  async text<T extends PtyServerMessage['t']>(t: T, nth = 0): Promise<Extract<PtyServerMessage, { t: T }>> {
    let out: Extract<PtyServerMessage, { t: T }> | undefined
    await vi.waitFor(
      () => {
        out = this.texts.filter((m) => m.t === t)[nth] as Extract<PtyServerMessage, { t: T }> | undefined
        if (!out) throw new Error(`no ${t} yet: ${JSON.stringify(this.texts)} ${stderr}`)
      },
      { timeout: 10_000, interval: 20 },
    )
    return out!
  }
}

beforeAll(async () => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-sup-')))
  bundle = path.join(tmp, 'pty-host.cjs')
  const { build } = await import('esbuild')
  await build({
    entryPoints: [path.join(__dirname, '..', 'src', 'pty', 'host.ts')],
    outfile: bundle,
    bundle: true,
    platform: 'node',
    format: 'cjs',
    target: 'node24',
    logLevel: 'silent',
    logOverride: { 'empty-import-meta': 'silent' },
    external: ['@lydell/node-pty', 'bufferutil', 'utf-8-validate'],
  })
  sup = new PtySupervisor({
    script: bundle,
    nodePty: createRequire(__filename).resolve('@lydell/node-pty'),
    origins: [ORIGIN],
    logFile: path.join(tmp, 'pty.log'),
    shellInitDir: path.join(tmp, 'shell-init'),
    mcpConfig: path.join(tmp, 'orrery-mcp.json'),
    onOrrery: async (action, args, caller) => {
      actions.push({ action, caller })
      if (action !== 'session.open') return { ok: false, error: 'failed' }
      asked.push(args as PaneOpenRequest)
      onOpen(args as PaneOpenRequest)
      return { ok: true, paneKey: 'k' }
    },
  })
  ep = await sup.endpoint()
}, 60_000)

afterAll(async () => {
  await sup?.stop()
  for (const h of hosts) if (h.exitCode === null) h.kill()
  try {
    rmSync(tmp, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
  } catch {
    return
  }
}, 20_000)

describe('PtySupervisor + PtyBroker + Node host (control contract end to end)', () => {
  it('hands out a loopback endpoint and writes the MCP config from main', () => {
    expect(ep.url).toMatch(/^ws:\/\/127\.0\.0\.1:\d+\/pty$/)
    expect(ep.protocol).toMatch(/^ccm\.pty\.[0-9a-f]{64}$/)
    expect(readFileSync(path.join(tmp, 'orrery-mcp.json'), 'utf8')).toContain('ORRERY_TOKEN')
  })

  it('refuses bad preset and cwd with the same errors as before', async () => {
    const c = new Client()
    await c.opened()
    c.send({ t: 'spawn', preset: 'bash', cols: 80, rows: 24 })
    expect(await c.text('error')).toEqual({ t: 'error', message: 'preset' })
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: path.join(tmp, 'missing') })
    expect(await c.text('error', 1)).toEqual({ t: 'error', message: 'cwd' })
    c.send({ t: 'spawn', preset: 'claude', cols: 80, rows: 24, resume: 'nope' })
    expect(await c.text('error', 2)).toEqual({ t: 'error', message: 'preset' })
    c.ws.close()
  })

  it('spawns a shell, validates cwd in main, census and kill', async () => {
    const c = new Client()
    await c.opened()
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: tmp })
    const ready = await c.text('ready')
    expect(ready).toMatchObject({ preset: 'shell', cwd: tmp, replay: false })
    const sub = path.join(tmp, 'sub')
    mkdirSync(sub)
    c.send({ t: 'cwd', cwd: sub })
    expect(await c.text('cwd')).toEqual({ t: 'cwd', cwd: sub })
    c.send({ t: 'cwd', cwd: path.parse(tmp).root })
    expect(await c.text('cwd', 1)).toEqual({ t: 'cwd', cwd: null })
    const census = await sup.census()
    expect(census).toEqual([expect.objectContaining({ id: ready.id, preset: 'shell', cwd: tmp, liveCwd: sub, alive: true })])
    c.send({ t: 'wt.list', cwd: tmp })
    const wt = await c.text('wt.list').catch(() => c.text('wt.error'))
    expect(['wt.list', 'wt.error']).toContain(wt.t)
    c.send({ t: 'kill' })
    await vi.waitFor(async () => expect(await sup.census()).toEqual([]), { timeout: 10_000 })
    c.ws.close()
  }, 30_000)

  it('open_session over MCP served by main: the ticketed spawn reports its pane id', async () => {
    const from = new Client()
    await from.opened()
    from.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: tmp })
    const ready = await from.text('ready')
    const broker = (sup as unknown as { broker: { panes: Map<string, { token: string }> } }).broker
    const token = broker.panes.get(ready.id)!.token
    let spawnedId = ''
    const target = new Client()
    await target.opened()
    onOpen = (req) => {
      target.send({ t: 'spawn', preset: req.preset, cols: 80, rows: 24, cwd: req.cwd, open: req.id })
    }
    const ptyPort = Number(new URL(ep.url).port)
    const port = (sup as unknown as { mcpPort: number }).mcpPort
    expect(port).toBeGreaterThan(0)
    expect(port).not.toBe(ptyPort)
    expect((await fetch(`http://127.0.0.1:${ptyPort}/mcp`, { method: 'POST', headers: { Authorization: `Bearer ${token}` }, body: '{}' })).status).toBe(404)
    const res = await fetch(`http://127.0.0.1:${port}/mcp`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'open_session', arguments: { agent: 'shell' } } }),
    })
    const body = (await res.json()) as { result: { structuredContent: { ok: boolean; paneId?: string; preset: string } } }
    expect(asked.at(-1)).toMatchObject({ from: ready.id, preset: 'shell', cwd: tmp })
    expect(body.result.structuredContent).toMatchObject({ ok: true, preset: 'shell' })
    spawnedId = (await target.text('ready')).id
    expect(body.result.structuredContent.paneId).toBe(spawnedId)
    const bad = await fetch(`http://127.0.0.1:${port}/mcp`, { method: 'POST', headers: { Authorization: `Bearer ${'0'.repeat(64)}` }, body: '{}' })
    expect(bad.status).toBe(401)
    const note = await fetch(`http://127.0.0.1:${port}/mcp`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'notify_owner', arguments: { text: 'hello' } } }),
    })
    expect(note.status).toBe(200)
    await vi.waitFor(() => expect(actions.some((a) => a.action !== 'session.open' && JSON.stringify(a.caller) === JSON.stringify({ source: 'mcp', pane: ready.id }))).toBe(true))
    from.send({ t: 'kill', id: spawnedId })
    from.send({ t: 'kill' })
    await vi.waitFor(async () => expect(await sup.census()).toEqual([]), { timeout: 10_000 })
    await vi.waitFor(async () => expect((await fetch(`http://127.0.0.1:${port}/mcp`, { method: 'POST', headers: { Authorization: `Bearer ${token}` }, body: '{}' })).status).toBe(401))
    from.ws.close()
    target.ws.close()
  }, 30_000)

  it('stop shuts the host down; endpoint refuses afterwards', async () => {
    await sup.stop()
    expect(hosts.every((h) => h.exitCode !== null)).toBe(true)
    await expect(sup.endpoint()).rejects.toThrow('pty host stopped')
    const port = (sup as unknown as { mcpPort: number }).mcpPort
    await expect(fetch(`http://127.0.0.1:${port}/mcp`, { method: 'POST', body: '{}' })).rejects.toThrow()
    await vi.waitFor(() => expect(readFileSync(path.join(tmp, 'pty.log'), 'utf8')).toContain('pty host exited'))
  }, 20_000)
})
