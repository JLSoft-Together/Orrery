// v111 G2: worktree config (.orrery/worktree.json), setup in a new shell pane, cleanup box after merge, suggest button.
// Native confirms are auto-answered "ok" through CCM_E2E_CONFIRM; the log in the data dir proves they were asked.
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

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v111wt-')))
const DIR = path.join(BASE, 'proj')
const DIR2 = path.join(BASE, 'plain')
const WT = path.join(BASE, 'proj-wt', 'feat-e2e')
const CLEANUP_MARK = path.join(BASE, 'proj-wt', 'cleanup.marker')
const gitIn = (cwd) => (...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', '-c', 'core.autocrlf=false', ...args], { cwd, stdio: 'pipe', encoding: 'utf8' })

function fixture() {
  fs.mkdirSync(DIR, { recursive: true })
  const g = gitIn(DIR)
  g('init', '-q', '-b', 'main')
  fs.writeFileSync(path.join(DIR, 'a.txt'), 'a\n')
  fs.writeFileSync(path.join(DIR, '.gitignore'), '*.marker\n.env\nkeep.cfg\n.orrery/\n')
  g('add', '-A')
  g('commit', '-q', '-m', 'init')
  fs.writeFileSync(path.join(DIR, '.env'), 'SECRET=1\n')
  fs.writeFileSync(path.join(DIR, 'keep.cfg'), 'keep\n')
  fs.mkdirSync(path.join(DIR, '.orrery'), { recursive: true })
  fs.writeFileSync(path.join(DIR, '.orrery', 'worktree.json'), JSON.stringify({ copy: ['keep.cfg'], setup: ['echo one > setup1.marker', 'echo two > setup2.marker'], cleanup: ['echo bye > ../cleanup.marker'] }, null, 2))
  fs.mkdirSync(DIR2, { recursive: true })
  const h = gitIn(DIR2)
  h('init', '-q', '-b', 'main')
  fs.writeFileSync(path.join(DIR2, 'a.txt'), 'a\n')
  fs.writeFileSync(path.join(DIR2, 'pnpm-lock.yaml'), 'lockfileVersion: 9\n')
  fs.writeFileSync(path.join(DIR2, 'package.json'), '{}\n')
  h('add', '-A')
  h('commit', '-q', '-m', 'init')
}

async function main() {
  fixture()
  const app = await launch({ exe: process.argv[2]?.startsWith('--') ? undefined : process.argv[2], env: { CCM_E2E_CONFIRM: 'ok', CCM_VERIFY_PROCESSES: '0', GIT_AUTHOR_NAME: 'e2e', GIT_AUTHOR_EMAIL: 'e2e@local', GIT_COMMITTER_NAME: 'e2e', GIT_COMMITTER_EMAIL: 'e2e@local', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep } = app
  const dataDir = path.join(OUT, 'run', 'data')
  const confirms = () => { try { return fs.readFileSync(path.join(dataDir, 'e2e-confirms.log'), 'utf8') } catch { return '' } }
  const paneKey = (dir) => evaluate(`${W}.panes.find((p) => p.cwd === ${JSON.stringify(dir)})?.key ?? null`)
  const openMenu = async (dir) => {
    const key = await paneKey(dir)
    await evaluate(`${W}.activate('${key}')`)
    await evaluate(`document.querySelector(':is([data-pane="${key}"], [data-pane-of="${key}"]) [data-pane-worktrees]').click()`)
    await waitFor(`!!document.querySelector('#wt-branch')`, 15000, 'worktree menu')
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.workspace.preset','shell')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${UI}`, 15000, 'reloaded')
    await evaluate(`${UI}.view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')

    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 20000, 'shell running')
    await openMenu(DIR)
    await waitFor(`!!document.querySelector('[data-wt-config-setup]')`, 15000, 'config lines').catch(() => {})
    const lines = await evaluate(`document.querySelector('[data-wt-config-setup]')?.textContent ?? ''`)
    check('menu shows the setup commands from .orrery/worktree.json', /setup1\.marker/.test(lines), lines)
    const copyLine = await evaluate(`[...document.querySelectorAll('.ccm-menu p')].map((p) => p.textContent).find((t) => /Copies/.test(t)) ?? ''`)
    check('copy list comes from the file and is not editable', /keep\.cfg/.test(copyLine) && /worktree\.json/.test(copyLine), copyLine)
    await shot('v111d-menu')

    await evaluate(`(() => { const i = document.querySelector('#wt-branch'); i.value = 'feat/e2e'; i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
    await evaluate(`document.querySelector('#wt-branch').form.requestSubmit()`)
    await waitFor(`${W}.panes.some((p) => p.cwd === ${JSON.stringify(WT)})`, 30000, 'worktree pane')
    check('worktree is created with only the configured files copied', fs.existsSync(path.join(WT, 'keep.cfg')) && !fs.existsSync(path.join(WT, '.env')))
    check('native confirm asked before running the repo setup', /setup1\.marker/.test(confirms()), confirms().slice(0, 200))
    await waitFor(`${W}.panes.filter((p) => p.cwd === ${JSON.stringify(WT)}).length >= 2`, 20000, 'setup pane').catch(() => {})
    const deadline = Date.now() + 40000
    while (Date.now() < deadline && !(fs.existsSync(path.join(WT, 'setup1.marker')) && fs.existsSync(path.join(WT, 'setup2.marker')))) await sleep(500)
    check('setup runs in a shell pane of the worktree, commands chained', fs.existsSync(path.join(WT, 'setup1.marker')) && fs.existsSync(path.join(WT, 'setup2.marker')))
    const confirmCount = (confirms().match(/setup1\.marker/g) ?? []).length

    const g = gitIn(WT)
    fs.writeFileSync(path.join(WT, 'b.txt'), 'b\n')
    g('add', 'b.txt')
    g('commit', '-q', '-m', 'work')

    await openMenu(DIR)
    await waitFor(`!!document.querySelector('button[title^="Merge into"]')`, 20000, 'merge button')
    await evaluate(`document.querySelector('button[title^="Merge into"]').click()`)
    await waitFor(`[...document.querySelectorAll('button')].some((b) => /Fast-forward/.test(b.textContent) && !b.disabled)`, 10000, 'ff enabled')
    await evaluate(`[...document.querySelectorAll('button')].find((b) => /Fast-forward/.test(b.textContent)).click()`)
    await waitFor(`!!document.querySelector('[data-wt-cleanup-remove]')`, 30000, 'cleanup box')
    const title = await evaluate(`document.querySelector('[data-wt-cleanup] h2').textContent`)
    check('after merge the box asks to remove worktree and branch', /feat\/e2e/.test(title), title)
    const paneNote = await evaluate(`document.querySelector('[data-wt-cleanup-panes]')?.textContent ?? ''`)
    check('box warns about the panes open in the worktree', /\d+ panes? open/.test(paneNote), paneNote)
    await shot('v111d-cleanup')

    await evaluate(`document.querySelector('[data-wt-cleanup-remove]').click()`)
    await waitFor(`!!document.querySelector('[data-wt-cleanup-result]')`, 60000, 'cleanup result')
    const result = await evaluate(`document.querySelector('[data-wt-cleanup-result]').textContent`)
    check('cleanup box reports worktree and branch removed', /Removed the worktree and branch feat\/e2e/.test(result), result)
    check('cleanup command ran before the removal', fs.existsSync(CLEANUP_MARK))
    check('worktree folder is gone', !fs.existsSync(WT))
    check('branch is deleted', gitIn(DIR)('branch', '--list', 'feat/e2e').trim() === '')
    check('panes of the worktree are closed', (await evaluate(`${W}.panes.filter((p) => (p.cwd ?? '').includes('feat-e2e')).length`)) === 0)
    check('native confirm asked once for setup and once for cleanup', confirmCount === 1 && (confirms().match(/cleanup\.marker/g) ?? []).length === 1, confirms().slice(0, 300))
    await evaluate(`document.querySelector('[data-wt-cleanup-close]').click()`)
    await waitFor(`!document.querySelector('[data-wt-cleanup]')`, 5000, 'box closed')

    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR2)})`)
    await waitFor(`!!${W}.panes.find((p) => p.cwd === ${JSON.stringify(DIR2)} && p.state === 'running')`, 20000, 'second shell')
    await openMenu(DIR2)
    await waitFor(`!!document.querySelector('[data-wt-config-suggest]')`, 15000, 'suggest button')
    await shot('v111d-menu-suggest')
    await evaluate(`document.querySelector('[data-wt-config-suggest]').click()`)
    const cfgPath = path.join(DIR2, '.orrery', 'worktree.json')
    const t1 = Date.now()
    while (Date.now() - t1 < 15000 && !fs.existsSync(cfgPath)) await sleep(250)
    const written = fs.existsSync(cfgPath) ? JSON.parse(fs.readFileSync(cfgPath, 'utf8')) : null
    check('suggest writes .orrery/worktree.json guessed from pnpm-lock.yaml', JSON.stringify(written?.setup) === '["pnpm install"]', JSON.stringify(written))
    check('suggest opens the file in an editor pane', await waitFor(`${W}.panes.some((p) => /worktree\\.json$/.test(p.path ?? ''))`, 10000, 'file pane').then(() => true).catch(() => false))
    check('no commit was made by the suggestion', gitIn(DIR2)('status', '--porcelain').includes('.orrery'))
  } catch (e) {
    check('e2e run', false, String(e.message ?? e))
    await shot('v111d-failure').catch(() => {})
  } finally {
    app.stop()
  }
  const failed = results.filter((r) => r.startsWith('FAIL'))
  console.log(`\n${results.length - failed.length}/${results.length} passed`)
  process.exit(failed.length ? 1 : 0)
}

main().catch((e) => { console.error(e); process.exit(1) })
