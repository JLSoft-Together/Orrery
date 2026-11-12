import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { createServer } from 'node:http'
import { extname, join } from 'node:path'

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
}

function walk(dir, base = '') {
  return readdirSync(dir).flatMap((name) => {
    const rel = base ? `${base}/${name}` : name
    return statSync(join(dir, name)).isDirectory() ? walk(join(dir, name), rel) : [rel]
  })
}

export function distHash(dist) {
  const all = createHash('sha256')
  for (const rel of walk(dist).sort()) all.update(rel).update('\0').update(readFileSync(join(dist, rel))).update('\0')
  return all.digest('hex').slice(0, 16)
}

export function cspFor(relayWs) {
  return [
    "default-src 'none'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self'",
    "img-src 'self' data: blob:",
    "font-src 'self'",
    `connect-src 'self' ${relayWs} wss://*:4320 https://*:4320`,
    "worker-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join('; ')
}

export function startStatic({ dist, relayWs }) {
  const state = { web: distHash(dist), requests: [] }
  const csp = cspFor(relayWs)
  const server = createServer((req, res) => {
    const url = new URL(req.url, 'http://x')
    state.requests.push(url.pathname)
    const headers = { 'content-security-policy': csp, 'x-content-type-options': 'nosniff', 'referrer-policy': 'no-referrer' }
    if (url.pathname === '/__version') {
      state.web = url.searchParams.get('web') ?? state.web
      res.writeHead(204).end()
      return
    }
    if (url.pathname === '/app/version.json') {
      res.writeHead(200, { ...headers, 'content-type': TYPES['.json'], 'cache-control': 'no-store' })
      res.end(JSON.stringify({ web: state.web, proto: 1, minProto: 1 }))
      return
    }
    if (!url.pathname.startsWith('/app')) {
      res.writeHead(404, headers).end()
      return
    }
    const rest = decodeURIComponent(url.pathname.slice(5))
    let file = join(dist, rest)
    if (rest.includes('..') || !existsSync(file) || statSync(file).isDirectory()) {
      if (extname(rest)) {
        res.writeHead(404, headers).end()
        return
      }
      file = join(dist, 'index.html')
    }
    const immutable = /^assets\/.+-[A-Za-z0-9_-]{8,}\./.test(rest)
    res.writeHead(200, {
      ...headers,
      'content-type': TYPES[extname(file)] ?? 'application/octet-stream',
      'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'no-cache',
      ...(file.endsWith('sw.js') ? { 'service-worker-allowed': '/app/' } : {}),
    })
    res.end(readFileSync(file))
  })
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({ server, state, port: server.address().port })))
}
