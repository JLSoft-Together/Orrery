import { spawn, type ChildProcess } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import WebSocket from 'ws'
import type { PtyServerMessage } from '../../../packages/shared/src/pty'
import { encodeControl, NdjsonReader, parseCoreToMain, type PtyCoreToMain, type PtyMainToCore } from '../../../packages/shared/src/ptyControl'

const ORIGIN = 'http://127.0.0.1:4317'
const TOKEN = randomBytes(32).toString('hex')

let tmp: string
let bundle: string
let host: ChildProcess
let port = 0
let stderr = ''
const fromHost: PtyCoreToMain[] = []
const listeners = new Set<(m: PtyCoreToMain) => void>()

const toHost = (msg: PtyMainToCore): void => void host.stdin!.write(encodeControl(msg))

function nextControl(pred: (m: PtyCoreToMain) => boolean, ms = 8_000): Promise<PtyCoreToMain> {
  const seen = fromHost.find(pred)
  if (seen) {
    fromHost.splice(fromHost.indexOf(seen), 1)
    return Promise.resolve(seen)
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      listeners.delete(on)
      reject(new Error(`no control message in ${ms} ms; stderr: ${stderr}`))
    }, ms)
    const on = (m: PtyCoreToMain): void => {
      if (!pred(m)) return
      clearTimeout(timer)
      listeners.delete(on)
      fromHost.splice(fromHost.indexOf(m), 1)
      resolve(m)
    }
    listeners.add(on)
  })
}

class Client {
  readonly ws: WebSocket
  readonly texts: PtyServerMessage[] = []
  output = ''
  closeCode: number | null = null
  private waiters: (() => void)[] = []

  constructor() {
    this.ws = new WebSocket(`ws://127.0.0.1:${port}/pty`, [`ccm.pty.${TOKEN}`], { origin: ORIGIN })
    this.ws.on('message', (data: Buffer, binary: boolean) => {
      if (binary) this.output += data.toString('utf8')
      else this.texts.push(JSON.parse(data.toString('utf8')) as PtyServerMessage)
      this.wake()
    })
    this.ws.on('close', (code) => {
      this.closeCode = code
      this.wake()
    })
  }

  private wake(): void {
    for (const w of this.waiters.splice(0)) w()
  }

  opened(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws.once('open', () => resolve())
      this.ws.once('error', reject)
    })
  }

  send(msg: unknown): void {
    this.ws.send(JSON.stringify(msg))
  }

  async until<T>(check: () => T | undefined | false, ms = 8_000): Promise<T> {
    const end = Date.now() + ms
    for (;;) {
      const v = check()
      if (v) return v
      if (Date.now() > end) throw new Error(`timeout; texts=${JSON.stringify(this.texts)} output=${JSON.stringify(this.output.slice(-200))} stderr=${stderr}`)
      await new Promise<void>((r) => {
        this.waiters.push(r)
        setTimeout(r, 100)
      })
    }
  }

  text<T extends PtyServerMessage['t']>(t: T): Promise<Extract<PtyServerMessage, { t: T }>> {
    return this.until(() => this.texts.find((m) => m.t === t) as Extract<PtyServerMessage, { t: T }> | undefined)
  }
}

const CHILD = "process.stdout.write('hi from child\\n');process.stdin.on('data',(d)=>{process.stdout.write('got:'+d);if(String(d).includes('q'))process.exit(3)})"

function resolvedFor(rid: string, id: string): PtyMainToCore {
  const env: Record<string, string> = {}
  for (const [k, v] of Object.entries(process.env)) if (typeof v === 'string' && !k.startsWith('CCM_') && !k.startsWith('VITEST')) env[k] = v
  return { t: 'resolved', rid, ok: true, id, file: process.execPath, args: ['-e', CHILD], env: { ...env, ORRERY_TOKEN: 'c'.repeat(64) }, cwd: tmp }
}

async function spawnPane(c: Client, id: string, delay = 0): Promise<void> {
  c.send({ t: 'spawn', preset: 'shell', cols: 100, rows: 30, cwd: tmp })
  const req = await nextControl((m) => m.t === 'resolve')
  if (req.t !== 'resolve') throw new Error('unreachable')
  expect(req.msg).toMatchObject({ t: 'spawn', preset: 'shell', cols: 100, rows: 30, cwd: tmp })
  if (delay) await new Promise((r) => setTimeout(r, delay))
  toHost(resolvedFor(req.rid, id))
}

beforeAll(async () => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-host-stdio-')))
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
  await startHost()
}, 60_000)

