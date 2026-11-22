import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const script = path.join(path.dirname(fileURLToPath(import.meta.url)), 'web-move.mjs')

function fixture(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'web-move-'))
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(root, rel)
    fs.mkdirSync(path.dirname(full), { recursive: true })
    fs.writeFileSync(full, content)
  }
  return root
}

const read = (root, rel) => fs.readFileSync(path.join(root, rel), 'utf8')
const run = (root, args) => spawnSync(process.execPath, [script, '--root', root, '--no-git', ...args], { encoding: 'utf8' })

test('dời file, sửa import tới nó và import bên trong nó', () => {
  const root = fixture({
    'lib/format.ts': "import { x } from './other'\nexport const f = x\n",
    'lib/other.ts': 'export const x = 1\n',
    'lib/use.ts': "import { f } from './format'\nexport { f }\n",
    'components/A.vue': '<script setup lang="ts">\nimport { f } from \'../lib/format\'\nconst m = () => import(\'../lib/format\')\nconst u = new URL(\'../lib/format.ts\', import.meta.url)\nconsole.log(f, m, u)\n</script>\n',
    'components/a.test.ts': "import { vi } from 'vitest'\nvi.mock('../lib/format', () => ({}))\nimport raw from '../lib/format.ts?raw'\nexport { raw }\n",
  })
  const r = run(root, ['lib/format.ts -> core/lib/format.ts'])
  assert.equal(r.status, 0, r.stderr)
  assert.match(r.stdout, /dời 1 file, sửa 7 import trong 4 file/)
  assert.ok(fs.existsSync(path.join(root, 'core/lib/format.ts')))
  assert.ok(!fs.existsSync(path.join(root, 'lib/format.ts')))
  assert.match(read(root, 'core/lib/format.ts'), /from '\.\.\/\.\.\/lib\/other'/)
  assert.match(read(root, 'lib/use.ts'), /from '\.\.\/core\/lib\/format'/)
  const vue = read(root, 'components/A.vue')
  assert.match(vue, /from '\.\.\/core\/lib\/format'/)
  assert.match(vue, /import\('\.\.\/core\/lib\/format'\)/)
  assert.match(vue, /new URL\('\.\.\/core\/lib\/format\.ts'/)
  const t = read(root, 'components/a.test.ts')
  assert.match(t, /vi\.mock\('\.\.\/core\/lib\/format'/)
  assert.match(t, /from '\.\.\/core\/lib\/format\.ts\?raw'/)
})

test('dry không đổi gì', () => {
  const root = fixture({ 'lib/a.ts': 'export const a = 1\n', 'lib/b.ts': "import { a } from './a'\nexport { a }\n" })
  const r = run(root, ['--dry', 'lib/a.ts -> core/a.ts'])
  assert.equal(r.status, 0, r.stderr)
  assert.match(r.stdout, /\[dry\] dời 1 file, sửa 1 import trong 1 file/)
  assert.ok(fs.existsSync(path.join(root, 'lib/a.ts')))
  assert.match(read(root, 'lib/b.ts'), /from '\.\/a'/)
})

test('dời cả thư mục, import nội bộ giữ nguyên, import ngoài được sửa', () => {
  const root = fixture({
    'lib/disk/a.ts': "import { b } from './b'\nimport { u } from '../util'\nexport const a = b + u\n",
    'lib/disk/b.ts': 'export const b = 1\n',
    'lib/util.ts': 'export const u = 1\n',
    'stores/s.ts': "import { a } from '../lib/disk/a'\nexport { a }\n",
  })
  const r = run(root, ['lib/disk -> features/disk/lib'])
  assert.equal(r.status, 0, r.stderr)
  assert.match(r.stdout, /dời 2 file/)
  assert.match(read(root, 'features/disk/lib/a.ts'), /from '\.\/b'/)
  assert.match(read(root, 'features/disk/lib/a.ts'), /from '\.\.\/\.\.\/\.\.\/lib\/util'/)
  assert.match(read(root, 'stores/s.ts'), /from '\.\.\/features\/disk\/lib\/a'/)
})

test('import qua index và alias', () => {
  const root = fixture({
    'lib/pkg/index.ts': "export * from './x'\n",
    'lib/pkg/x.ts': 'export const x = 1\n',
    'app.ts': "import { x } from './lib/pkg'\nimport { y } from '@core/y'\nexport { x, y }\n",
    'core/y.ts': 'export const y = 2\n',
  })
  const r = run(root, ['lib/pkg -> features/pkg', 'core/y.ts -> core/lib/y.ts'])
  assert.equal(r.status, 0, r.stderr)
  const app = read(root, 'app.ts')
  assert.match(app, /from '\.\/features\/pkg'/)
  assert.match(app, /from '@core\/lib\/y'/)
})

test('file map JSON', () => {
  const root = fixture({ 'a.ts': 'export const a = 1\n', 'b.ts': "import { a } from './a'\nexport { a }\n" })
  const map = path.join(root, 'map.json')
  fs.writeFileSync(map, JSON.stringify({ 'a.ts': 'core/a.ts' }))
  const r = run(root, ['--map', map])
  assert.equal(r.status, 0, r.stderr)
  assert.match(read(root, 'b.ts'), /from '\.\/core\/a'/)
})

test('đích đã tồn tại thì báo lỗi, không dời', () => {
  const root = fixture({ 'a.ts': 'export const a = 1\n', 'core/a.ts': 'export const z = 1\n' })
  const r = run(root, ['a.ts -> core/a.ts'])
  assert.notEqual(r.status, 0)
  assert.ok(fs.existsSync(path.join(root, 'a.ts')))
})
