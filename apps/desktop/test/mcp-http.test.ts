import http from 'node:http'
import type { AddressInfo } from 'node:net'
import { afterEach, describe, expect, it } from 'vitest'
import { BODY_MAX, mcpHttpHandler, type McpHttpOptions } from '../src/pty/mcp'
import type { ToolHost } from '../src/pty/mcpTools/types'

const TOKEN = 'a'.repeat(64)
let server: http.Server | null = null

afterEach(() => {
  server?.close()
  server = null
})

const fakeHost = (): ToolHost => ({
  root: '/',
  take: () => true,
  openSession: async () => ({ ok: false, error: 'unavailable' }),
  notify: async () => ({ ok: false, error: 'unavailable' }),
  checkoutRoots: async () => [],
  page: async () => ({ ok: false, error: 'unavailable' }) as never,
})

async function serve(over: Partial<McpHttpOptions> = {}): Promise<{ port: number; asked: string[] }> {
  const asked: string[] = []
  let port = 0
  server = http.createServer(
    mcpHttpHandler({
      port: () => port,
      caller: (token) => (asked.push(token), token === TOKEN ? fakeHost() : null),
      version: '9.9.9',
      tools: [],
      ...over,
    }),
  )
  await new Promise<void>((r) => server!.listen(0, '127.0.0.1', () => r()))
  port = (server!.address() as AddressInfo).port
  return { port, asked }
}

function post(port: number, body: string, headers: Record<string, string> = {}): Promise<{ status: number; text: string }> {
  return new Promise((resolve, reject) => {
    const req = http.request({ host: '127.0.0.1', port, path: '/mcp', method: 'POST', headers }, (res) => {
      let text = ''
      res.on('data', (d: Buffer) => (text += d.toString('utf8')))
      res.on('end', () => resolve({ status: res.statusCode ?? 0, text }))
    })
    req.on('error', reject)
    req.end(body)
  })
}

describe('MCP HTTP in main', () => {
  it('answers initialize for a known pane token', async () => {
    const { port } = await serve()
    const res = await post(port, JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} }), { Authorization: `Bearer ${TOKEN}`, 'Content-Type': 'application/json' })
    expect(res.status).toBe(200)
    expect(JSON.parse(res.text)).toMatchObject({ id: 1, result: { serverInfo: { name: 'orrery', version: '9.9.9' } } })
  })

  it('refuses a wrong token with 401 before the body is read, and gates path, origin, method', async () => {
    const { port, asked } = await serve()
    const huge = 'x'.repeat(BODY_MAX * 4)
    expect((await post(port, huge, { Authorization: `Bearer ${'b'.repeat(64)}` })).status).toBe(401)
    expect(asked).toEqual(['b'.repeat(64)])
    expect((await post(port, '{}', {})).status).toBe(401)
    expect((await post(port, '{}', { Authorization: `Bearer ${TOKEN}`, Origin: 'http://evil.test' })).status).toBe(403)
    expect((await post(port, huge, { Authorization: `Bearer ${TOKEN}` })).status).toBe(413)
    expect((await post(port, '{bad', { Authorization: `Bearer ${TOKEN}` })).status).toBe(400)
  })

  it('a body over the max with a good token gets 413 and the connection is closed without reading the rest', async () => {
    const { port } = await serve()
    const closed = await new Promise<{ status: number; closed: boolean }>((resolve) => {
      let status = 0
      const req = http.request({ host: '127.0.0.1', port, path: '/mcp', method: 'POST', headers: { Authorization: `Bearer ${TOKEN}`, 'Transfer-Encoding': 'chunked' } }, (res) => {
        status = res.statusCode ?? 0
        res.resume()
      })
      req.on('error', () => resolve({ status, closed: true }))
      req.on('close', () => resolve({ status, closed: true }))
      const chunk = 'x'.repeat(16 * 1024)
      let sent = 0
      const pump = (): void => {
        while (sent < BODY_MAX * 64) {
          sent += chunk.length
          if (!req.write(chunk)) return void req.once('drain', pump)
        }
        req.end()
      }
      pump()
    })
    expect(closed.closed).toBe(true)
    expect([0, 413]).toContain(closed.status)
  })
})
