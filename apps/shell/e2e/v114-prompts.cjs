const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4714'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9714'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v114-prompts')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}
const DIR = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-prompts-')))
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.prompts.coreMigrated','1')`

function freshCore() {
  const name = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
  const at = (p) => {
    try {
      return fs.statSync(p).mtimeMs
    } catch {
      return 0
    }
  }
  const dbg = path.join(REPO, 'apps/core/target/debug', name)
  const rel = path.join(REPO, 'apps/core/target/release', name)
  return at(dbg) > at(rel) ? { CCM_CORE_BIN: dbg } : {}
}

async function main() {
  const app = await launch({ ui: 'next', env: { CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...freshCore() } })
  const { evaluate, waitFor, sleep, cmd, key, type } = app
  const ok = (s, v, extra) => check(s, !!v, extra)
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const set = (req) => evaluate(`window.ccmDesktop.promptsSet(${JSON.stringify(req)}).then((r) => JSON.stringify(r))`).then((j) => JSON.parse(j))
  const get = () => evaluate(`window.ccmDesktop.promptsGet().then((r) => JSON.stringify(r))`).then((j) => JSON.parse(j))
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace'); true`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')

    const first = await get()
    ok('prompts-get answers ok with an items array', first.ok === true && Array.isArray(first.items))
    for (const p of first.items) await set({ op: 'delete', id: p.id })

    await evaluate(`window.__changed = 0; window.ccmDesktop.onPromptsChanged(() => { window.__changed++ }); true`)
    const added = await set({ op: 'upsert', item: { title: 'E2E multi', body: 'first line\nsecond line', scope: 'all' } })
    await set({ op: 'upsert', item: { title: 'E2E slash', body: '/review', scope: 'all' } })
    ok('upsert returns the list with both prompts', added.ok === true && (await get()).items.length === 2)
    ok('the changed event fires per write', await wait(`window.__changed >= 2`, 5000, 'changed'))
    const tooLong = await set({ op: 'upsert', item: { title: 'x'.repeat(81), body: 'b', scope: 'all' } })
    ok('a title over 80 characters is refused', tooLong.ok === false && tooLong.error === 'invalid-title')

    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)}); true`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 25000, 'shell running')
    await cmd('Page.bringToFront')
    await evaluate(`${S}.get('ui').paletteOpen = true; true`)
    await waitFor(`!!document.querySelector('[data-palette-ui]')`, 10000, 'palette')
    await evaluate(`document.querySelector('[data-palette-ui] input')?.focus(); true`)
    await type(';')
    ok('the ; prefix lists the saved prompts', await wait(`[...document.querySelectorAll('[data-palette-ui] [data-index]')].some((e) => /E2E multi/.test(e.textContent))`, 8000, 'rows'))
    await type('slash')
    await sleep(300)
    await key('Enter', 'Enter', 13)
    await sleep(800)
    ok('choosing one while a shell pane is focused types nothing', (await evaluate(`${W}.panes[0].prefill ?? ''`)) === '')
    ok('and says why', await evaluate(`${S}.get('toasts').list.some((t) => /agent pane/i.test(t.text))`))
    ok('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= innerWidth`))
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
    await app.shot?.('prompts-error').catch(() => undefined)
  } finally {
    console.log(results.join('\n'))
    app.stop()
    await new Promise((r) => setTimeout(r, 800))
    try {
      fs.rmSync(DIR, { recursive: true, force: true })
    } catch {}
  }
}

main().then(() => process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0))
