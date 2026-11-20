const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4531'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9531'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v104h')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const SP = `${S}.get('toolSpans')`
const SET = `${S}.get('settings')`
const TE = `${S}.get('turnEdits')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const QA = process.env.CCM_E2E_QA_DIR || path.resolve(__dirname, '../../../docs/qa/v104/WP-104h')
const CTRL = 2, SHIFT = 8

const REPO = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-turn-')))
fs.mkdirSync(path.join(REPO, 'src'))
const FA = path.join(REPO, 'src', 'alpha.ts')
const FB = path.join(REPO, 'src', 'beta.ts')
const FX = path.join(REPO, 'gen.txt')
fs.writeFileSync(FA, 'export const a = 1\n')
fs.writeFileSync(FB, 'export const b = 1\n')
fs.writeFileSync(FX, 'one\n')
const git = (...args) => execFileSync('git', args, { cwd: REPO, stdio: 'pipe' })
git('init', '-q')
git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'add', '.')
git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'commit', '-qm', 'init')
fs.writeFileSync(FA, 'export const a = 2\n')
fs.writeFileSync(FB, 'export const b = 2\n')
fs.writeFileSync(FX, 'two\n')

let seq = 0
const spanJs = (terminalId, file) => `${SP}.upsert({ id: 'e2e-turn-${++seq}', terminalId: '${terminalId}', agentId: '${terminalId}:main', tool: 'Edit', start: new Date().toISOString(), path: ${JSON.stringify(file)}, access: 'edit' })`
const bashJs = (terminalId) => `${SP}.upsert({ id: 'e2e-turn-${++seq}', terminalId: '${terminalId}', agentId: '${terminalId}:main', tool: 'Bash', start: new Date().toISOString() })`
const setStatus = (id, title, status) => `${T}.upsert({ id: '${id}', title: '${title}', status: '${status}', cwd: ${JSON.stringify(REPO)} })`
const rows = `[...document.querySelectorAll('[role=listbox] [role=option]')].map((n) => n.querySelector('.font-medium')?.textContent.trim()).filter(Boolean).sort().join(',')`
const segs = `[...document.querySelectorAll('[data-session-filter] [data-session-seg]')].map((n) => n.dataset.sessionSeg).join(',')`
const filterOf = `(${W}.panes.find((p) => p.kind === 'changes')?.sessionFilter ?? '')`

async function run(app) {
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
  await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.notify.clash'); localStorage.removeItem('ccm.workspace.changesTree'); sessionStorage.removeItem('ccm.turnEdits')`)
  await cmd('Page.reload')
  await sleep(1500)
  await waitFor(`!!${S}.get('ui') && ${S}.get('connection').hasData`, 30000, 'reloaded with data')
  await evaluate(`${S}.get('ui').view = 'workspace'`)

  await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
  await waitFor(`${W}.panes.filter((p) => p.state === 'running').length >= 1`, 25000, 'first shell')
  await evaluate(`${W}.split(${W}.panes[0].key, 'row', { kind: 'term', preset: 'shell' }, { cwd: ${JSON.stringify(REPO)} })`)
  await waitFor(`${W}.panes.filter((p) => p.state === 'running').length >= 2`, 25000, 'second shell')
  await sleep(600)
  await evaluate(`(() => {
    ${setStatus('e2e-turn-a', 'Alpha', 'idle')}
    ${setStatus('e2e-turn-b', 'Beta', 'idle')}
    const terms = ${W}.panes.filter((p) => !p.kind || p.kind === 'term')
    ${W}.update(terms[0].key, { preset: 'claude', terminalId: 'e2e-turn-a' })
    ${W}.update(terms[1].key, { preset: 'claude', terminalId: 'e2e-turn-b' })
  })()`)
  await sleep(300)
  await evaluate(`(() => { ${setStatus('e2e-turn-a', 'Alpha', 'working')}; ${setStatus('e2e-turn-b', 'Beta', 'working')} })()`)
  await sleep(300)
  await evaluate(`(() => { ${spanJs('e2e-turn-a', FA)}; ${spanJs('e2e-turn-b', FB)}; ${bashJs('e2e-turn-a')} })()`)
  await sleep(300)
  await evaluate(`(() => { ${setStatus('e2e-turn-a', 'Alpha', 'idle')}; ${setStatus('e2e-turn-b', 'Beta', 'idle')} })()`)
  await sleep(500)
  check('finished turns kept per session', (await evaluate(`${TE}.last.has('e2e-turn-a') && ${TE}.last.has('e2e-turn-b') && ${TE}.countOf('e2e-turn-a') === 1`)) === true)
  check('turn set saved to sessionStorage', /alpha\.ts/i.test((await evaluate(`sessionStorage.getItem('ccm.turnEdits') ?? ''`)) || ''))

  await evaluate(`${W}.openChanges(${JSON.stringify(REPO)}, { session: 'e2e-turn-a' })`)
  await waitFor(`!!document.querySelector('[data-session-filter]')`, 15000, 'filter bar').catch(() => {})
  await waitFor(`${rows} === 'alpha.ts'`, 8000, 'alpha only').catch(() => {})
  check('filter A shows only the file A edited', (await evaluate(rows)) === 'alpha.ts', await evaluate(rows))
  check('segments: all, both sessions, unknown', (await evaluate(segs)) === '~all,e2e-turn-a,e2e-turn-b,~unknown', await evaluate(segs))
  check('session segment shows icon, name and count', /^Alpha\D*1$/.test((await evaluate(`document.querySelector('[data-session-seg="e2e-turn-a"]')?.textContent ?? ''`)).trim()) && (await evaluate(`!!document.querySelector('[data-session-seg="e2e-turn-a"] svg')`)))
  check('unknown has a tooltip', ((await evaluate(`document.querySelector('[data-session-seg="~unknown"]')?.title ?? ''`)) || '').length > 10)
  for (const theme of ['galaxy', 'orrery-light']) {
    await evaluate(`${SET}.theme = '${theme}'`)
    await sleep(500)
    await shot(`v104h-01-filter-${theme === 'galaxy' ? 'dark' : 'light'}`)
  }

  const narrow = await evaluate(`(async () => {
    const sec = document.querySelector('[data-session-filter]').closest('section')
    sec.style.maxWidth = '380px'
    await new Promise((r) => setTimeout(r, 300))
    const sel = document.querySelector('[data-session-select]')
    const seg = document.querySelector('[data-session-filter] [role=radiogroup]')
    const out = { select: !!sel && sel.getBoundingClientRect().width > 0, seg: !!seg && seg.getBoundingClientRect().width > 0, over: sec.scrollWidth > sec.clientWidth + 1 }
    return out
  })()`)
  check('narrow pane: dropdown instead of segments, no sideways scroll', narrow.select && !narrow.seg && !narrow.over, JSON.stringify(narrow))
  await shot('v104h-03-narrow-light')
  await evaluate(`document.querySelector('[data-session-filter]').closest('section').style.maxWidth = ''`)
  await sleep(200)

  await evaluate(`document.querySelector('[data-session-seg="~unknown"]').click()`)
  await waitFor(`${rows} === 'gen.txt'`, 5000, 'unknown').catch(() => {})
  check('Unknown holds the file changed without Edit / Write', (await evaluate(rows)) === 'gen.txt', await evaluate(rows))
  check('filter stored on the pane', (await evaluate(filterOf)) === '~unknown')
  await evaluate(`document.querySelector('[data-session-seg="~all"]').click()`)
  await sleep(300)
  check('All lists every changed file', (await evaluate(rows)) === 'alpha.ts,beta.ts,gen.txt', await evaluate(rows))

  const rowB = `[...document.querySelectorAll('[data-side-row]')].find((n) => /Beta/.test(n.textContent))`
  await evaluate(`(() => { const el = ${rowB}; const r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 20, clientY: r.top + 10, button: 2 })) })()`)
  await waitFor(`!!document.querySelector('[data-menu-id="changesOfTurn"]')`, 5000, 'rail menu').catch(() => {})
  check('rail menu has View this turn changes', (await evaluate(`!!document.querySelector('[data-menu-id="changesOfTurn"]') && !document.querySelector('[data-menu-id="changesOfTurn"]').disabled && document.querySelector('[data-menu-id="changesOfTurn"]').getAttribute('aria-disabled') !== 'true'`)) === true)
  await evaluate(`document.querySelector('[data-menu-id="changesOfTurn"]').click()`)
  await waitFor(`${filterOf} === 'e2e-turn-b'`, 5000, 'filter B').catch(() => {})
  await sleep(300)
  check('rail menu opens Changes filtered to B', (await evaluate(filterOf)) === 'e2e-turn-b' && (await evaluate(rows)) === 'beta.ts', `${await evaluate(filterOf)} ${await evaluate(rows)}`)
  check('only one Changes pane for the repo', (await evaluate(`${W}.panes.filter((p) => p.kind === 'changes').length`)) === 1)

  await evaluate(`document.querySelector('[data-turn-mark-seen]').click()`)
  await sleep(500)
  check('Mark seen clears the last turn of B', (await evaluate(`${TE}.last.has('e2e-turn-b')`)) === false)
  check('filter falls back to All once B has no turn', (await evaluate(`document.querySelector('[data-session-filter]')?.dataset.filter`)) === '~all' && !(await evaluate(segs)).includes('e2e-turn-b'), await evaluate(segs))
  check('Unknown segment still there', (await evaluate(segs)).includes('~unknown'))
  check('beta.ts now counts as Unknown', /2/.test(await evaluate(`document.querySelector('[data-session-seg="~unknown"]')?.textContent ?? ''`)))

  await evaluate(`(() => { ${SET}.notifyEnabled = true; ${SET}.notifyDone = true })()`)
  await evaluate(`${S}.get('ui').view = 'monitor'`)
  await evaluate(setStatus('e2e-turn-b', 'Beta', 'working'))
  await sleep(300)
  await evaluate(spanJs('e2e-turn-b', FB))
  await sleep(200)
  await evaluate(setStatus('e2e-turn-b', 'Beta', 'idle'))
  await waitFor(`!!document.querySelector('[data-turn-toast="e2e-turn-b"]')`, 5000, 'done toast').catch(() => {})
  check('done toast offers View changes', (await evaluate(`!!document.querySelector('[data-turn-toast="e2e-turn-b"] [data-turn-toast-view]')`)) === true)
  for (const theme of ['galaxy', 'orrery-light']) {
    await evaluate(`${SET}.theme = '${theme}'`)
    await sleep(400)
    await shot(`v104h-02-toast-${theme === 'galaxy' ? 'dark' : 'light'}`)
  }
  await evaluate(`${W}.update(${W}.panes.find((p) => p.kind === 'changes').key, { sessionFilter: undefined })`)
  await evaluate(`document.querySelector('[data-turn-toast="e2e-turn-b"] [data-turn-toast-view]').click()`)
  await sleep(500)
  check('toast opens Workspace with Changes filtered to B', (await evaluate(`${S}.get('ui').view`)) === 'workspace' && (await evaluate(filterOf)) === 'e2e-turn-b' && (await evaluate(rows)) === 'beta.ts', `${await evaluate(filterOf)} ${await evaluate(rows)}`)

  await evaluate(`${W}.update(${W}.panes.find((p) => p.kind === 'changes').key, { sessionFilter: undefined })`)
  const paneA = await evaluate(`${W}.panes.find((p) => p.terminalId === 'e2e-turn-a').key`)
  await evaluate(`${W}.activate(${W}.panes.find((p) => p.kind === 'changes').key)`)
  await evaluate(`${W}.update(${JSON.stringify(paneA)}, { attention: 'done' })`)
  await sleep(300)
  await key('A', 'KeyA', 65, CTRL | SHIFT)
  await waitFor(`!!document.querySelector('[data-inbox] [data-inbox-row][data-reason="done"] [data-inbox-changes]')`, 5000, 'inbox done').catch(() => {})
  const hasInbox = await evaluate(`!!document.querySelector('[data-inbox] [data-inbox-row][data-reason="done"] [data-inbox-changes]')`)
  check('Inbox Done row has View changes', hasInbox === true)
  if (hasInbox) {
    await evaluate(`document.querySelector('[data-inbox] [data-inbox-row][data-reason="done"] [data-inbox-changes]').click()`)
    await sleep(500)
    check('Inbox View changes opens the filter of that session', (await evaluate(filterOf)) === 'e2e-turn-a' && (await evaluate(rows)) === 'alpha.ts', `${await evaluate(filterOf)} ${await evaluate(rows)}`)
  }

  check('no horizontal page scroll', (await evaluate(`document.documentElement.scrollWidth <= innerWidth`)) === true)
  await evaluate(`(() => { ${T}.remove('e2e-turn-a'); ${T}.remove('e2e-turn-b') })()`)
  await sleep(500)
  check('closed terminals drop their turn sets', (await evaluate(`['e2e-turn-a', 'e2e-turn-b'].some((id) => ${TE}.last.has(id) || ${TE}.current.has(id))`)) === false)
}

;(async () => {
  const app = await launch({ ui: 'next', env: process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {} })
  try {
    await run(app)
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  } finally {
    app.stop()
    try {
      fs.mkdirSync(QA, { recursive: true })
      for (const f of fs.readdirSync(OUT).filter((n) => n.startsWith('v104h-') && n.endsWith('.png'))) fs.copyFileSync(path.join(OUT, f), path.join(QA, f))
    } catch {}
    fs.rmSync(REPO, { recursive: true, force: true, maxRetries: 5 })
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
