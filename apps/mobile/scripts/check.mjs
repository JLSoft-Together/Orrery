import { spawnSync } from 'node:child_process'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const args = process.argv.slice(2)
const has = (f) => args.includes(f)
const opt = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]
const threads = Number(process.env.JIN_MAX_THREADS) || 4
const maxLines = Number(opt('max-lines')) || 400
const win = process.platform === 'win32'
let failed = false

function run(cmd, list, quiet = true) {
  const r = spawnSync(cmd, list, { cwd: root, encoding: 'utf8', shell: win, maxBuffer: 64 * 1024 * 1024 })
  return { code: r.status ?? 1, out: `${r.stdout ?? ''}${r.stderr ?? ''}`, quiet }
}

function step(name, fn) {
  const t = Date.now()
  const { ok, lines } = fn()
  if (!ok) failed = true
  console.log(`${ok ? 'OK  ' : 'FAIL'} ${name} (${((Date.now() - t) / 1000).toFixed(1)}s)`)
  for (const l of lines.slice(0, 40)) console.log(`     ${l}`)
  if (lines.length > 40) console.log(`     … ${lines.length - 40} more`)
}

function changedDart() {
  const r = run('git', ['status', '--porcelain', '--untracked-files=all', '--', 'lib', 'test'])
  return r.out
    .split('\n')
    .map((l) => l.slice(3).trim())
    .filter((f) => f.endsWith('.dart') && !f.includes('/l10n/') && !f.includes('/src/rust/'))
    .map((f) => f.replace(/^apps\/mobile\//, ''))
}

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) walk(p, out)
    else if (p.endsWith('.dart')) out.push(p)
  }
  return out
}

if (!has('--no-l10n')) {
  step('gen-l10n', () => {
    const r = run('flutter', ['gen-l10n'])
    return { ok: r.code === 0, lines: r.code === 0 ? [] : r.out.trim().split('\n') }
  })
}

if (!has('--no-format')) {
  step('dart format (changed)', () => {
    const files = changedDart()
    if (!files.length) return { ok: true, lines: ['no changed files'] }
    const r = run('dart', ['format', ...files])
    const changed = r.out.split('\n').filter((l) => l.startsWith('Formatted ') && !/^Formatted \d+ files?/.test(l))
    return { ok: r.code === 0, lines: r.code === 0 ? changed : r.out.trim().split('\n') }
  })
}

if (!has('--no-analyze')) {
  step('flutter analyze', () => {
    const r = run('flutter', ['analyze', '--no-pub'])
    const issues = r.out.split('\n').map((l) => l.trim()).filter((l) => /^(error|warning|info) •/.test(l))
    return { ok: r.code === 0, lines: issues.length ? issues : r.code === 0 ? [] : r.out.trim().split('\n').slice(-10) }
  })
}

if (!has('--no-test') && (!failed || has('--force-test'))) {
  step('flutter test', () => {
    const extra = opt('test') ? opt('test').split(',') : []
    const r = run('flutter', ['test', `--concurrency=${threads}`, '--reporter=compact', ...extra])
    const lines = r.out.split(/\r?\n|\r/).map((l) => l.trim()).filter(Boolean)
    const last = lines.filter((l) => /^\d+:\d+ \+\d+/.test(l)).at(-1) ?? lines.at(-1) ?? ''
    const fails = [...new Set(lines.filter((l) => /\[E\]$/.test(l) && !/: loading /.test(l)).map((l) => l.replace(/^\d+:\d+ \+\d+( ~\d+)?( -\d+)?: /, '')))]
    const errors = r.code === 0 ? [] : [...new Set(lines.filter((l) => /^(Expected|Actual):|^lib\/.*Error:/.test(l)).map((l) => l.slice(0, 220)))].slice(0, 20)
    return { ok: r.code === 0, lines: [last.replace(/^\d+:\d+ /, ''), ...fails, ...errors] }
  })
}

step(`files > ${maxLines} lines (lib, hand-written)`, () => {
  const long = walk(join(root, 'lib'))
    .filter((p) => !p.includes(`${join('lib', 'l10n')}`) && !p.includes(join('src', 'rust')))
    .map((p) => [relative(root, p).replaceAll('\\', '/'), readFileSync(p, 'utf8').split('\n').length - 1])
    .filter(([, n]) => n > maxLines)
    .sort((a, b) => b[1] - a[1])
  return { ok: true, lines: long.map(([f, n]) => `${n}  ${f}`) }
})

process.exit(failed ? 1 : 0)
