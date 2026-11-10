import { EventEmitter } from 'node:events'
import { mkdtempSync } from 'node:fs'
import http from 'node:http'
import type { AddressInfo } from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { PassThrough } from 'node:stream'
import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'

vi.mock('electron', () => ({ utilityProcess: { fork: () => null } }))

const { FatalError } = await import('../src/collector-host')
const { CoreServeHost, SwitchingHost, coreServeMode, fallbackReason } = await import('../src/core-serve-host')
const { HostRpc } = await import('../src/host-rpc')
const { createServeToasts, parseFullToast } = await import('../src/serve-toast')

let up = false
let server: http.Server
let port = 0
const logDir = mkdtempSync(path.join(os.tmpdir(), 'ccm-serve-host-'))

beforeAll(async () => {
  server = http.createServer((req, res) => {
    res.statusCode = up && req.url === '/health' ? 200 : 503
    res.end('{}')
  })
  await new Promise<void>((r) => server.listen(0, '127.0.0.1', () => r()))
  port = (server.address() as AddressInfo).port
})

afterAll(() => new Promise<void>((r) => server.close(() => r())))

class FakeChild extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  written = ''
  constructor(readonly env: NodeJS.ProcessEnv, readonly args: string[], exitCode: number | null) {
    super()
    this.stdin.on('data', (d: Buffer) => {
      this.written += String(d)
      if (String(d).includes('"shutdown"')) this.exit(0)
    })
    setTimeout(() => {
      if (exitCode !== null) return this.exit(exitCode)
      up = true
      this.stdout.write(`${JSON.stringify({ type: 'listening', port })}\n`)
    }, 20)
  }
  kill(): boolean {
    this.exit(null)
    return true
  }
  exit(code: number | null): void {
    up = false
    setTimeout(() => this.emit('exit', code), 5)
  }
}

function spawner(codes: (number | null)[] = []) {
  const children: FakeChild[] = []
  const fn = (_bin: string, args: string[], env: NodeJS.ProcessEnv): ChildProcessWithoutNullStreams => {
    const c = new FakeChild(env, args, codes.shift() ?? null)
    children.push(c)
    return c as unknown as ChildProcessWithoutNullStreams
  }
  return { fn, children }
}

const until = async (check: () => boolean, ms = 3_000): Promise<void> => {
  const end = Date.now() + ms
  while (!check()) {
    if (Date.now() > end) throw new Error('timeout')
    await new Promise((r) => setTimeout(r, 10))
  }
}

function host(over: Partial<ConstructorParameters<typeof CoreServeHost>[0]> = {}) {
  const onFatal = vi.fn()
  const onMessage = vi.fn()
  const s = spawner()
  const h = new CoreServeHost({ bin: 'orrery-core', port, env: { A: '1' }, logFile: path.join(logDir, `c${Math.random()}.log`), onFatal, onMessage, spawner: s.fn, exists: () => true, bootTimeoutMs: 3_000, ...over })
  return { h, onFatal, onMessage, s }
}

describe('core serve mode', () => {
  it('is rust unless CCM_CORE=node', () => {
    expect(coreServeMode({})).toBe('rust')
    expect(coreServeMode({ CCM_CORE: ' Rust ' })).toBe('rust')
    expect(coreServeMode({ CCM_CORE: 'node' })).toBe('node')
    expect(coreServeMode({ CCM_CORE: 'x' })).toBe('rust')
  })
})

describe('CoreServeHost', () => {
  it('spawns serve with host rpc, relays messages, answers host.req and stops on a shutdown line', async () => {
    const pick = vi.fn(async () => ({ result: 'cancelled' as const }))
    const rpc = new HostRpc({ focus: vi.fn(), open: vi.fn(), launch: vi.fn(), pick })
    const { h, onMessage, s } = host({ rpc })
    await h.start()
    const child = s.children[0]!
    expect(child.args).toEqual(['serve'])
    expect(child.env).toMatchObject({ A: '1', CCM_HOST_RPC: '1' })
    expect(h.ownsPort).toBe(true)
    child.stdout.write('{"type":"busy","agents":1,"jobs":0}\nnot json\n{"type":"host.req","id":7,"method":"pickFolder","params":{}}\n')
    await until(() => child.written.includes('"host.res"'))
    expect(onMessage).toHaveBeenCalledWith({ type: 'busy', agents: 1, jobs: 0 })
    expect(onMessage).toHaveBeenCalledTimes(1)
    expect(JSON.parse(child.written.trim().split('\n').pop()!)).toEqual({ type: 'host.res', id: 7, result: { result: 'cancelled' } })
    expect(h.tail).toContain('not json')
    h.post({ type: 'disk.alert', label: 'C:' })
    await until(() => child.written.includes('disk.alert'))
    await h.stop()
    expect(child.written).toContain('{"type":"shutdown"}')
    expect(h.ownsPort).toBe(false)
  })

  it('reports port in use and missing binaries from start without onFatal', async () => {
    const taken = host()
    up = true
    await expect(taken.h.start()).rejects.toMatchObject({ reason: { kind: 'port-in-use' } })
    up = false
    expect(taken.s.children).toHaveLength(0)
    const busy = host({ spawner: spawner([3]).fn })
    await expect(busy.h.start()).rejects.toMatchObject({ reason: { kind: 'port-in-use' } })
    const missing = host({ exists: () => false })
    await expect(missing.h.start()).rejects.toMatchObject({ reason: { kind: 'exited', code: null } })
    for (const x of [taken, busy, missing]) expect(x.onFatal).not.toHaveBeenCalled()
  })

  it('restarts a crash after boot and gives up after 3 with onFatal', async () => {
    const s = spawner()
    const { h, onFatal } = host({ spawner: s.fn })
    await h.start()
    for (let i = 0; i < 4; i++) {
      const n = s.children.length
      s.children[n - 1]!.exit(1)
      if (i < 3) await until(() => s.children.length === n + 1)
    }
    await until(() => onFatal.mock.calls.length > 0, 5_000)
    expect(s.children).toHaveLength(4)
    expect(onFatal).toHaveBeenCalledWith({ kind: 'exited', code: 1 })
    await h.stop()
  })
})

