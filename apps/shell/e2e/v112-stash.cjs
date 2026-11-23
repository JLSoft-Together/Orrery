const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4612'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9612'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112-stash')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const SHOTS = process.env.CCM_E2E_SHOTS || ''
const ENV = { GIT_AUTHOR_NAME: 'E2E', GIT_AUTHOR_EMAIL: 'e2e@example.com', GIT_COMMITTER_NAME: 'E2E', GIT_COMMITTER_EMAIL: 'e2e@example.com' }

function fixture() {
  const base = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-stash-')))
  const work = path.join(base, 'repo')
  const remote = path.join(base, 'remote.git')
  fs.mkdirSync(work, { recursive: true })
  const git = (args, cwd = work) => execFileSync('git', args, { cwd, stdio: 'pipe', env: { ...process.env, ...ENV } }).toString()
  git(['init', '-q', '-b', 'main'])
  git(['config', 'core.autocrlf', 'false'])
  fs.writeFileSync(path.join(work, 'README.md'), '# repo\n')
  git(['add', '.'])
  git(['commit', '-q', '-m', 'init'])
  fs.writeFileSync(path.join(work, 'base.txt'), 'base\n')
  git(['add', '.'])
  git(['commit', '-q', '-m', 'second'])
  fs.mkdirSync(remote)
  git(['init', '-q', '--bare', '-b', 'main'], remote)
  git(['remote', 'add', 'origin', remote])
  git(['push', '-q', '-u', 'origin', 'main'])
  fs.writeFileSync(path.join(work, 'local.txt'), 'local\n')
  git(['add', '.'])
  git(['commit', '-q', '-m', 'local work', '-m', 'body line'])
  return { base, work, git }
}

function copyShot(name) {
  if (!SHOTS) return
  fs.mkdirSync(SHOTS, { recursive: true })
  fs.copyFileSync(path.join(process.env.CCM_E2E_OUT, `${name}.png`), path.join(SHOTS, `${name}.png`))
}

