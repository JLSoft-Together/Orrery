import path from 'node:path'
import { describe, expect, it, vi } from 'vitest'
import { LaunchError } from '../../collector/src/launcher'
import { HostRpc, validDir, validPid, type HostHandlers } from '../src/host-rpc'

const dir = path.resolve('work', 'app')

function handlers(over: Partial<HostHandlers> = {}): HostHandlers {
  return {
    focus: vi.fn(async () => 'ok' as const),
    open: vi.fn(async () => 'ok' as const),
    launch: vi.fn(async () => undefined),
    pick: vi.fn(async () => ({ result: 'ok' as const, dir })),
    ...over,
  }
}

const req = (id: number, method: string, params: unknown) => ({ type: 'host.req', id, method, params })

describe('host rpc validation', () => {
  it('accepts only real pids and absolute clean dirs', () => {
    expect(validPid(5)).toBe(true)
    for (const bad of [4, 0, -1, 1.5, '12', null, 2 ** 32, Number.NaN]) expect(validPid(bad)).toBe(false)
    expect(validDir(dir)).toBe(true)
    for (const bad of ['', 'relative/dir', `${dir}\u0007`, dir + 'a'.repeat(1100), 5, null]) expect(validDir(bad)).toBe(false)
  })

  it('ignores lines that are not host requests', async () => {
    const rpc = new HostRpc(handlers())
    expect(await rpc.handleLine('not json')).toBeNull()
    expect(await rpc.handleLine(JSON.stringify({ type: 'shutdown' }))).toBeNull()
    expect(await rpc.handleLine(JSON.stringify({ type: 'host.req', id: 'x', method: 'focus' }))).toBeNull()
    expect(await rpc.handleLine(JSON.stringify({ type: 'host.req', id: -1, method: 'focus' }))).toBeNull()
  })

  it('answers bad params and unknown methods with an error and never calls a handler', async () => {
    const h = handlers()
    const rpc = new HostRpc(h)
    for (const m of [req(1, 'focus', { pid: 2 }), req(2, 'focus', {}), req(3, 'open', { dir: 'rel', app: 'vscode' }), req(4, 'open', { dir, app: 'rm' }), req(5, 'launch', { dir, mode: 'x' }), req(6, 'launch', { dir, mode: 'new', app: 'zsh' }), req(7, 'exec', { cmd: 'calc' }), req(8, 'launch', { dir: `${dir}\n`, mode: 'new' })]) {
      expect(await rpc.handle(m)).toEqual({ type: 'host.res', id: m.id, error: 'bad request' })
    }
    expect(h.focus).not.toHaveBeenCalled()
    expect(h.open).not.toHaveBeenCalled()
    expect(h.launch).not.toHaveBeenCalled()
  })
})

describe('host rpc methods', () => {
  it('focus returns the handler result and failure becomes failed', async () => {
    const rpc = new HostRpc(handlers({ focus: vi.fn(async () => 'ambiguous' as const) }))
    expect(await rpc.handle(req(1, 'focus', { pid: 4242 }))).toEqual({ type: 'host.res', id: 1, result: { result: 'ambiguous' } })
    const bad = new HostRpc(handlers({ focus: vi.fn(async () => Promise.reject(new Error('x'))) }))
    expect(await bad.handle(req(2, 'focus', { pid: 4242 }))).toEqual({ type: 'host.res', id: 2, result: { result: 'failed' } })
  })

  it('focus requests run one at a time', async () => {
    const order: string[] = []
    let release!: () => void
    const gate = new Promise<void>((r) => (release = r))
    const rpc = new HostRpc(
      handlers({
        focus: vi.fn(async (pid: number) => {
          order.push(`start ${pid}`)
          if (pid === 10) await gate
          order.push(`end ${pid}`)
          return 'ok' as const
        }),
      }),
    )
    const a = rpc.handle(req(1, 'focus', { pid: 10 }))
    const b = rpc.handle(req(2, 'focus', { pid: 11 }))
    await new Promise((r) => setTimeout(r, 20))
    expect(order).toEqual(['start 10'])
    release()
    await Promise.all([a, b])
    expect(order).toEqual(['start 10', 'end 10', 'start 11', 'end 11'])
  })

  it('open passes dir and app', async () => {
    const h = handlers({ open: vi.fn(async () => 'noApp' as const) })
    const rpc = new HostRpc(h)
    expect(await rpc.handle(req(1, 'open', { dir, app: 'vscode' }))).toEqual({ type: 'host.res', id: 1, result: { result: 'noApp' } })
    expect(h.open).toHaveBeenCalledWith(dir, 'vscode')
  })

  it('launch maps errors to codes', async () => {
    const cases: [unknown, unknown][] = [
      [undefined, { ok: true }],
      [new LaunchError('denied'), { ok: false, error: 'denied' }],
      [new LaunchError('notInstalled'), { ok: false, error: 'notInstalled' }],
      [new Error('directory not found'), { ok: false, error: 'notFound' }],
      [new Error('spawn boom'), { ok: false, error: 'failed' }],
    ]
    for (const [err, want] of cases) {
      const h = handlers({ launch: vi.fn(async () => (err ? Promise.reject(err) : undefined)) })
      expect(await new HostRpc(h).handle(req(1, 'launch', { dir, mode: 'continue', app: 'iterm' }))).toEqual({ type: 'host.res', id: 1, result: want })
      expect(h.launch).toHaveBeenCalledWith(dir, 'continue', 'iterm')
    }
  })

  it('pickFolder: one dialog at a time, ok needs a valid dir', async () => {
    let release!: () => void
    const gate = new Promise<void>((r) => (release = r))
    const h = handlers({
      pick: vi.fn(async () => {
        await gate
        return { result: 'ok' as const, dir }
      }),
    })
    const rpc = new HostRpc(h)
    const first = rpc.handle(req(1, 'pickFolder', {}))
    expect(await rpc.handle(req(2, 'pickFolder', {}))).toEqual({ type: 'host.res', id: 2, result: { result: 'busy' } })
    release()
    expect(await first).toEqual({ type: 'host.res', id: 1, result: { result: 'ok', dir } })
    const bad = new HostRpc(handlers({ pick: vi.fn(async () => ({ result: 'ok' as const, dir: 'rel' })) }))
    expect(await bad.handle(req(3, 'pickFolder', {}))).toEqual({ type: 'host.res', id: 3, result: { result: 'failed' } })
    const cancelled = new HostRpc(handlers({ pick: vi.fn(async () => ({ result: 'cancelled' as const, dir })) }))
    expect(await cancelled.handle(req(4, 'pickFolder', {}))).toEqual({ type: 'host.res', id: 4, result: { result: 'cancelled' } })
  })

  it('handleLine returns one NDJSON line', async () => {
    const rpc = new HostRpc(handlers())
    const out = await rpc.handleLine(JSON.stringify(req(9, 'focus', { pid: 77 })))
    expect(out?.endsWith('\n')).toBe(true)
    expect(JSON.parse(out as string)).toEqual({ type: 'host.res', id: 9, result: { result: 'ok' } })
  })
})
