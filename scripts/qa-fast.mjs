import { spawn, execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const argv = process.argv.slice(2)
const flag = (name) => argv.includes(`--${name}`)
const opt = (name) => argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3)

const HELP = `node scripts/qa-fast.mjs [options]
  --base=<ref>      so sánh với ref này (mặc định: thay đổi chưa commit + file mới)
  --e2e=a,b         chạy thêm các E2E này (tên file trong apps/desktop/e2e, bỏ .cjs)
  --no-e2e          bỏ E2E
  --no-cargo        bỏ cargo test
  --clippy          chạy thêm cargo clippy -D warnings cho crate đang đổi
  --all-e2e-map     in bảng map file → E2E rồi thoát
  --electron        E2E chạy Electron (mặc định Tauri)
  --jobs=<n>        số E2E chạy song song (mặc định 3)
  --dry             chỉ in kế hoạch`
if (flag('help') || flag('h')) {
  console.log(HELP)
  process.exit(0)
}

const E2E_MAP = [
  [/bugReport|bug_report|errorLog|SettingsAbout|DiagnosticsDialog/, ['v90-bug-report']],
  [/extensions[\\/]|SessionToolsSheet|sessionTools|useSessionTools/, ['v89-session-tools', 'v89-pane']],
  [/contextWarn|ContextTokens|useNotifications|BoardCard/, ['v89-pane']],
  [/stores[\\/]toasts|ToastHost/, ['v80-toasts']],
  [/[\\/]explorer[\\/]|Explorer/, ['explorer']],
  [/GitLog|[\\/]gitlog[\\/]|[\\/]git[\\/]/, ['log']],
  [/LayoutPreview|[\\/]layout[\\/]/, ['layout-speed']],
  [/drop_paths|DeviceFilesDialog|pathsForFiles/, ['v25']],
  [/csp/, ['csp']],
  [/useCompact|compact/i, ['v51-compact']],
  [/themes|style\.css|ThemePicker|Appearance/, ['v91-themes']],
  [/termIme|TermPane\.vue|DropTerminal\.vue/, ['term-ime']],
]
if (flag('all-e2e-map')) {
  for (const [re, list] of E2E_MAP) console.log(`${re.source}  →  ${list.join(', ')}`)
  process.exit(0)
}

