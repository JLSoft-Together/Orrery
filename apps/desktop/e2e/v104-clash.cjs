const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4481'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9481'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v104d')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const SP = `${S}.get('toolSpans')`
const SET = `${S}.get('settings')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const QA = process.env.CCM_E2E_QA_DIR || path.resolve(__dirname, '../../../docs/qa/v104/WP-104d')

const REPO = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-clash-')))
fs.mkdirSync(path.join(REPO, 'src'))
const FILE = path.join(REPO, 'src', 'a.ts')
fs.writeFileSync(FILE, 'export const a = 1\n')
fs.writeFileSync(path.join(REPO, 'src', 'b.ts'), 'export const b = 1\n')
let hasGit = true
try {
  const git = (...args) => execFileSync('git', args, { cwd: REPO, stdio: 'pipe' })
  git('init', '-q')
  git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'add', '.')
  git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'commit', '-qm', 'init')
  fs.writeFileSync(FILE, 'export const a = 2\n')
  fs.writeFileSync(path.join(REPO, 'src', 'b.ts'), 'export const b = 2\n')
} catch {
  hasGit = false
}

let seq = 0
const spanJs = (terminalId, file) => `${SP}.upsert({ id: 'e2e-span-${++seq}', terminalId: '${terminalId}', agentId: '${terminalId}:main', tool: 'Edit', start: new Date().toISOString(), path: ${JSON.stringify(file)}, access: 'edit' })`
const counts = `(() => ({ rail: document.querySelectorAll('[data-side-row] [data-clash-chip]').length, head: document.querySelectorAll('[data-pane-head] [data-clash-chip]').length, toast: document.querySelectorAll('[data-clash-toast]').length }))()`

async function run(app) {
  const { evaluate, waitFor, shot, sleep, cmd } = app
  await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
  await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.notify.clash'); localStorage.removeItem('ccm.clash.ignored')`)
  await cmd('Page.reload')
  await sleep(1500)
  await waitFor(`!!${S}.get('ui') && ${S}.get('connection').hasData`, 30000, 'reloaded with data')
  await evaluate(`${S}.get('ui').view = 'workspace'`)
  check('switch is off by default', (await evaluate(`${SET}.clashWarn`)) === false)

  await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
  await waitFor(`${W}.panes.filter((p) => p.state === 'running').length >= 1`, 25000, 'first shell')
  await evaluate(`${W}.split(${W}.panes[0].key, 'row', { kind: 'term', preset: 'shell' }, { cwd: ${JSON.stringify(REPO)} })`)
  await waitFor(`${W}.panes.filter((p) => p.state === 'running').length >= 2`, 25000, 'second shell')
  await sleep(800)
  await evaluate(`(() => {
    ${T}.upsert({ id: 'e2e-clash-a', title: 'Alpha', status: 'working', cwd: ${JSON.stringify(REPO)} })
    ${T}.upsert({ id: 'e2e-clash-b', title: 'Beta', status: 'working', cwd: ${JSON.stringify(REPO)} })
    const terms = ${W}.panes.filter((p) => !p.kind || p.kind === 'term')
    ${W}.update(terms[0].key, { preset: 'claude', terminalId: 'e2e-clash-a' })
    ${W}.update(terms[1].key, { preset: 'claude', terminalId: 'e2e-clash-b' })
    ${spanJs('e2e-clash-a', FILE)}
    ${spanJs('e2e-clash-b', FILE)}
  })()`)
  await sleep(800)
  const off = await evaluate(counts)
  check('switch off: no chip on rail or header, no toast', off.rail === 0 && off.head === 0 && off.toast === 0, JSON.stringify(off))
  check('switch off: explorer index still marks the conflict', (await evaluate(`${S}.get('agentTouches').index.files.get(${JSON.stringify(FILE.replace(/\\/g, '/').toLowerCase())})?.state`)) === 'conflict')

  await evaluate(`${S}.get('ui').settingsPage = 'notify'; ${S}.get('ui').settingsOpen = true`)
  await waitFor(`!!document.querySelector('[data-setting-row="clash"]')`, 10000, 'clash row').catch(() => {})
  check('Settings > Notifications has the clash switch', (await evaluate(`document.querySelectorAll('[data-setting-row="clash"]').length`)) === 1)
  await evaluate(`document.querySelector('[data-setting-row="clash"] [role=switch]').click()`)
  await sleep(300)
  check('switch on persists ccm.notify.clash=1', (await evaluate(`localStorage.getItem('ccm.notify.clash')`)) === '1')
  await evaluate(`${S}.get('ui').settingsOpen = false`)
  await waitFor(`document.querySelectorAll('[data-clash-toast]').length === 1`, 8000, 'toast').catch(() => {})
  const on = await evaluate(counts)
  check('switch on: chip on both rail rows', on.rail === 2, JSON.stringify(on))
  check('switch on: chip on both pane headers', on.head === 2, JSON.stringify(on))
  check('switch on: exactly one toast', on.toast === 1, JSON.stringify(on))
  const toastText = await evaluate(`document.querySelector('[data-clash-toast]')?.textContent ?? ''`)
  check('toast names both sessions and the relative file', /Alpha/.test(toastText) && /Beta/.test(toastText) && toastText.includes('src/a.ts'), toastText.slice(0, 160))
  const railTip = await evaluate(`document.querySelector('[data-side-row] [data-clash-chip]')?.closest('[data-side-row]')?.querySelector('[data-rail-open]')?.title ?? ''`)
  check('rail row tooltip lists peer and file', railTip.includes('src/a.ts'), railTip.slice(0, 160))
  for (const theme of ['galaxy', 'orrery-light']) {
    await evaluate(`${SET}.theme = '${theme}'`)
    await sleep(500)
    await shot(`v104d-01-chips-${theme === 'galaxy' ? 'dark' : 'light'}`)
  }

  await evaluate(`${S}.get('toasts').list.filter((t) => t.id.startsWith('clash-')).forEach((t) => ${S}.get('toasts').remove(t.id))`)
  await sleep(300)
  await evaluate(spanJs('e2e-clash-a', FILE))
  await evaluate(spanJs('e2e-clash-b', FILE))
  await sleep(1200)
  check('same turn: no second toast', (await evaluate(`document.querySelectorAll('[data-clash-toast]').length`)) === 0)

  await evaluate(`${T}.upsert({ id: 'e2e-clash-a', title: 'Alpha', status: 'idle', cwd: ${JSON.stringify(REPO)} })`)
  await sleep(200)
  await evaluate(`${T}.upsert({ id: 'e2e-clash-a', title: 'Alpha', status: 'working', cwd: ${JSON.stringify(REPO)} })`)
  await sleep(200)
  await evaluate(spanJs('e2e-clash-a', FILE))
  await waitFor(`document.querySelectorAll('[data-clash-toast]').length === 1`, 5000, 'toast again').catch(() => {})
  check('new turn of one side: toast once more', (await evaluate(`document.querySelectorAll('[data-clash-toast]').length`)) === 1)

  await evaluate(`document.querySelector('[data-clash-toast] [data-clash-toast-ignore]').click()`)
  await sleep(400)
  const ignored = await evaluate(counts)
  check('Ignore this file: chips and toast gone', ignored.rail === 0 && ignored.head === 0 && ignored.toast === 0, JSON.stringify(ignored))
  check('Ignore this file: stored in ccm.clash.ignored', (await evaluate(`JSON.parse(localStorage.getItem('ccm.clash.ignored') ?? '[]').length`)) === 1)
  check('Undo toast shown', (await evaluate(`!!document.querySelector('[data-clash-undo]')`)) === true)
  await evaluate(`document.querySelector('[data-clash-undo]').click()`)
  await sleep(400)
  const undone = await evaluate(counts)
  check('Undo brings the chips back', undone.rail === 2 && undone.head === 2, JSON.stringify(undone))

  await evaluate(`document.querySelector('[data-pane-head] [data-clash-chip]').click()`)
  await waitFor(`!!document.querySelector('[data-clash-pop]')`, 5000, 'popover').catch(() => {})
  check('header chip opens the file list', (await evaluate(`document.querySelectorAll('[data-clash-pop] [data-clash-row]').length`)) === 1)
  check('popover offers Split into a worktree', (await evaluate(`!!document.querySelector('[data-clash-pop] [data-clash-worktree]')`)) === true)
  const pop = await evaluate(`(() => { const r = document.querySelector('[data-clash-pop]').getBoundingClientRect(); return { l: r.left, r: r.right, w: innerWidth } })()`)
  check('popover stays inside the window', pop.l >= 0 && pop.r <= pop.w, JSON.stringify(pop))
  await evaluate(`${SET}.theme = 'galaxy'`)
  await sleep(300)
  await shot('v104d-02-popover-dark')
  await evaluate(`${SET}.theme = 'orrery-light'`)
  await sleep(300)
  await shot('v104d-02-popover-light')
  await evaluate(`document.querySelector('[data-clash-pop] [data-clash-diff]').click()`)
  await waitFor(`${W}.panes.some((p) => p.kind === 'changes')`, 8000, 'changes pane').catch(() => {})
  const changes = await evaluate(`JSON.stringify(${W}.panes.filter((p) => p.kind === 'changes').map((p) => ({ select: p.select, cwd: p.cwd })))`)
  check('View diff opens Changes for the repo', JSON.parse(changes).length === 1, changes)
  if (hasGit) {
    await waitFor(`[...document.querySelectorAll('[aria-selected="true"]')].some((n) => /a\\.ts/.test(n.textContent))`, 10000, 'selected file').catch(() => {})
    const sel = await evaluate(`[...document.querySelectorAll('[aria-selected="true"]')].map((n) => n.textContent.trim()).filter((s) => /\\.ts/.test(s)).join('|')`)
    check('Changes selects src/a.ts', /a\.ts/.test(sel) && !/b\.ts/.test(sel), sel)
    await evaluate(`${SET}.theme = 'galaxy'`)
    await sleep(300)
    await shot('v104d-03-diff-dark')
    await evaluate(`${SET}.theme = 'orrery-light'`)
    await sleep(300)
    await shot('v104d-03-diff-light')
  } else console.log('SKIP git not found: Changes selection')

  await evaluate(`${SET}.clashWarn = false`)
  await sleep(300)
  const offAgain = await evaluate(counts)
  check('switch off again hides every chip', offAgain.rail === 0 && offAgain.head === 0, JSON.stringify(offAgain))
  await evaluate(`(() => { ${T}.remove('e2e-clash-a'); ${T}.remove('e2e-clash-b') })()`)
}

;(async () => {
  const app = await launch({ ui: 'next' })
  try {
    await run(app)
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  } finally {
    app.stop()
    try {
      fs.mkdirSync(QA, { recursive: true })
      for (const f of fs.readdirSync(OUT).filter((n) => n.startsWith('v104d-') && n.endsWith('.png'))) fs.copyFileSync(path.join(OUT, f), path.join(QA, f))
    } catch {}
    fs.rmSync(REPO, { recursive: true, force: true, maxRetries: 5 })
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
