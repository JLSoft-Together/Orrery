import { spawn } from 'node:child_process'
import { createWriteStream, existsSync, mkdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const argv = process.argv.slice(2)
const flag = (name) => argv.includes(`--${name}`)
const opt = (name) => argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3)

if (flag('help') || flag('h')) {
  console.log(`node scripts/qa-remote.mjs [--only=pwa,rust,e2e,mobile] [--no-mobile] [--browsers=chromium,webkit] [--no-build]
  pwa     vitest @ccm/pwa + vue-tsc
  rust    cargo test -j 6 remote:: (apps/shell)
  e2e     PWA E2E scenario=remote (pair, layout 375/390/430, chat default, Fit screen, chat steps)
  mobile  node apps/mobile/scripts/check.mjs
Log: .jin/logs/qa-remote-<time>.log, failure shots: .jin/logs/qa-remote-<time>-shots/`)
  process.exit(0)
}

const ALL = ['pwa', 'rust', 'e2e', 'mobile']
const only = opt('only') ? opt('only').split(',').map((s) => s.trim()) : ALL
const parts = ALL.filter((p) => only.includes(p) && !(p === 'mobile' && flag('no-mobile')))
const stamp = new Date().toISOString().replace(/[-:]/g, '').replace('T', '-').slice(0, 15)
const logDir = path.join(REPO, '.jin/logs')
mkdirSync(logDir, { recursive: true })
const logFile = path.join(logDir, `qa-remote-${stamp}.log`)
const shotDir = path.join(logDir, `qa-remote-${stamp}-shots`)
const log = createWriteStream(logFile)
const win = process.platform === 'win32'

function run(title, cmd, args, opts = {}) {
  return new Promise((resolve) => {
    log.write(`\n===== ${title}: ${cmd} ${args.join(' ')}\n`)
    const child = spawn(cmd, args, { cwd: opts.cwd ?? REPO, env: { ...process.env, JIN_RAW: '1', FORCE_COLOR: '0', NO_COLOR: '1', ...opts.env }, shell: win && cmd !== process.execPath })
    let tail = ''
    const take = (d) => {
      const s = String(d).replace(/\x1b\[[0-9;]*m/g, '')
      log.write(s)
      tail = (tail + s).slice(-20000)
    }
    child.stdout.on('data', take)
    child.stderr.on('data', take)
    child.on('close', (code) => resolve({ code: code ?? 1, tail }))
  })
}

const pick = (text, re) => {
  const m = text.match(re)
  return m ? m[0].replace(/\s+/g, ' ').trim() : ''
}

const results = []
async function part(name, fn) {
  const t = Date.now()
  const r = await fn()
  results.push({ name, ok: r.ok, note: r.note, s: ((Date.now() - t) / 1000).toFixed(0) })
}

if (parts.includes('pwa')) {
  await part('pwa vitest', async () => {
    const r = await run('vitest', 'npm', ['test', '-w', '@ccm/pwa'])
    return { ok: r.code === 0, note: pick(r.tail, /Tests\s+[^\n]+/g) || `exit ${r.code}` }
  })
  await part('pwa typecheck', async () => {
    const r = await run('vue-tsc', 'npm', ['run', 'typecheck', '-w', '@ccm/pwa'])
    return { ok: r.code === 0, note: r.code === 0 ? 'clean' : (r.tail.match(/error TS\d+/g) ?? []).length + ' errors' }
  })
}

if (parts.includes('rust')) {
  await part('rust remote', async () => {
    const r = await run('cargo', 'cargo', ['test', '-j', '6', 'remote::'], { cwd: path.join(REPO, 'apps/shell') })
    const sums = [...r.tail.matchAll(/test result: (\w+)\. (\d+) passed; (\d+) failed/g)]
    const pass = sums.reduce((n, m) => n + Number(m[2]), 0)
    const fail = sums.reduce((n, m) => n + Number(m[3]), 0)
    return { ok: r.code === 0, note: sums.length ? `${pass} pass, ${fail} fail` : `exit ${r.code}` }
  })
}

if (parts.includes('e2e')) {
  await part('pwa e2e remote', async () => {
    const args = ['apps/pwa/e2e/run.mjs', '--scenario=remote', `--browsers=${opt('browsers') ?? 'chromium,webkit'}`, `--failDir=${shotDir}`]
    if (flag('no-build')) args.push('--no-build')
    const r = await run('e2e', process.execPath, args)
    const fails = [...r.tail.matchAll(/^FAIL (.+)$/gm)].map((m) => m[1].slice(0, 160))
    return { ok: r.code === 0, note: [pick(r.tail, /e2e: [^\n]+/g) || `exit ${r.code}`, ...fails.slice(0, 4)].join('\n       ') }
  })
}

if (parts.includes('mobile')) {
  await part('mobile check', async () => {
    const script = path.join(REPO, 'apps/mobile/scripts/check.mjs')
    if (!existsSync(script)) return { ok: false, note: 'apps/mobile/scripts/check.mjs missing' }
    const r = await run('mobile', process.execPath, [script], { cwd: path.join(REPO, 'apps/mobile') })
    const lines = r.tail.split(/\r?\n/).filter((l) => /^(OK|FAIL)/.test(l))
    return { ok: r.code === 0, note: lines.slice(0, 4).join('\n       ') || `exit ${r.code}` }
  })
}

log.end()
for (const r of results) console.log(`${r.ok ? 'OK  ' : 'FAIL'} ${r.name} (${r.s}s) ${r.note}`)
const bad = results.filter((r) => !r.ok).length
console.log(`qa-remote: ${results.length - bad} ok, ${bad} fail; log ${path.relative(REPO, logFile)}${bad && existsSync(shotDir) ? `; shots ${path.relative(REPO, shotDir)}` : ''}`)
process.exitCode = bad ? 1 : 0
