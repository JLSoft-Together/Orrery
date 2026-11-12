import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'

const root = join(dirname(fileURLToPath(import.meta.url)), '../..')
let out = ''

function walk(dir: string, base = ''): string[] {
  return readdirSync(dir).flatMap((name) => {
    const rel = base ? `${base}/${name}` : name
    return statSync(join(dir, name)).isDirectory() ? walk(join(dir, name), rel) : [rel]
  })
}

beforeAll(() => {
  out = mkdtempSync(join(tmpdir(), 'orrery-pwa-dist-'))
  const vite = join(root, '../../node_modules/vite/bin/vite.js')
  const run = spawnSync(process.execPath, [vite, 'build', '--outDir', out, '--emptyOutDir'], { cwd: root, encoding: 'utf8' })
  if (run.status !== 0) throw new Error(run.stdout + run.stderr)
}, 120_000)

afterAll(() => {
  if (out) rmSync(out, { recursive: true, force: true })
})

describe('production build', () => {
  it('has the files the relay contract needs', () => {
    expect(existsSync(join(out, 'index.html'))).toBe(true)
    expect(existsSync(join(out, 'sw.js'))).toBe(true)
    expect(existsSync(join(out, 'manifest.webmanifest'))).toBe(true)
    const assets = walk(out).filter((f) => f.startsWith('assets/'))
    expect(assets.length).toBeGreaterThanOrEqual(3)
    for (const asset of assets) expect(asset).toMatch(/^assets\/.+-[A-Za-z0-9_-]{8,}\.[a-z0-9]+$/)
    expect(assets.some((a) => a.endsWith('.wasm'))).toBe(true)
  })

  it('index.html has no inline script, style, style attribute or event handler and loads nothing remote', () => {
    const html = readFileSync(join(out, 'index.html'), 'utf8')
    const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)]
    expect(scripts.length).toBeGreaterThan(0)
    for (const [, attrs, body] of scripts) {
      expect(attrs).toMatch(/\bsrc="\/app\/assets\//)
      expect((body ?? '').trim()).toBe('')
    }
    expect(html).not.toMatch(/<style\b/i)
    expect(html).not.toMatch(/\sstyle\s*=/i)
    expect(html).not.toMatch(/\son[a-z]+\s*=/i)
    expect(html).not.toMatch(/(?:src|href)="(?:https?:)?\/\//)
    expect(html).toContain('viewport-fit=cover')
    expect(html).toContain('apple-mobile-web-app-capable')
    expect(html).toContain('rel="apple-touch-icon"')
  })

  it('manifest scope and start_url are /app/ and the icons exist', () => {
    const manifest = JSON.parse(readFileSync(join(out, 'manifest.webmanifest'), 'utf8')) as { scope: string; start_url: string; display: string; icons: Array<{ src: string; sizes: string }> }
    expect(manifest).toMatchObject({ scope: '/app/', start_url: '/app/', display: 'standalone' })
    expect(manifest.icons.map((i) => i.sizes).sort()).toEqual(['192x192', '512x512'])
    for (const icon of manifest.icons) expect(existsSync(join(out, icon.src))).toBe(true)
  })

  it('ships no dev tooling and nothing that needs unsafe-eval', () => {
    for (const file of walk(out).filter((f) => /\.(js|html|css)$/.test(f))) {
      const text = readFileSync(join(out, file), 'utf8')
      expect(text.toLowerCase(), file).not.toContain('eruda')
      expect(text, file).not.toMatch(/new Function\(|\beval\(/)
    }
  })

  it('service worker precaches every built file under /app/ and has the push hooks', () => {
    const sw = readFileSync(join(out, 'sw.js'), 'utf8')
    const list = JSON.parse(/const FILES = (\[.*\])/.exec(sw)?.[1] ?? '[]') as string[]
    expect(list).toContain('/app/')
    for (const file of walk(out).filter((f) => f !== 'sw.js' && f !== 'index.html')) expect(list).toContain(`/app/${file}`)
    expect(sw).toContain("addEventListener('push'")
    expect(sw).toContain("addEventListener('notificationclick'")
    expect(sw).not.toContain('__')
  })
})
