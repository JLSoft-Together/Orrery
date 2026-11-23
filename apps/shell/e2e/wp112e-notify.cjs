const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const NAV = `document.querySelector('nav[aria-label=Projects]')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const DIR = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-112e-')))
fs.writeFileSync(path.join(DIR, 'package.json'), JSON.stringify({ name: 'notify-fixture', private: true, scripts: { dev: 'vite' } }))

;(async () => {
  const app = await launch({ ui: 'classic', env: process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {} })
  const { evaluate, waitFor, shot, sleep } = app
  const level = () => evaluate(`JSON.parse(localStorage.getItem('ccm.notify.projects') || '{}')`)
  const rail = () => evaluate(`${NAV}.querySelector('[data-rail-notify]')?.dataset.railNotify ?? null`)
  const pick = async (l) => {
    await evaluate(`${NAV}.querySelector('section[data-group] [data-side-menu]').click()`)
    await waitFor(`!!document.querySelector('[data-project-menu] [data-menu="notify-${l}"]')`, 5000, 'project menu')
    await sleep(250)
    await evaluate(`document.querySelector('[data-project-menu] [data-menu="notify-${l}"]').click()`)
    await sleep(300)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.theme','dark')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length === 1 && ${W}.panes.every((p) => p.state === 'running')`, 25000, 'shell')
    await sleep(800)

    check('no notify icon by default', (await rail()) === null)
    await evaluate(`${NAV}.querySelector('section[data-group] [data-side-menu]').click()`)
    await waitFor(`!!document.querySelector('[data-project-menu] [data-menu="notify-off"]')`, 5000, 'menu')
    await sleep(300)
    const labels = await evaluate(`[...document.querySelectorAll('[data-project-menu] [role=menuitemradio]')].map((b) => b.textContent.trim())`)
    check('menu lists the 3 levels', JSON.stringify(labels) === JSON.stringify(['All', 'Only when waiting for me', 'Off']), JSON.stringify(labels))
    check('All is checked by default', await evaluate(`document.querySelector('[data-menu="notify-all"]').getAttribute('aria-checked')`) === 'true')
    await shot('wp112e-01-project-menu')
    await evaluate(`document.querySelector('[data-project-menu] [data-menu="notify-off"]').click()`)
    await sleep(300)
    const saved = await level()
    check('Off saved by project root', Object.values(saved).includes('off') && Object.keys(saved).some((k) => k === DIR.replace(/\\/g, '/').toLowerCase()), JSON.stringify(saved))
    check('rail shows the muted bell for Off', (await rail()) === 'off')
    check('Off blocks every notification kind of that folder', await evaluate(`['waiting','done','context'].every((k) => !${S}.get('projectNotify').allows(${JSON.stringify(DIR)}, k))`))

    await shot('wp112e-02-rail-off')
    await pick('waiting')
    check('Only waiting: rail icon = waiting', (await rail()) === 'waiting')
    check('Only waiting lets waiting through and blocks done', await evaluate(`${S}.get('projectNotify').allows(${JSON.stringify(DIR)}, 'waiting') && !${S}.get('projectNotify').allows(${JSON.stringify(DIR)}, 'done')`))
    await evaluate(`${NAV}.querySelector('section[data-group] [data-side-menu]').click()`)
    await waitFor(`!!document.querySelector('[data-project-menu] [data-menu="notify-waiting"]')`, 5000, 'menu again')
    await sleep(300)
    check('menu marks the current level', await evaluate(`document.querySelector('[data-menu="notify-waiting"]').getAttribute('aria-checked')`) === 'true')
    await shot('wp112e-03-project-menu-waiting')
    await evaluate(`document.querySelector('[data-project-menu] [data-menu="notify-all"]').click()`)
    await sleep(300)
    check('All clears the entry and the icon', (await rail()) === null && Object.keys(await level()).length === 0)

    const reply = await evaluate(`window.__TAURI_INTERNALS__.invoke('ccm', { channel: 'ccm:tray-waiting', args: [{ total: 2, rows: [{ id: 't1', agent: 'Claude', project: 'alpha', question: 'Run it?' }, { id: 't2', agent: 'Codex', project: 'beta', question: '' }] }] }).then(() => 'ok', (e) => String(e))`)
    check('IPC ccm:tray-waiting accepted by the shell', reply === 'ok', reply)
    const clear = await evaluate(`window.__TAURI_INTERNALS__.invoke('ccm', { channel: 'ccm:tray-waiting', args: [{ total: 0, rows: [] }] }).then(() => 'ok', (e) => String(e))`)
    check('IPC clears the list', clear === 'ok', clear)
    results.push('SKIP native tray menu click: not drivable over CDP')
  } catch (e) {
    check('run', false, e.message)
    await shot('wp112e-99-error').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results-wp112e.txt'), results.join('\n'))
    try {
      fs.rmSync(DIR, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
    } catch {}
    process.exit(0)
  }
})()