async function run(ui) {
  const { base, work, git } = fixture()
  const answerFile = path.join(process.env.CCM_E2E_OUT, 'run', 'data', 'e2e-confirm-answer')
  const app = await launch({ ui, env: { CCM_E2E_CONFIRM: 'ok', CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...ENV, ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const q = JSON.stringify(work)
  const row = (subject) => `[...document.querySelectorAll('[data-commit]')].find((r) => r.querySelector('[data-log-subject]')?.textContent === ${JSON.stringify(subject)})`
  const click = (sel) => evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return false; e.click(); return true })()`)
  const count = () => evaluate(`document.querySelector('[data-stash-count]')?.textContent ?? '0'`)
  const openStashList = async () => {
    if (!(await evaluate(`!!document.querySelector('[data-stash-panel]')`))) await click('[data-stash-menu-button]')
    await waitFor(`!!document.querySelector('[data-stash-panel]')`, 5000, 'stash panel')
  }
  const head = () => git(['log', '-1', '--format=%s']).trim()
  const ctxMenu = (subject) => evaluate(`(() => { const r = ${row(subject)}; if (!r) return false; const b = r.getBoundingClientRect(); r.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: b.left + 60, clientY: b.top + 8 })); return true })()`)
  const menuItems = () => evaluate(`[...document.querySelectorAll('[data-log-pane] [role=menu] [role=menuitem]')].map((b) => b.textContent.trim())`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'; true`)
    await evaluate(`${W}.open('shell', ${q}, undefined, { tab: true }); true`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 25000, 'shell pane')
    await sleep(800)
    await evaluate(`${W}.openChanges(${q}); true`)
    await evaluate(`${W}.openLog(${q}); true`)
    ok('git log lists the commits', await wait(`!!${row('local work')}`, 20000, 'log rows'))
    ok('stash button is in the Log header and starts empty', await evaluate(`!!document.querySelector('[data-stash-menu-button]') && !document.querySelector('[data-stash-count]')`))

    await evaluate(`${W}.openChanges(${q}); true`)
    fs.writeFileSync(path.join(work, 'README.md'), '# repo changed\n')
    fs.writeFileSync(path.join(work, 'u.txt'), 'untracked\n')
    ok('Changes shows the edit and the untracked file', await wait(`document.querySelectorAll('[data-change]').length >= 2 || document.body.textContent.includes('u.txt')`, 20000, 'changes'))
    await sleep(500)

    const moreBtn = `[role=toolbar][data-pane-head] button[aria-label="More actions"]`
    await click(moreBtn)
    ok('Changes menu offers Stash', await wait(`!!document.querySelector('[data-menu-id="stash"]')`, 5000, 'stash item'))
    await click('[data-menu-id="stash"]')
    ok('dialog opens with a name from branch and time', await wait(`!!document.querySelector('[data-stash-dialog]')`, 5000, 'dialog'))
    const prefilled = await evaluate(`document.querySelector('[data-stash-name]').value`)
    ok('default name starts with the branch', /^main \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/.test(prefilled), prefilled)
    ok('untracked checkbox is offered and off', await evaluate(`document.querySelector('[data-stash-untracked]')?.checked === false`))
    await evaluate(`(() => { const i = document.querySelector('[data-stash-name]'); i.value = 'wip e2e'; i.dispatchEvent(new Event('input', { bubbles: true })); const c = document.querySelector('[data-stash-untracked]'); c.click() })()`)
    await sleep(300)
    if (SHOTS) {
      await shot(`stash-dialog-${ui}`)
      copyShot(`stash-dialog-${ui}`)
    }
    await click('[data-stash-submit]')
    ok('dialog closes after stash', await wait(`!document.querySelector('[data-stash-dialog]')`, 15000, 'dialog closed'))
    ok('tracked edit is back to HEAD', await wait(`true`, 100) && fs.readFileSync(path.join(work, 'README.md'), 'utf8') === '# repo\n')
    ok('untracked file went into the stash', !fs.existsSync(path.join(work, 'u.txt')))
    ok('git sees one stash named by the owner', git(['stash', 'list']).includes('wip e2e') && git(['stash', 'list']).trim().split('\n').length === 1)

    await evaluate(`${W}.openLog(${q}); true`)
    await sleep(500)
    await openStashList()
    ok('Log stash list shows the new stash', await wait(`document.querySelectorAll('[data-stash-row]').length === 1 && document.querySelector('[data-stash-row]').textContent.includes('wip e2e')`, 8000, 'row'))
    ok('count badge says 1', (await count()) === '1')
    await sleep(300)
    if (SHOTS) {
      await shot(`stash-list-${ui}`)
      copyShot(`stash-list-${ui}`)
    }

    await click('[data-stash-apply]')
    ok('Apply brings the changes back and keeps the stash', await wait(`true`, 100) && (await wait(`document.querySelector('[data-stash-note]')?.textContent.includes('Applied')`, 10000, 'applied')) && fs.readFileSync(path.join(work, 'README.md'), 'utf8') === '# repo changed\n' && fs.existsSync(path.join(work, 'u.txt')) && (await count()) === '1')
    git(['checkout', '--', 'README.md'])
    fs.rmSync(path.join(work, 'u.txt'))
    await click('[data-stash-pop]')
    ok('Pop restores the files and empties the list', (await wait(`document.querySelector('[data-stash-note]')?.textContent.includes('Popped')`, 10000, 'popped')) && fs.readFileSync(path.join(work, 'README.md'), 'utf8') === '# repo changed\n' && fs.existsSync(path.join(work, 'u.txt')) && git(['stash', 'list']).trim() === '')
    ok('empty list says so', await wait(`!!document.querySelector('[data-stash-empty]')`, 8000, 'empty'))

    git(['stash', 'push', '--include-untracked', '-m', 'second stash'])
    await click('[data-stash-menu-button]')
    await sleep(200)
    await openStashList()
    await wait(`document.querySelectorAll('[data-stash-row]').length === 1`, 8000, 'row 2')
    fs.writeFileSync(answerFile, 'cancel')
    await click('[data-stash-drop]')
    await sleep(1500)
    ok('Drop cancelled in the native confirm keeps the stash', git(['stash', 'list']).includes('second stash') && (await evaluate(`document.querySelectorAll('[data-stash-row]').length`)) === 1)
    fs.writeFileSync(answerFile, 'ok')
    await click('[data-stash-drop]')
    ok('Drop confirmed removes it', (await wait(`document.querySelector('[data-stash-note]')?.textContent.includes('Dropped')`, 10000, 'dropped')) && git(['stash', 'list']).trim() === '')
    const asked = fs.existsSync(path.join(process.env.CCM_E2E_OUT, 'run', 'data', 'e2e-confirms.log')) ? fs.readFileSync(path.join(process.env.CCM_E2E_OUT, 'run', 'data', 'e2e-confirms.log'), 'utf8') : ''
    ok('the confirm named the stash', /Drop stash@\{0\}: second stash/.test(asked) || /second stash/.test(asked), asked.slice(0, 120))
    await evaluate(`document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); true`)
    git(['checkout', '--', '.'])
    fs.rmSync(path.join(work, 'u.txt'), { force: true })

    await sleep(500)
    ok('menu on an older commit has no undo', (await ctxMenu('second')) && !(await menuItems()).some((m) => /Undo last commit/.test(m)), JSON.stringify(await menuItems()))
    await evaluate(`document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); true`)
    ok('menu on the first commit has no undo', (await ctxMenu('init')) && !(await menuItems()).some((m) => /Undo last commit/.test(m)))
    await evaluate(`document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); true`)
    await ctxMenu('local work')
    ok('menu on the unpushed head offers undo', await wait(`[...document.querySelectorAll('[data-log-pane] [role=menuitem]')].some((b) => /Undo last commit/.test(b.textContent))`, 5000, 'undo item'), JSON.stringify(await menuItems()))
    await evaluate(`[...document.querySelectorAll('[data-log-pane] [role=menuitem]')].find((b) => /Undo last commit/.test(b.textContent)).click(); true`)
    ok('undo notice appears with Commit again', await wait(`!!document.querySelector('[data-log-undo-notice] [data-log-redo]')`, 15000, 'notice'))
    ok('HEAD moved back one commit', head() === 'second')
    ok('files stay staged', git(['status', '--porcelain']).includes('A  local.txt') && fs.existsSync(path.join(work, 'local.txt')))
    await sleep(400)
    if (SHOTS) {
      await shot(`undo-toast-${ui}`)
      copyShot(`undo-toast-${ui}`)
    }
    await click('[data-log-redo]')
    ok('Commit again restores the commit with the same message', (await wait(`document.querySelector('[data-log-undo-notice]')?.textContent.includes('Committed again')`, 15000, 'redone')) && head() === 'local work' && git(['log', '-1', '--format=%B']).trim() === 'local work\n\nbody line')

    git(['push', '-q', 'origin', 'main'])
    await ctxMenu('local work')
    await wait(`[...document.querySelectorAll('[data-log-pane] [role=menuitem]')].length > 0`, 5000, 'menu again')
    const staleOffer = await evaluate(`[...document.querySelectorAll('[data-log-pane] [role=menuitem]')].some((b) => /Undo last commit/.test(b.textContent))`)
    if (staleOffer) {
      await evaluate(`[...document.querySelectorAll('[data-log-pane] [role=menuitem]')].find((b) => /Undo last commit/.test(b.textContent)).click(); true`)
      ok('after a push the backend still refuses and says so', (await wait(`document.querySelector('[data-log-undo-notice]')?.textContent.includes('already on the remote')`, 10000, 'pushed')) && head() === 'local work')
    } else {
      ok('after a push the undo item is gone', true)
    }
    const direct = await evaluate(`window.ccmDesktop.gitUndoCommit(${q}, ${JSON.stringify(git(['rev-parse', 'HEAD']).trim())})`)
    ok('IPC refuses a pushed commit', direct && direct.ok === false && direct.error === 'pushed', JSON.stringify(direct))
    if (failed) await shot(`stash-${ui}-fail`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`stash-${ui}-error`).catch(() => undefined)
  } finally {
    app.stop()
    fs.rmSync(base, { recursive: true, force: true })
  }
}

;(async () => {
  try {
    await run('classic')
    await run('next')
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
