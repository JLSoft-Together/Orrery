import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const RELAY = path.join(ROOT, 'apps', 'relay')
const OUT = path.join(ROOT, 'apps', 'shell', 'resources', 'relay')
const stripJsonc = (text) => {
  let out = ''
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if (c === '"') {
      let j = i + 1
      while (text[j] !== '"') j += text[j] === '\\' ? 2 : 1
      out += text.slice(i, j + 1)
      i = j
    } else if (c === '/' && text[i + 1] === '/') {
      while (i < text.length && text[i] !== '\n') i++
      out += '\n'
    } else if (c === '/' && text[i + 1] === '*') {
      i = text.indexOf('*/', i + 2) + 1
    } else out += c
  }
  return out.replace(/,(\s*[}\]])/g, '$1')
}
const config = JSON.parse(stripJsonc(readFileSync(path.join(RELAY, 'wrangler.jsonc'), 'utf8')))

const tmp = mkdtempSync(path.join(tmpdir(), 'relay-bundle-'))
try {
  const env = { ...process.env, CLOUDFLARE_ACCOUNT_ID: config.account_id }
  const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx'
  execFileSync(npx, ['wrangler', 'deploy', '--dry-run', '--outdir', tmp], { cwd: RELAY, env, stdio: 'inherit', shell: process.platform === 'win32' })
  const worker = readFileSync(path.join(tmp, 'index.js'), 'utf8').replace(/\r\n/g, '\n').replace(/\n\/\/# sourceMappingURL=.*\n?$/, '\n')

  const bindings = [
    ...config.durable_objects.bindings.map((b) => ({ type: 'durable_object_namespace', name: b.name, class_name: b.class_name })),
    ...config.ratelimits.map((r) => ({ type: 'ratelimit', name: r.name, namespace_id: r.namespace_id, simple: r.simple })),
    ...Object.entries(config.vars).map(([name, text]) => ({ type: 'plain_text', name, text })),
  ]
  const meta = {
    mainModule: 'worker.js',
    compatibilityDate: config.compatibility_date,
    migrations: config.migrations.map((m) => ({ tag: m.tag, new_sqlite_classes: m.new_sqlite_classes ?? [] })),
    bindings,
  }
  const hash = createHash('sha256').update(worker).update(JSON.stringify(meta)).digest('hex').slice(0, 16)

  mkdirSync(OUT, { recursive: true })
  writeFileSync(path.join(OUT, 'worker.js'), worker)
  writeFileSync(path.join(OUT, 'worker.json'), JSON.stringify({ hash, ...meta }, null, 2) + '\n')
  console.log(`relay bundle ${hash} (${worker.length} bytes) -> ${path.relative(ROOT, OUT)}`)
} finally {
  rmSync(tmp, { recursive: true, force: true })
}
