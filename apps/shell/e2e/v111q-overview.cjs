const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4626'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9626'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v111q-overview')
const { launch } = require('./cdp.cjs')
const REPO = path.resolve(__dirname, '../../..')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v111/QA')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-111q-ov-')))
const git = (cwd, ...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', '-c', 'core.autocrlf=false', ...args], { cwd, stdio: 'pipe', encoding: 'utf8' }).trim()

function makeRepo(name) {
  const dir = path.join(BASE, name)
  fs.mkdirSync(dir, { recursive: true })
  git(dir, 'init', '-q', '-b', 'main')
  for (const [i, msg] of ['init', 'feat: add login form', 'fix: trim email'].entries()) {
    fs.writeFileSync(path.join(dir, 'a.txt'), `line ${i}\n`.repeat(i + 1))
    git(dir, 'add', '-A')
    git(dir, 'commit', '-q', '-m', msg)
  }
  fs.writeFileSync(path.join(dir, 'a.txt'), 'changed\n')
  return dir
}

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const a = makeRepo('alpha')
  const b = makeRepo('beta')
  const app = await launch({ env: { CCM_VERIFY_PROCESSES: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, sleep, cmd } = app
  const snap = async (name) => {
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    if (r.result?.data) fs.writeFileSync(path.join(SHOTS, `${name}.png`), Buffer.from(r.result.data, 'base64'))
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    const p1 = await evaluate(`${W}.open('shell', ${JSON.stringify(a)}, undefined, { tab: true }).key`)
    const p2 = await evaluate(`${W}.open('shell', ${JSON.stringify(b)}, undefined, { tab: true }).key`)
    await waitFor(`${W}.panes.find((p) => p.key === '${p1}')?.state === 'running' && ${W}.panes.find((p) => p.key === '${p2}')?.state === 'running'`, 30000, 'shells')
    await evaluate(`${T}.upsert({ id: 'ov-1', title: 'Agent alpha', status: 'working', cwd: ${JSON.stringify(a)} }); ${T}.upsert({ id: 'ov-2', title: 'Agent beta', status: 'waiting', cwd: ${JSON.stringify(b)} })`)
    await evaluate(`${W}.update('${p1}', { preset: 'claude', terminalId: 'ov-1', focusedAt: 1 }); ${W}.update('${p2}', { preset: 'codex', terminalId: 'ov-2', focusedAt: 2 })`)
    await sleep(2000)
    check('rail shows both projects', await evaluate(`document.body.textContent.includes('alpha') && document.body.textContent.includes('beta')`))
    check('footer (rail foot or workspace footer) present', await evaluate(`!!document.querySelector('[data-ws-footer], [data-next-rail-foot]')`))
    check('agent pane header has a more menu', await evaluate(`!!document.querySelector('[data-pane-more]')`))
    await snap('01-workspace-rail-header-footer')
    await evaluate(`${W}.openLog(${JSON.stringify(a)})`)
    await waitFor(`!!document.querySelector('[data-log-pane], [data-pr-state]')`, 20000, 'log').catch(() => undefined)
    await sleep(2500)
    check('git log lists commits', await evaluate(`document.body.textContent.includes('add login form')`))
    await snap('02-git-log')
    await evaluate(`document.querySelector('[data-pane-more]')?.click()`)
    await sleep(500)
    await snap('03-pane-more-menu')
    await app.key('Escape', 'Escape', 27)
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  } finally {
    app.stop()
    fs.rmSync(BASE, { recursive: true, force: true })
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