async function startHost(): Promise<void> {
  fromHost.length = 0
  listeners.clear()
  host = spawn(process.execPath, [bundle], {
    cwd: path.join(__dirname, '..'),
    env: { ...process.env, CCM_PTY_CONTROL: 'stdio', CCM_NODE_PTY: createRequire(__filename).resolve('@lydell/node-pty') },
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
  })
  host.stderr!.setEncoding('utf8')
  host.stderr!.on('data', (d: string) => (stderr += d))
  const reader = new NdjsonReader((raw) => {
    const m = parseCoreToMain(raw)
    if (!m) return
    fromHost.push(m)
    for (const l of [...listeners]) l(m)
  })
  host.stdout!.setEncoding('utf8')
  host.stdout!.on('data', (d: string) => reader.push(d))
  toHost({ t: 'init', token: TOKEN, origins: [ORIGIN], appVersion: '9.9.9' })
  const listening = await nextControl((m) => m.t === 'listening', 15_000)
  if (listening.t === 'listening') port = listening.port
}

afterAll(async () => {
  if (host && host.exitCode === null) {
    const exited = new Promise((r) => host.once('exit', r))
    host.stdin!.end()
    await Promise.race([exited, new Promise((r) => setTimeout(r, 5_000))])
    if (host.exitCode === null) host.kill()
  }
  try {
    rmSync(tmp, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
  } catch {
    return
  }
}, 15_000)

describe('Node PTY host over the stdio control channel', () => {
  it('refuses a wrong token or origin before the upgrade', async () => {
    const bad = new WebSocket(`ws://127.0.0.1:${port}/pty`, ['ccm.pty.nope'], { origin: ORIGIN })
    await expect(new Promise((resolve, reject) => (bad.once('open', resolve), bad.once('error', reject)))).rejects.toThrow()
    const foreign = new WebSocket(`ws://127.0.0.1:${port}/pty`, [`ccm.pty.${TOKEN}`], { origin: 'http://evil.test' })
    await expect(new Promise((resolve, reject) => (foreign.once('open', resolve), foreign.once('error', reject)))).rejects.toThrow()
  })

  it('spawn goes through resolve; frames sent before the answer reach the child; exit flushes then reports', async () => {
    const c = new Client()
    await c.opened()
    const id = '6f1c1d2e-0000-4000-8000-000000000001'
    const pending = spawnPane(c, id, 150)
    c.send({ t: 'resize', cols: 90, rows: 20 })
    c.ws.send(Buffer.from('q\r'))
    await pending
    const ready = await c.text('ready')
    expect(ready).toMatchObject({ t: 'ready', id, preset: 'shell', cwd: tmp, replay: false })
    expect(ready.si).toBeUndefined()
    const spawned = await nextControl((m) => m.t === 'spawned' && m.id === id)
    expect(spawned).toMatchObject({ t: 'spawned', id })
    await c.until(() => c.output.includes('hi from child'))
    const exit = await c.text('exit')
    expect(exit.code).toBe(3)
    expect(c.output).toContain('got:q')
    expect(await nextControl((m) => m.t === 'exited' && m.id === id)).toEqual({ t: 'exited', id, code: 3 })

    const again = new Client()
    await again.opened()
    again.send({ t: 'attach', id })
    const replay = await again.text('ready')
    expect(replay).toMatchObject({ id, replay: true })
    await again.until(() => again.output.includes('hi from child'))
    expect((await again.text('exit')).code).toBe(3)
    await c.until(() => c.closeCode === 4000)
    again.ws.close()
  }, 30_000)

  it('a refused resolve answers error and the socket may spawn again; unknown attach is gone', async () => {
    const c = new Client()
    await c.opened()
    c.send({ t: 'attach', id: 'nope' })
    await c.text('gone')
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: '/nope' })
    const req = await nextControl((m) => m.t === 'resolve')
    if (req.t !== 'resolve') throw new Error('unreachable')
    toHost({ t: 'resolved', rid: req.rid, ok: false, message: 'cwd' })
    expect(await c.text('error')).toEqual({ t: 'error', message: 'cwd' })
    const id = '6f1c1d2e-0000-4000-8000-000000000002'
    await spawnPane(c, id)
    expect(await c.text('ready')).toMatchObject({ id })
    await nextControl((m) => m.t === 'spawned' && m.id === id)

    c.send({ t: 'cwd', cwd: tmp })
    const cwdAsk = await nextControl((m) => m.t === 'cwd')
    if (cwdAsk.t !== 'cwd') throw new Error('unreachable')
    expect(cwdAsk).toMatchObject({ id, cwd: tmp })
    const sub = path.join(tmp, 'sub')
    toHost({ t: 'cwd-ok', rid: cwdAsk.rid, cwd: sub })
    expect(await c.text('cwd')).toEqual({ t: 'cwd', cwd: sub })
    c.send({ t: 'cwd', cwd: 42 })
    await c.until(() => c.texts.filter((m) => m.t === 'cwd').length === 2)
    expect(c.texts.filter((m) => m.t === 'cwd')[1]).toEqual({ t: 'cwd', cwd: null })

    toHost({ t: 'branch', id, branch: 'main' })
    expect(await c.text('branch')).toEqual({ t: 'branch', branch: 'main' })

    c.send({ t: 'wt.list', cwd: tmp })
    const wt = await nextControl((m) => m.t === 'wt')
    if (wt.t !== 'wt') throw new Error('unreachable')
    expect(wt.msg).toEqual({ t: 'wt.list', cwd: tmp })
    toHost({ t: 'wt-reply', rid: wt.rid, reply: { t: 'wt.list', repo: null, worktrees: [] } })
    expect(await c.text('wt.list')).toEqual({ t: 'wt.list', repo: null, worktrees: [] })

    toHost({ t: 'census' })
    const census = await nextControl((m) => m.t === 'census-reply')
    if (census.t !== 'census-reply') throw new Error('unreachable')
    expect(census.sessions.find((s) => s.id === id)).toMatchObject({ id, preset: 'shell', cwd: tmp, liveCwd: sub, alive: true })

    const late = new Client()
    await late.opened()
    late.send({ t: 'attach', id })
    const replay = await late.text('ready')
    expect(replay).toMatchObject({ id, replay: true })
    await late.until(() => late.texts.some((m) => m.t === 'cwd') && late.texts.some((m) => m.t === 'branch'))
    expect(late.texts.map((m) => m.t)).toEqual(['ready', 'cwd', 'branch'])

    const killer = new Client()
    await killer.opened()
    killer.send({ t: 'kill', id })
    expect(await nextControl((m) => m.t === 'removed' && m.id === id)).toEqual({ t: 'removed', id })
    toHost({ t: 'census' })
    const after = await nextControl((m) => m.t === 'census-reply')
    if (after.t !== 'census-reply') throw new Error('unreachable')
    expect(after.sessions.some((s) => s.id === id)).toBe(false)
    for (const x of [c, late, killer]) x.ws.close()
  }, 30_000)

  it('a resolve main never answers ends in error spawn after 15 s; the late answer is dropped', async () => {
    const c = new Client()
    await c.opened()
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24 })
    const req = await nextControl((m) => m.t === 'resolve')
    if (req.t !== 'resolve') throw new Error('unreachable')
    expect(await c.until(() => c.texts.find((m) => m.t === 'error'), 20_000)).toEqual({ t: 'error', message: 'spawn' })
    const id = '6f1c1d2e-0000-4000-8000-000000000003'
    toHost(resolvedFor(req.rid, id))
    expect(await nextControl((m) => m.t === 'removed' && m.id === id)).toEqual({ t: 'removed', id })
    c.ws.close()
  }, 40_000)

  it('input queued while spawning is capped at 4 MB: binary over it is dropped and logged', async () => {
    const c = new Client()
    await c.opened()
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24 })
    const req = await nextControl((m) => m.t === 'resolve')
    if (req.t !== 'resolve') throw new Error('unreachable')
    const chunk = Buffer.alloc(1_000_000, 0x78)
    for (let i = 0; i < 5; i++) c.ws.send(chunk)
    c.send({ t: 'attach', id: 'nope' })
    await new Promise((r) => setTimeout(r, 300))
    toHost({ t: 'resolved', rid: req.rid, ok: false, message: 'cwd' })
    expect(await c.text('error')).toEqual({ t: 'error', message: 'cwd' })
    expect(await c.text('gone')).toEqual({ t: 'gone' })
    await c.until(() => stderr.includes('pty input dropped while spawning: 1 frames'))
    c.ws.close()
  }, 20_000)

  it('after shutdown no new pane spawns: pending and new spawns get error spawn, ids are removed', async () => {
    const a = new Client()
    await a.opened()
    const live = '6f1c1d2e-0000-4000-8000-000000000005'
    await spawnPane(a, live)
    await nextControl((m) => m.t === 'spawned' && m.id === live)
    const b = new Client()
    await b.opened()
    b.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24 })
    const req = await nextControl((m) => m.t === 'resolve')
    if (req.t !== 'resolve') throw new Error('unreachable')
    const exited = new Promise<number | null>((r) => host.once('exit', (code) => r(code)))
    toHost({ t: 'shutdown' })
    const late = '6f1c1d2e-0000-4000-8000-000000000006'
    toHost(resolvedFor(req.rid, late))
    expect(await nextControl((m) => m.t === 'removed' && m.id === late, 2_000)).toEqual({ t: 'removed', id: late })
    expect(await b.text('error')).toEqual({ t: 'error', message: 'spawn' })
    expect(await nextControl((m) => m.t === 'removed' && m.id === live, 2_000)).toEqual({ t: 'removed', id: live })
    expect(fromHost.some((m) => m.t === 'spawned')).toBe(false)
    expect(await exited).toBe(0)
  }, 20_000)

  it('stdin EOF shuts the host down and kills live panes', async () => {
    await startHost()
    const c = new Client()
    await c.opened()
    const id = '6f1c1d2e-0000-4000-8000-000000000004'
    await spawnPane(c, id)
    await nextControl((m) => m.t === 'spawned' && m.id === id)
    const exited = new Promise<number | null>((r) => host.once('exit', (code) => r(code)))
    host.stdin!.end()
    expect(await exited).toBe(0)
    expect(await nextControl((m) => m.t === 'removed' && m.id === id, 1_000)).toEqual({ t: 'removed', id })
  }, 20_000)
})
