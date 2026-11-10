import http from 'node:http'
import type { AddressInfo } from 'node:net'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerHttpIpc } from '../src/preview/httpSend'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const BIG = Buffer.alloc(6 * 1024 * 1024, 0x61)

describe.skipIf(!HAVE_DESK)('desk previewTools http twin (TS fetch vs orrery-core desk)', () => {
  let twin: Twin
  let server: http.Server
  let base = ''
  let closed = ''
  const asked: string[] = []
  let node: ReturnType<typeof fakeIpc>
  let rust: ReturnType<typeof fakeIpc>
  const ev = { sender: { id: 5 } }

  beforeAll(async () => {
    server = http.createServer((req, res) => {
      const chunks: Buffer[] = []
      req.on('data', (c: Buffer) => chunks.push(c))
      req.on('end', () => {
        const url = new URL(req.url ?? '/', 'http://x')
        const send = (status: number, headers: Record<string, string | string[]>, body: string | Buffer = ''): void => {
          res.writeHead(status, headers)
          res.end(body)
        }
        if (url.pathname === '/echo') {
          const echoed = { method: req.method, path: req.url, body: Buffer.concat(chunks).toString('utf8'), x: req.headers['x-test'] ?? null, type: req.headers['content-type'] ?? null }
          return send(200, { 'content-type': 'application/json; charset=utf-8', 'x-a': 'one', 'x-b': 'two' }, JSON.stringify(echoed))
        }
        if (url.pathname === '/redirect') return send(302, { location: '/echo', 'set-cookie': ['a=1', 'b=2'] })
        if (url.pathname === '/png') return send(200, { 'content-type': 'image/png' }, Buffer.from([1, 2, 3, 4]))
        if (url.pathname === '/bom') return send(200, { 'content-type': 'text/plain' }, Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from('héllo')]))
        if (url.pathname === '/big') return send(200, { 'content-type': 'text/plain' }, BIG)
        if (url.pathname === '/empty') return send(204, {})
        if (url.pathname === '/slow') return void setTimeout(() => send(200, {}, 'late'), 3000)
        return send(404, { 'content-type': 'text/plain' }, 'missing')
      })
    })
    await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
    const dead = http.createServer()
    await new Promise<void>((ok) => dead.listen(0, '127.0.0.1', ok))
    closed = `http://127.0.0.1:${(dead.address() as AddressInfo).port}`
    await new Promise((ok) => dead.close(ok))
    twin = await startTwin({})
    node = fakeIpc()
    rust = fakeIpc()
    const confirmHost = async (_e: unknown, host: string, method: string): Promise<boolean> => {
      asked.push(`${host} ${method}`)
      return false
    }
    registerHttpIpc({ ipcMain: node.ipcMain, isAppFrame: () => true, confirmHost })
    registerHttpIpc({ ipcMain: rust.ipcMain, isAppFrame: () => true, confirmHost, desk: twin.router })
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    server?.close()
  })

  const strip = (v: unknown): unknown =>
    JSON.parse(JSON.stringify(v), (k, x) => (k === 'timeMs' ? 0 : k === 'headers' && Array.isArray(x) ? x.filter(([n]: [string]) => !['date', 'connection', 'keep-alive'].includes(n)) : x))

  const same = async (req: unknown): Promise<any> => {
    const a = (await node.call('ccm:http-send', ev, req)) as { body?: string }
    const b = (await rust.call('ccm:http-send', ev, req)) as { body?: string }
    expect(strip(b), JSON.stringify(req).slice(0, 120)).toEqual(strip(a))
    return b
  }

  const get = (p: string, extra: Record<string, unknown> = {}) => ({ method: 'GET', url: `${base}${p}`, headers: [], ...extra })

  it('sends and reads responses the same', async () => {
    expect(twin.host.channels).toEqual(expect.arrayContaining(['ccm:http-send', 'ccm:http-cancel']))
    const echo = await same({ method: 'post', url: `${base}/echo?q=é&b=1`, headers: [['X-Test', 'v1'], ['Content-Type', 'text/plain']], body: 'héllo wörld' })
    expect(JSON.parse(echo.body)).toMatchObject({ method: 'POST', path: '/echo?q=%C3%A9&b=1', body: 'héllo wörld', x: 'v1', type: 'text/plain' })
    await same(get('/echo', { method: 'put', headers: [['x-test', 'a'], ['X-Test', 'b']], body: '{"a":1}' }))
    await same(get('/echo', { method: 'DELETE' }))
    await same(get('/echo', { method: 'PATCH', body: '' }))
    await same(get('/echo#frag'))
    await same(get('/redirect'))
    await same(get('/png'))
    await same(get('/bom'))
    await same(get('/empty'))
    await same(get('/missing'))
    await same(get('/echo', { id: 'abc' }))
    const big = await same(get('/big'))
    expect(big).toMatchObject({ ok: true, truncated: true, size: 5 * 1024 * 1024 })
  })

  it('refuses and fails the same', async () => {
    await same(get('/echo', { body: 'x' }))
    await same({ method: 'HEAD', url: `${base}/echo`, headers: [], body: 'x' })
    await same(get('/echo', { method: 'g e' }))
    await same({ method: 'GET', url: 'ftp://x/y', headers: [] })
    await same({ method: 'GET', url: 'not a url', headers: [] })
    await same({ method: 'GET', url: `${base}/`, headers: [['bad name', 'x']] })
    await same({ method: 'GET', url: `${base}/`, headers: [['x', 'a\nb']] })
    await same({ method: 'GET', url: `${base}/`, headers: 'x' })
    await same({ method: 'GET', url: `${base}/`, headers: [], body: 5 })
    await same(null)
    expect(await same({ method: 'GET', url: `${closed}/`, headers: [] })).toEqual({ ok: false, error: 'network', detail: 'ECONNREFUSED' })
  })

  it('asks once per public host and cancels the same', async () => {
    for (const [h, s] of [[node, 'node'], [rust, 'rust']] as const) {
      asked.length = 0
      const pub = { method: 'GET', url: 'http://example.com/x', headers: [] }
      expect(await h.call('ccm:http-send', ev, pub), s).toEqual({ ok: false, error: 'declined' })
      expect(await h.call('ccm:http-send', ev, { ...pub, method: 'POST' }), s).toEqual({ ok: false, error: 'declined' })
      expect(asked, s).toEqual(['example.com GET', 'example.com POST'])
      const local = h.call('ccm:http-send', ev, get('/slow', { id: 'k1' }))
      await new Promise((res) => setTimeout(res, 300))
      expect(await h.call('ccm:http-cancel', ev, 'k1'), s).toBe(true)
      expect(await h.call('ccm:http-cancel', ev, 'k1'), s).toBe(false)
      expect(await local, s).toEqual({ ok: false, error: 'cancelled' })
      expect(await h.call('ccm:http-cancel', ev, 5), s).toBe(false)
      const first = h.call('ccm:http-send', ev, get('/slow', { id: 'k2' }))
      await new Promise((res) => setTimeout(res, 200))
      const second = h.call('ccm:http-send', ev, get('/echo', { id: 'k2' }))
      expect(await first, s).toEqual({ ok: false, error: 'cancelled' })
      expect(await second, s).toMatchObject({ ok: true, status: 200 })
    }
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
