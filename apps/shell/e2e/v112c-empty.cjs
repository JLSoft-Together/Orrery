const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4613'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9613'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112c')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-112c-')))
const REPO = path.join(BASE, 'clean')
fs.mkdirSync(REPO, { recursive: true })
fs.writeFileSync(path.join(REPO, 'a.txt'), 'x\n')
const git = (...a) => execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', ...a], { cwd: REPO, stdio: 'pipe' })
git('init', '-q')
git('add', '.')
git('commit', '-q', '-m', 'init')

async function run() {
  const app = await launch({ ui: 'next', env: { CCM_VERIFY_PROCESSES: '0', CLAUDE_CONFIG_DIR: path.join(BASE, 'claude'), CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')

    await evaluate(`${S}.get('ui').setView('monitor')`)
    check('board empty state renders', await wait(`!!document.querySelector('[data-board-empty-open]')`, 8000, 'board'))
    await sleep(500)
    await shot('112c-board')
    await evaluate(`document.querySelector('[data-board-empty-open]').click()`)
    check('board button opens workspace', await wait(`${S}.get('ui').view === 'workspace'`, 5000, 'ws'))

    await evaluate(`${S}.get('ui').setView('monitor')`)
    await evaluate(`${S}.get('inbox').toggle(document.querySelector('[data-monitor-board]').parentElement.firstElementChild)`)
    check('inbox empty state renders', await wait(`!!document.querySelector('[data-inbox-empty-action]')`, 8000, 'inbox'))
    await sleep(400)
    await shot('112c-inbox')
    await evaluate(`document.querySelector('[data-inbox-empty-action]').click()`)
    check('inbox button opens workspace and closes panel', await wait(`${S}.get('ui').view === 'workspace' && !document.querySelector('[data-inbox-empty-action]')`, 5000, 'inbox close'))

    await evaluate(`${S}.get('ui').setView('workspace')`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await wait(`${W}.panes.length >= 1 && ${W}.panes.every((p) => p.state === 'running')`, 25000, 'shell')
    await evaluate(`${W}.openChanges(${JSON.stringify(REPO)})`)
    await sleep(2500)
    console.log(String(await evaluate('document.body.innerText')).slice(0, 400).split('\n').join(' / '))
    check('changes empty state renders', await wait(`!!document.querySelector('[data-changes-empty-log]') || /No changes|Không có thay đổi/i.test(document.body.innerText)`, 15000, 'changes'))
    await sleep(600)
    await shot('112c-changes')
    check('changes empty has Git Log button', await evaluate(`!!document.querySelector('[data-changes-empty-log]')`), 'needs ChangesPane patch from main')
  } catch (e) {
    check('run', false, e.message)
    await shot('112c-error').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results-112c.txt'), results.join('\n'))
    try { fs.rmSync(BASE, { recursive: true, force: true }) } catch {}
    process.exit(0)
  }
}
run()
