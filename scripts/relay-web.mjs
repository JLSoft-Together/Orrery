import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.woff2': 'font/woff2',
  '.woff': 'font/woff',
  '.ttf': 'font/ttf',
  '.txt': 'text/plain; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
}

const HASHED = /^assets\/.+[-.][A-Za-z0-9_-]{8,}\.[a-z0-9]+$/

const walk = (dir, base = '') =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const rel = base ? `${base}/${e.name}` : e.name
    return e.isDirectory() ? walk(path.join(dir, e.name), rel) : [rel]
  })

export const STUB = 'import type { WebBundle } from "./web-bundle";\n\nexport const WEB: WebBundle | null = null;\n'

export function buildWebModule(distDir) {
  if (!distDir || !existsSync(distDir) || !statSync(distDir).isDirectory()) return null
  const rels = walk(distDir).filter((r) => !r.endsWith('.map')).sort()
  if (!rels.includes('index.html')) throw new Error(`${distDir} has no index.html`)
  const all = createHash('sha256')
  const files = {}
  let raw = 0
  for (const rel of rels) {
    const ext = path.extname(rel).toLowerCase()
    const type = TYPES[ext]
    if (!type) throw new Error(`no content-type for ${rel}`)
    const bytes = readFileSync(path.join(distDir, rel))
    raw += bytes.length
    const digest = createHash('sha256').update(bytes).digest('hex')
    all.update(rel).update('\0').update(digest).update('\0')
    files[rel] = {
      type,
      cache: HASHED.test(rel) ? 'immutable' : 'revalidate',
      etag: `"${digest.slice(0, 32)}"`,
      data: bytes.toString('base64'),
    }
  }
  const hash = all.digest('hex').slice(0, 16)
  const source = `import type { WebBundle } from "./web-bundle";\n\nexport const WEB: WebBundle | null = ${JSON.stringify({ hash, files })};\n`
  return { source, hash, raw, count: rels.length }
}
