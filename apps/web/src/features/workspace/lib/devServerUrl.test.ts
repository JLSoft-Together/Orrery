import { describe, expect, it } from 'vitest'
import { DevServerDetector } from '@features/resources'
import { DEV_SCAN_BYTES, DevOpenGuard, devScanSlice } from './devServerUrl'

const scan = (log: string): string[] => new DevServerDetector().feed(log)

describe('dev server banners', () => {
  const table: Array<[string, string, string]> = [
    ['vite', '\n  VITE v7.1.0  ready in 312 ms\n\n  \x1b[32m➜\x1b[39m  \x1b[1mLocal\x1b[22m:   \x1b[36mhttp://localhost:\x1b[1m5173\x1b[22m/\x1b[39m\n  ➜  Network: http://192.168.1.20:5173/\n', 'http://localhost:5173/'],
    ['vite preview', '  ➜  Local:   http://localhost:4173/\n  ➜  Network: use --host to expose\n', 'http://localhost:4173/'],
    ['next', '   ▲ Next.js 15.1.0\n   - Local:        http://localhost:3000\n   - Network:      http://192.168.0.5:3000\n\n ✓ Ready in 1.2s\n', 'http://localhost:3000/'],
    ['nuxt', '  ➜ Local:    http://localhost:3000/\n  ➜ Network:  http://10.0.0.7:3000/\n', 'http://localhost:3000/'],
    ['angular', '** Angular Live Development Server is listening on localhost:4200, open your browser on http://localhost:4200/ **\n', 'http://localhost:4200/'],
    ['angular v17', '  ➜  Local:   http://localhost:4200/\n', 'http://localhost:4200/'],
    ['webpack-dev-server', '<i> [webpack-dev-server] Project is running at:\n<i> [webpack-dev-server] Loopback: http://localhost:8080/\n<i> [webpack-dev-server] On Your Network (IPv4): http://172.20.10.3:8080/\n', 'http://localhost:8080/'],
    ['expo web', 'Starting Metro Bundler\nWeb is waiting on http://localhost:8081\n', 'http://localhost:8081/'],
    ['flutter web', 'Launching lib/main.dart on Web Server in debug mode...\nlib/main.dart is being served at http://localhost:54321\n', 'http://localhost:54321/'],
    ['python http.server', 'Serving HTTP on 0.0.0.0 port 8000 (http://0.0.0.0:8000/) ...\n', 'http://localhost:8000/'],
    ['ipv6', '  ➜  Local:   http://[::1]:5173/\n', 'http://localhost:5173/'],
  ]

  for (const [name, log, expected] of table) {
    it(`${name}`, () => {
      expect(scan(log)[0]).toBe(expected)
    })
  }

  it('prefers Local over Network on the same port', () => {
    expect(scan('  ➜  Local:   http://localhost:5173/\n  ➜  Network: http://192.168.1.20:5173/\n')).toEqual(['http://localhost:5173/'])
  })

  it('accepts a Network-only banner on a LAN ip', () => {
    expect(scan('  Network: http://192.168.1.20:3000/\n')).toEqual(['http://192.168.1.20:3000/'])
  })

  it('ignores request logs', () => {
    expect(scan('Server ready\nGET http://localhost:3000/api/users 200 12ms\n')).toEqual([])
    expect(scan('127.0.0.1 - - [23/Nov/2026 10:00:00] "GET / HTTP/1.1" 200 -\nlistening\n')).toEqual([])
    expect(scan('ready\n[nuxt] POST http://localhost:3000/api/x 201\n')).toEqual([])
    expect(scan('Fetching http://localhost:3000/health\n')).toEqual([])
  })

  it('ignores public hosts', () => {
    expect(scan('Local: https://example.com:3000/\n')).toEqual([])
    expect(scan('Network: http://8.8.8.8:3000/\n')).toEqual([])
  })
})

describe('devScanSlice', () => {
  it('keeps only the first 4 KB of a chunk', () => {
    expect(devScanSlice(new Uint8Array(100)).length).toBe(100)
    expect(devScanSlice(new Uint8Array(DEV_SCAN_BYTES * 3)).length).toBe(DEV_SCAN_BYTES)
  })
})

describe('DevOpenGuard', () => {
  it('lets each tab open a url once', () => {
    const g = new DevOpenGuard()
    expect(g.take('t1', 'http://localhost:5173/')).toBe(true)
    expect(g.take('t1', 'http://localhost:5173/')).toBe(false)
    expect(g.take('t2', 'http://localhost:5173/')).toBe(true)
    expect(g.take('t1', 'http://localhost:3000/')).toBe(true)
    g.dropTab('t1')
    expect(g.take('t1', 'http://localhost:5173/')).toBe(true)
  })
})
