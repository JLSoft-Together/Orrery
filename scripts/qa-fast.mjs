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
  --e2e=a,b         chạy thêm các E2E này (tên file trong apps/shell/e2e, bỏ .cjs)
  --no-e2e          bỏ E2E
  --no-cargo        bỏ cargo test
  --all-e2e-map     in bảng map file → E2E rồi thoát
  --jobs=<n>        số E2E chạy song song (mặc định 3)
  --dry             chỉ in kế hoạch`
if (flag('help') || flag('h')) {
  console.log(HELP)
  process.exit(0)
}

const E2E_MAP = [
  [/bugReport|bug_report|errorLog|SettingsAbout|DiagnosticsDialog/, ['v90-bug-report']],
  [/features[\\/]extensions[\\/]components[\\/]|SessionToolsSheet|sessionTools|useSessionTools/, ['v89-session-tools', 'v89-pane']],
  [/contextWarn|ContextTokens|useNotifications|BoardCard/, ['v89-pane']],
  [/stores[\\/]toasts|ToastHost/, ['v80-toasts']],
  [/[\\/]explorer[\\/]|Explorer/, ['explorer']],
  [/GitLog|[\\/]gitlog[\\/]|[\\/]git[\\/]/, ['log']],
  [/LayoutPreview|[\\/]layout[\\/]/, ['layout-speed']],
  [/drop_paths|DeviceFilesDialog|pathsForFiles/, ['v25']],
  [/csp/, ['csp']],
  [/useCompact|compact/i, ['v51-compact']],
  [/themes|style\.css|ThemePicker|Appearance|palettes|uiGen/, ['v91-themes', 'v109-themes-all']],
  [/termIme|TermPane\.vue|DropTerminal\.vue/, ['term-ime']],
  [/features[\\/]deepseek[\\/]|deepseek|DeepSeek/, ['v104-deepseek-lazy']],
  [/clash|Clash|agentTouches|RailRowItem|railLive/, ['v104-clash']],
  [/features[\\/]rail[\\/]lib[\\/]railMenu|LiveRail|WorkspaceSidebar|ProjectMenu/, ['v109-project-menu']],
  [/useClusterDrag|clusterReorder|railOrder|railView|WorkspaceSidebar|RailClusterView|LiveRail|railLive|usePaneDrag|useRail\.ts/, ['rail-reorder']],
  [/inbox|Inbox|menuQueue|MenuToast|useInboxGlobal|remoteSummary/, ['v104-inbox', 'v100-menu-queue']],
  [/turnEdits|turnEdges|TurnChanges|SessionFilterBar|ChangesPane|reviewQueue/, ['v104-turn-changes']],
  [/features[\\/]notify[\\/]|useNotifications|SettingsNotify|apps[\\/]shell[\\/]src[\\/]badge\.rs|core-toast/, ['v109-notify']],
  [/apps[\\/]shell[\\/]src[\\/](toast|menu_toast|power|episodes)\.rs/, ['v76', 'v100-menu-toast']],
  [/features[\\/]shortcuts[\\/]|workspaceKeys|paletteKey|lib[\\/]shortcuts|ShortcutsDialog|MruSwitcher/, ['v109-shortcuts']],
  [/footerSys|sys_sample|NextQuotaBar|WorkspaceFooter/, ['v109-footer-sys']],
  [/features[\\/]shellDone[\\/]|useShellMarks|shellMarks|core-pty[\\/]src[\\/](marks|hub)|apps[\\/]shell[\\/]src[\\/](shell_init|presets|pty)\.rs|core-toast[\\/]src[\\/]toaster[\\/]shell/, ['v109-shell-done']],
  [/features[\\/]diffComments[\\/]|DiffView\.vue|SplitDiff\.vue|ChangesPane/, ['v109-diff-comments']],
  [/features[\\/]chat[\\/]|chat_ipc|remote[\\/]chat_src|core-transcript[\\/]src[\\/]chat[\\/]|TermPane\.vue/, ['v109-chat-view']],
  [/features[\\/]termSearch[\\/]|TermFind\.vue|TermPane\.vue|workspaceKeys/, ['v109-term-search']],
  [/features[\\/]secretGuard[\\/]|CommitBox|PushDialog|preview[\\/](secrets|ipc_git)|core-transcript[\\/]src[\\/]mask\.rs/, ['v109-secret-guard']],
  [/core-monitor[\\/]src[\\/]native[\\/]|apps[\\/]shell[\\/]src[\\/]folder_pick\.rs|core-serve[\\/]src[\\/](lib|host_ops)\.rs/, ['v110-external-open']],
  [/features[\\/]devices[\\/]/, ['devices', 'v97-device', 'v25']],
  [/features[\\/]disk[\\/]/, ['v101-diskmap-ui', 'v101-diskclean', 'v101-dupes', 'v101-organize']],
  [/features[\\/]remote[\\/]/, ['v97-remote', 'v98-remote-next']],
  [/features[\\/]files[\\/]/, ['explorer', 'file-editor']],
  [/features[\\/]git[\\/]/, ['log', 'diff']],
  [/features[\\/]agent-tools[\\/]/, ['v104-clash', 'v104-turn-changes', 'v89-pane']],
  [/features[\\/]shareMode[\\/]|core[\\/]lib[\\/](shareMode|mask)\.ts|ShareBar|ShareModeChip|GitHubAccountsSettings|core-toast[\\/]src[\\/]toaster[\\/]mod/, ['v109-share-mode']],
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
  const m = f.match(/^apps\/shell\/e2e\/([^/]+)\.cjs$/)
  if (m && m[1] !== 'cdp' && !flag('no-e2e')) e2e.add(m[1])
}
const e2eFile = (x) => [path.join('apps/shell/e2e', x + '.cjs')].find((p) => existsSync(path.join(REPO, p)))
for (const x of [...e2e]) if (!e2eFile(x)) e2e.delete(x)
const webChanged = touchedWs.has('apps/web') || touchedWs.has('packages/shared')
const needShellBuild = e2e.size > 0 && (shell || crates.size > 0 || changed.includes('apps/shell/src/shim.js'))
const threads = Number(process.env.JIN_MAX_THREADS) || Math.max(2, Math.floor(os.cpus().length * 0.6))
const jobs = Math.max(1, Number(opt('jobs')) || 3)

const plan = [
  `file đổi: ${changed.length}`,
  `typecheck: ${typecheckWs.map((w) => WS[w]).join(', ') || '-'}`,
  `vitest related: ${Object.entries(relatedBy).map(([w, l]) => `${WS[w]} (${l.length})`).join(', ') || '-'}`,
  `cargo test: ${flag('no-cargo') ? 'bỏ' : [...[...crates].map((c) => `-p ${c}`), shell ? 'apps/shell' : ''].filter(Boolean).join(', ') || '-'}`,
  `E2E (tauri, ${jobs} song song): ${[...e2e].join(', ') || '-'}${e2e.size && webChanged ? ' · vite build' : ''}${needShellBuild ? ' · cargo build shell' : ''}`,
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
}
if (e2e.size && webChanged) checks.push(() => run('vite build web', npx, ['vite', 'build'], { cwd: path.join(REPO, 'apps/web') }))
if (needShellBuild) checks.push(() => run('cargo build shell', 'cargo', ['build', '-j', String(threads)], { cwd: path.join(REPO, 'apps/shell') }))
await pool(checks, Math.max(2, Math.min(4, Math.floor(threads / 2))))

const buildFailed = results.some((r) => !r.ok && /build/.test(r.name))
if (e2e.size && !buildFailed) {
  const outRoot = path.join(os.tmpdir(), 'ccm-qa-fast')
  mkdirSync(outRoot, { recursive: true })
  const tasks = [...e2e].map((name) => (i) => {
    return run(`e2e ${name}`, process.execPath, [e2eFile(name)], {
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
