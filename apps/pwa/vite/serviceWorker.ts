import { createHash } from 'node:crypto'
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { join, relative, resolve, sep } from 'node:path'
import type { Plugin } from 'vite'

const SKIP = new Set(['sw.js', 'push-sw.js', 'version.json'])

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name)
    return statSync(full).isDirectory() ? walk(full) : [full]
  })
}

export function serviceWorkerPlugin(): Plugin {
  let outDir = ''
  let root = ''
  let base = '/'
  return {
    name: 'ccm-pwa-sw',
    apply: 'build',
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir)
      root = config.root
      base = config.base
    },
    closeBundle() {
      const files = walk(outDir)
        .map((file) => relative(outDir, file).split(sep).join('/'))
        .filter((rel) => !SKIP.has(rel) && !rel.endsWith('.map'))
        .sort()
      const digest = createHash('sha256')
      for (const rel of files) digest.update(rel).update('\0').update(readFileSync(join(outDir, rel))).update('\0')
      const cache = `orrery-pwa-${digest.digest('hex').slice(0, 12)}`
      const urls = [base, ...files.filter((rel) => rel !== 'index.html').map((rel) => `${base}${rel}`)]
      const template = readFileSync(join(root, 'vite/sw.template.js'), 'utf8')
      const source = template.replace('__CACHE__', cache).replace('__SHELL__', base).replace('__FILES__', JSON.stringify(urls)).replace('__BASE__', base)
      writeFileSync(join(outDir, 'sw.js'), source)
    },
  }
}