describe('SwitchingHost', () => {
  const fake = (start: () => Promise<void>) => ({ ownsPort: true, tail: ['x'], start: vi.fn(start), stop: vi.fn(async () => undefined), post: vi.fn() })

  it('falls back to node once when rust does not come up', async () => {
    const rust = fake(async () => Promise.reject(new FatalError({ kind: 'timeout' })))
    const node = fake(async () => undefined)
    const onFallback = vi.fn()
    const sw = new SwitchingHost(rust, () => node, onFallback)
    expect(sw.mode).toBe('rust')
    await sw.start()
    expect(sw.mode).toBe('node')
    expect(onFallback).toHaveBeenCalledWith('timeout')
    expect(rust.stop).toHaveBeenCalled()
    expect(sw.describe()).toBe('core serve: node (rust serve unusable, fell back: timeout)')
    await sw.start()
    expect(rust.start).toHaveBeenCalledTimes(1)
    expect(node.start).toHaveBeenCalledTimes(2)
    expect(onFallback).toHaveBeenCalledTimes(1)
    sw.post({ type: 'x' })
    expect(node.post).toHaveBeenCalledWith({ type: 'x' })
  })

  it('keeps rust when it boots and does not fall back on a taken port', async () => {
    const rust = fake(async () => undefined)
    const sw = new SwitchingHost(rust, () => fake(async () => undefined), vi.fn())
    await sw.start()
    expect(sw.describe()).toBe('core serve: rust')
    const taken = new SwitchingHost(fake(async () => Promise.reject(new FatalError({ kind: 'port-in-use' }))), () => fake(async () => undefined), vi.fn())
    await expect(taken.start()).rejects.toBeInstanceOf(FatalError)
    expect(taken.mode).toBe('rust')
    expect(new SwitchingHost(null, () => fake(async () => undefined), vi.fn()).describe()).toBe('core serve: node')
  })

  it('describes failures', () => {
    expect(fallbackReason(new FatalError({ kind: 'exited', code: 101 }))).toBe('exited with code 101')
    expect(fallbackReason(new FatalError({ kind: 'exited', code: null }))).toBe('exited')
    expect(fallbackReason(new Error('boom'))).toBe('boom')
  })
})

describe('serve toasts', () => {
  const full = { type: 'toast.show', tag: 'ccm-1', title: 'T', body: 'B', url: 'agent-monitor://focus?k=1', open: 'Open', dismiss: 'Dismiss', persistent: true }

  it('parses only full toast.show messages', () => {
    expect(parseFullToast(full)).toEqual({ tag: 'ccm-1', title: 'T', body: 'B', url: 'agent-monitor://focus?k=1', open: 'Open', dismiss: 'Dismiss', persistent: true })
    expect(parseFullToast({ ...full, snoozeUrl: 'u', snoozeLabel: 'l' })).toMatchObject({ snoozeUrl: 'u', snoozeLabel: 'l' })
    expect(parseFullToast({ type: 'toast.show', tag: 't', title: 'T', persistent: false })).toBeNull()
    expect(parseFullToast({ ...full, tag: '' })).toBeNull()
    expect(parseFullToast({ ...full, body: 5 })).toBeNull()
  })

  it('draws, hides and reports health after a test toast', async () => {
    const drawer = { show: vi.fn(async () => true), hide: vi.fn(async () => true), health: vi.fn(async () => 'appOff' as const) }
    const post = vi.fn()
    const toasts = createServeToasts(post, drawer)
    expect(toasts.handle(full)).toBe(true)
    expect(toasts.handle({ type: 'toast.hide', tag: 'ccm-1' })).toBe(true)
    expect(toasts.handle({ type: 'busy' })).toBe(false)
    const { toastTag } = await import('../../collector/src/toast')
    expect(toasts.handle({ ...full, tag: toastTag('test') })).toBe(true)
    await until(() => post.mock.calls.length === 1)
    expect(drawer.show).toHaveBeenCalledTimes(2)
    expect(drawer.hide).toHaveBeenCalledWith('ccm-1')
    expect(post).toHaveBeenCalledWith({ type: 'toast.health', health: 'appOff' })
  })
})
