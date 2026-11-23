// v111 G3: pre-merge checklist in the worktree menu, check script run in a shell pane (OSC 633 exit code),
// stale result after a new commit, merge warning with confirm, and squash then branch removed with -D.
const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const UI = `${S}.get('ui')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v111ck-')))
const DIR = path.join(BASE, 'proj')
const WT = path.join(BASE, 'proj-wt', 'feat-ck')
const gitIn = (cwd) => (...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', '-c', 'core.autocrlf=false', ...args], { cwd, stdio: 'pipe', encoding: 'utf8' })

function fixture() {
  fs.mkdirSync(DIR, { recursive: true })
  const g = gitIn(DIR)
  g('init', '-q', '-b', 'main')
  fs.writeFileSync(path.join(DIR, 'a.txt'), 'a\n')
  fs.writeFileSync(path.join(DIR, '.gitignore'), 'fail.flag\n')
  fs.writeFileSync(path.join(DIR, 'package.json'), JSON.stringify({ name: 'ck', version: '1.0.0', scripts: { qa: 'node qa.js' } }, null, 2))
  fs.writeFileSync(path.join(DIR, 'qa.js'), "process.exit(require('fs').existsSync('fail.flag') ? 1 : 0)\n")
  g('add', '-A')
  g('commit', '-q', '-m', 'init')
  g('worktree', 'add', '-q', '-b', 'feat/ck', WT)
  fs.writeFileSync(path.join(WT, 'b.txt'), 'b\n')
  const w = gitIn(WT)
  w('add', 'b.txt')
  w('commit', '-q', '-m', 'work')
}

async function main() {
  fixture()
  const app = await launch({ exe: process.argv[2]?.startsWith('--') ? undefined : process.argv[2], env: { CCM_E2E_CONFIRM: 'ok', CCM_VERIFY_PROCESSES: '0', GIT_AUTHOR_NAME: 'e2e', GIT_AUTHOR_EMAIL: 'e2e@local', GIT_COMMITTER_NAME: 'e2e', GIT_COMMITTER_EMAIL: 'e2e@local', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep } = app
  const dataDir = path.join(OUT, 'run', 'data')
  const state = () => evaluate(`document.querySelector('[data-wt-checklist-state]')?.getAttribute('data-wt-checklist-state') ?? ''`)
  const stored = () => evaluate(`window.ccmDesktop.checklist(${JSON.stringify(DIR)}, { op: 'list', wt: ${JSON.stringify(WT)} }).then((r) => r.check.state)`)
  const waitStored = async (want, ms) => { const end = Date.now() + ms; while (Date.now() < end) { if ((await stored()) === want) return; await sleep(500) } throw new Error('timeout: stored ' + want) }
  const waitState = (want, ms) => waitFor(`document.querySelector('[data-wt-checklist-state]')?.getAttribute('data-wt-checklist-state') === '${want}'`, ms, `state ${want}`)
  const openMerge = async () => {
    const key = await evaluate(`${W}.panes.find((p) => p.cwd === ${JSON.stringify(DIR)})?.key ?? null`)
    if (!(await evaluate(`!!document.querySelector('#wt-branch')`))) {
      await evaluate(`${W}.activate('${key}')`)
      await evaluate(`document.querySelector(':is([data-pane="${key}"], [data-pane-of="${key}"]) [data-pane-worktrees]').click()`)
    }
    await waitFor(`!!document.querySelector('button[title^="Merge into"]')`, 20000, 'merge button')
    if (!(await evaluate(`!!document.querySelector('[data-wt-checklist]')`))) await evaluate(`document.querySelector('button[title^="Merge into"]').click()`)
    await waitFor(`!!document.querySelector('[data-wt-checklist]')`, 10000, 'checklist')
  }
  const type = (sel, value) => evaluate(`(() => { const i = document.querySelector('${sel}'); i.value = ${JSON.stringify(value)}; i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.workspace.preset','shell')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${UI}`, 15000, 'reloaded')
    await evaluate(`${UI}.view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 20000, 'shell running')

    await openMerge()
    await waitFor(`!!document.querySelector('[data-wt-checklist-script]')?.value`, 15000, 'default script').catch(() => {})
    const picked = await evaluate(`document.querySelector('[data-wt-checklist-script]')?.value ?? ''`)
    check('qa script is preselected', picked === 'npm run qa', picked)
    check('check state starts as not run', (await state()) === 'idle')

    await type('[data-wt-checklist-input]', 'update the docs')
    await evaluate(`document.querySelector('[data-wt-checklist-add]').click()`)
    await waitFor(`document.querySelectorAll('[data-wt-checklist-item]').length === 1`, 10000, 'item added')
    const open1 = await evaluate(`document.querySelector('[data-wt-checklist-open]')?.textContent ?? ''`)
    check('an added item shows up and the open count says 1', /1 item still open/.test(open1), open1)
    const file = fs.readdirSync(path.join(dataDir, 'checklists'))
    check('the list is stored in the data dir per worktree', file.length === 1 && /^[0-9a-f]{16}\.json$/.test(file[0]), file.join(','))

    await evaluate(`document.querySelector('[data-wt-checklist-run]').click()`)
    await waitStored('pass', 90000)
    await openMerge()
    await waitState('pass', 10000)
    check('running the check script in a shell pane ends as pass (exit 0)', true)
    const shellPane = await evaluate(`${W}.panes.some((p) => p.cwd === ${JSON.stringify(WT)} && p.kind !== 'browser')`)
    check('the script ran in a terminal pane of the worktree', shellPane === true)
    await shot('v111e-checklist')

    fs.writeFileSync(path.join(WT, 'c.txt'), 'c\n')
    gitIn(WT)('add', 'c.txt')
    gitIn(WT)('commit', '-q', '-m', 'more')
    await evaluate(`document.querySelector('button[title^="Merge into"]').click()`)
    await waitFor(`!document.querySelector('[data-wt-checklist]')`, 5000, 'panel closed')
    await openMerge()
    await waitState('stale', 15000)
    check('a new commit makes the earlier result out of date', true)

    fs.writeFileSync(path.join(WT, 'fail.flag'), 'x')
    await evaluate(`document.querySelector('[data-wt-checklist-run]').click()`)
    await waitStored('fail', 90000)
    await openMerge()
    await waitState('fail', 10000)
    check('a non-zero exit ends as fail', true)
    await shot('v111e-checklist-fail')

    await evaluate(`[...document.querySelectorAll('button')].find((b) => /Squash into one commit/.test(b.textContent)).click()`)
    await waitFor(`!!document.querySelector('[data-wt-merge-confirm]')`, 10000, 'merge confirm')
    const confirmText = await evaluate(`document.querySelector('[data-wt-merge-confirm]').textContent`)
    check('merge asks first: names the open item and the failed check', /update the docs/.test(confirmText) && /check failed/i.test(confirmText), confirmText)
    await shot('v111e-merge-confirm')
    await evaluate(`document.querySelector('[data-wt-merge-confirm-back]').click()`)
    await sleep(500)
    check('going back does not merge', gitIn(DIR)('log', '--oneline', 'main').trim().split('\n').length === 1)

    await evaluate(`[...document.querySelectorAll('button')].find((b) => /Squash into one commit/.test(b.textContent)).click()`)
    await waitFor(`!!document.querySelector('[data-wt-merge-confirm-go]')`, 10000, 'confirm again')
    await evaluate(`document.querySelector('[data-wt-merge-confirm-go]').click()`)
    await waitFor(`!!document.querySelector('[data-wt-cleanup-remove]')`, 40000, 'cleanup box')
    check('merge anyway squashes into main', gitIn(DIR)('log', '--oneline', 'main').trim().split('\n').length === 2)
    await evaluate(`document.querySelector('[data-wt-cleanup-remove]').click()`)
    await waitFor(`!!document.querySelector('[data-wt-cleanup-result]')`, 60000, 'cleanup result')
    const result = await evaluate(`document.querySelector('[data-wt-cleanup-result]').textContent`)
    check('after a squash the branch is removed too (content already in main)', /Removed the worktree and branch feat\/ck/.test(result), result)
    check('branch is gone', gitIn(DIR)('branch', '--list', 'feat/ck').trim() === '')
    await shot('v111e-squash-removed')
  } catch (e) {
    check('e2e run', false, String(e.message ?? e))
    await shot('v111e-failure').catch(() => {})
  } finally {
    app.stop()
  }
  const failed = results.filter((r) => r.startsWith('FAIL'))
  console.log(`\n${results.length - failed.length}/${results.length} passed`)
  process.exit(failed.length ? 1 : 0)
}

main().catch((e) => { console.error(e); process.exit(1) })