const git = (...a) => execFileSync('git', a, { cwd: REPO, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).split('\n').map((s) => s.trim()).filter(Boolean)
const base = opt('base')
const changed = [...new Set([...(base ? git('diff', '--name-only', `${base}...HEAD`) : []), ...git('diff', '--name-only', 'HEAD'), ...git('ls-files', '--others', '--exclude-standard')])]
  .filter((f) => !/(^|\/)(\.vitest|node_modules|target|dist|stage|release)\//.test(f) && existsSync(path.join(REPO, f)))

const WS = {
  'apps/web': '@ccm/web',
  'apps/desktop': '@ccm/desktop',
  'apps/collector': '@ccm/collector',
  'packages/shared': '@ccm/shared',
}
const SRC = /\.(ts|tsx|vue|mts|cts|js|mjs)$/
const wsOf = (f) => Object.keys(WS).find((w) => f.startsWith(w + '/'))
const touchedWs = new Set(changed.filter((f) => SRC.test(f)).map(wsOf).filter(Boolean))
const typecheckWs = touchedWs.has('packages/shared') ? Object.keys(WS) : [...touchedWs]
const relatedBy = {}
for (const f of changed) {
  const w = wsOf(f)
  if (w && SRC.test(f)) (relatedBy[w] ??= []).push(path.relative(w, f).replace(/\\/g, '/'))
}
const crates = new Set()
let shell = false
for (const f of changed) {
  const m = f.match(/^apps\/core\/crates\/([^/]+)\//)
  if (m && /\.rs$|Cargo\.toml$|\.json$/.test(f)) crates.add(m[1])
  if (/^apps\/shell\/(src\/.*\.rs$|Cargo\.toml$|build\.rs$|permissions\/)/.test(f)) shell = true
}

const e2e = new Set(opt('e2e')?.split(',').map((s) => s.trim()).filter(Boolean) ?? [])
if (!flag('no-e2e')) for (const f of changed) for (const [re, list] of E2E_MAP) if (re.test(f)) list.forEach((x) => e2e.add(x))
for (const f of changed) {
  const m = f.match(/^apps\/desktop\/e2e\/([^/]+)\.cjs$/)
  if (m && m[1] !== 'cdp' && !flag('no-e2e')) e2e.add(m[1])
}
const e2eFile = (x) => ['apps/desktop/e2e', 'apps/shell/e2e'].map((d) => path.join(d, x + '.cjs')).find((p) => existsSync(path.join(REPO, p)))
for (const x of [...e2e]) if (!e2eFile(x)) e2e.delete(x)
const tauri = !flag('electron')
const webChanged = touchedWs.has('apps/web') || touchedWs.has('packages/shared')
const needShellBuild = tauri && e2e.size > 0 && (shell || crates.size > 0 || changed.includes('apps/shell/src/shim.js'))
const threads = Number(process.env.JIN_MAX_THREADS) || Math.max(2, Math.floor(os.cpus().length * 0.6))
const jobs = Math.max(1, Number(opt('jobs')) || 3)

const plan = [
  `file đổi: ${changed.length}`,
  `typecheck: ${typecheckWs.map((w) => WS[w]).join(', ') || '-'}`,
  `vitest related: ${Object.entries(relatedBy).map(([w, l]) => `${WS[w]} (${l.length})`).join(', ') || '-'}`,
  `cargo test: ${flag('no-cargo') ? 'bỏ' : [...[...crates].map((c) => `-p ${c}`), shell ? 'apps/shell' : ''].filter(Boolean).join(', ') || '-'}`,
  `E2E (${tauri ? 'tauri' : 'electron'}, ${jobs} song song): ${[...e2e].join(', ') || '-'}${e2e.size && webChanged ? ' · vite build' : ''}${needShellBuild ? ' · cargo build shell' : ''}`,
]
console.log(plan.join('\n'))
if (flag('dry') || !changed.length && !e2e.size) process.exit(0)

const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx'
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm'
const results = []
function run(name, cmd, args, opts = {}) {
  const t0 = Date.now()
  return new Promise((resolve) => {
    let out = ''
    const child = spawn(cmd, args, { cwd: opts.cwd ?? REPO, env: { ...process.env, ...opts.env }, shell: process.platform === 'win32' && cmd.endsWith('.cmd'), windowsHide: true })
    child.stdout.on('data', (d) => (out += d))
    child.stderr.on('data', (d) => (out += d))
    child.on('close', (code) => {
      const r = { name, ok: code === 0, s: ((Date.now() - t0) / 1000).toFixed(1), out }
      results.push(r)
      console.log(`${r.ok ? 'PASS' : 'FAIL'} ${name} (${r.s}s)`)
      resolve(r)
    })
  })
}
async function pool(tasks, n) {
  const queue = [...tasks]
  await Promise.all(Array.from({ length: Math.min(n, queue.length) }, async (_, w) => { while (queue.length) await queue.shift()(w) }))
}

const T0 = Date.now()
const checks = []
for (const w of typecheckWs) checks.push(() => run(`typecheck ${WS[w]}`, npm, ['run', 'typecheck', '-w', WS[w]]))
for (const [w, files] of Object.entries(relatedBy)) checks.push(() => run(`vitest related ${WS[w]}`, npx, ['vitest', 'related', '--run', '--passWithNoTests', '--retry=1', ...files], { cwd: path.join(REPO, w) }))
if (!flag('no-cargo')) {
  if (crates.size) checks.push(() => run(`cargo test ${[...crates].join(' ')}`, 'cargo', ['test', '-j', String(threads), ...[...crates].flatMap((c) => ['-p', c])], { cwd: path.join(REPO, 'apps/core') }))
  if (shell) checks.push(() => run('cargo test shell', 'cargo', ['test', '-j', String(threads)], { cwd: path.join(REPO, 'apps/shell') }))
  if (flag('clippy') && crates.size) checks.push(() => run(`cargo clippy ${[...crates].join(' ')}`, 'cargo', ['clippy', '-j', String(threads), ...[...crates].flatMap((c) => ['-p', c]), '--', '-D', 'warnings'], { cwd: path.join(REPO, 'apps/core') }))
  if (flag('clippy') && shell) checks.push(() => run('cargo clippy shell', 'cargo', ['clippy', '-j', String(threads), '--', '-D', 'warnings'], { cwd: path.join(REPO, 'apps/shell') }))
}
if (e2e.size && webChanged) checks.push(() => run('vite build web', npx, ['vite', 'build'], { cwd: path.join(REPO, 'apps/web') }))
if (needShellBuild) checks.push(() => run('cargo build shell', 'cargo', ['build', '-j', String(threads)], { cwd: path.join(REPO, 'apps/shell') }))
await pool(checks, Math.max(2, Math.min(4, Math.floor(threads / 2))))

const buildFailed = results.some((r) => !r.ok && /build/.test(r.name))
if (e2e.size && !buildFailed) {
  const outRoot = path.join(os.tmpdir(), 'ccm-qa-fast')
  mkdirSync(outRoot, { recursive: true })
  const tasks = [...e2e].map((name) => (i) => {
    return run(`e2e ${name}`, process.execPath, [e2eFile(name), ...(tauri ? ['--shell=tauri'] : [])], {
      env: { CCM_E2E_PORT: String(4399 + i * 10), CCM_E2E_DBG: String(9333 + i * 10), CCM_E2E_OUT: path.join(outRoot, name) },
    })
  })
  await pool(tasks, jobs)
} else if (e2e.size) console.log('bỏ E2E: build lỗi')

const fails = results.filter((r) => !r.ok)
for (const r of fails) {
  const lines = r.out.split('\n').filter((l) => /FAIL|Error|error\[|error TS|✗|×|panicked|AssertionError/.test(l)).slice(0, 25)
  console.log(`\n── ${r.name} ──\n${(lines.length ? lines : r.out.split('\n').slice(-25)).join('\n')}`)
}
console.log(`\n${results.length - fails.length}/${results.length} pass · ${((Date.now() - T0) / 1000).toFixed(0)}s${e2e.size ? ` · ảnh E2E: ${path.join(os.tmpdir(), 'ccm-qa-fast')}` : ''}`)
process.exit(fails.length ? 1 : 0)
