const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4591'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9591'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-share-mode')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const USER = os.userInfo().username
const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-share-')))
const DIR = path.join(BASE, 'secretproj')
fs.mkdirSync(DIR)
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.sidebar','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.shareMode')`

const LEAKS = (user) => `(() => {
  const u = ${JSON.stringify(user.toLowerCase())}
  const out = []
  for (const el of document.querySelectorAll('#app *')) {
    if (el.closest('.xterm, [data-term-host], [data-share-hide]')) continue
    const t = el.getAttribute('title') || ''
    const a = el.getAttribute('aria-label') || ''
    const own = el.childElementCount === 0 && el.tagName !== 'INPUT' && el.tagName !== 'TEXTAREA' ? el.textContent : ''
    for (const v of [t, a, own]) if (v.toLowerCase().includes(u)) out.push((el.tagName + ':' + v).slice(0, 160))
  }
  return out
})()`

async function press(cmd, code, key, vk, modifiers) {
  await cmd('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: vk, modifiers })
  await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: vk, modifiers })
}

async function run(ui) {
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(`${ui}: ${s}`, v, extra) }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length >= 1 && ${W}.panes.every((p) => p.state === 'running')`, 25000, 'shell running')
    await waitFor(`!!document.querySelector('[data-share-chip]')`, 15000, 'chip')
    await sleep(800)

    const before = await evaluate(LEAKS(USER))
    ok('before: the user name is visible in the UI (test is meaningful)', before.length > 0, JSON.stringify(before.slice(0, 3)))
    ok('before: no bar and root flag off', await evaluate(`!document.querySelector('[data-share-bar]') && !document.documentElement.hasAttribute('data-share')`))
    ok('chip is not pressed while off', (await evaluate(`document.querySelector('[data-share-chip]').getAttribute('aria-pressed')`)) === 'false')

    await evaluate(`document.querySelector('[data-share-chip]').click(); true`)
    await waitFor(`document.documentElement.getAttribute('data-share') === 'on'`, 5000, 'share on')
    await sleep(500)
    ok('chip click turns it on, bar shows', await evaluate(`!!document.querySelector('[data-share-bar]')`))
    ok('state is stored', (await evaluate(`localStorage.getItem('ccm.shareMode')`)) === '1')
    const during = await evaluate(LEAKS(USER))
    ok('on: rail, pane header, tooltips no longer show the user name', during.length === 0, JSON.stringify(during.slice(0, 4)))
    const masked = await evaluate(`[...document.querySelectorAll('[title]')].some((e) => /Users[\\\\/]•••/.test(e.getAttribute('title') || ''))`)
    ok('on: a path tooltip shows the masked form', masked)
    const veil = await evaluate(`(() => { const s = document.createElement('span'); s.setAttribute('data-share-hide', ''); s.textContent = 'secret'; document.body.appendChild(s); const c = getComputedStyle(s).color; s.remove(); return c })()`)
    ok('on: data-share-hide text is transparent', /rgba\(0, 0, 0, 0\)|transparent/.test(veil), veil)

    await evaluate(`(() => { const u = ${S}.get('ui'); u.settingsPage = 'integrations'; u.settingsOpen = true; return true })()`)
    await waitFor(`!!document.querySelector('[data-github-accounts]')`, 10000, 'github section').then(() => ok('Settings GitHub section renders while on', true), () => ok('Settings GitHub section renders while on', false))
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    await sleep(300)

    await evaluate(`document.querySelector('[data-share-off]').click(); true`)
    await waitFor(`!document.documentElement.hasAttribute('data-share')`, 5000, 'share off')
    await sleep(500)
    ok('bar off button turns it off', await evaluate(`!document.querySelector('[data-share-bar]')`))
    ok('off clears the stored state', (await evaluate(`localStorage.getItem('ccm.shareMode')`)) === null)
    const after = await evaluate(LEAKS(USER))
    ok('off: the user name shows again', after.length > 0, JSON.stringify(after.slice(0, 3)))
    const veilOff = await evaluate(`(() => { const s = document.createElement('span'); s.setAttribute('data-share-hide', ''); s.textContent = 'secret'; document.body.appendChild(s); const c = getComputedStyle(s).color; s.remove(); return c })()`)
    ok('off: data-share-hide text is normal again', !/rgba\(0, 0, 0, 0\)|transparent/.test(veilOff), veilOff)

    await evaluate(`document.body.focus(); true`)
    await press(cmd, 'KeyH', 'h', 72, 3)
    await waitFor(`document.documentElement.getAttribute('data-share') === 'on'`, 5000, 'chord on').then(() => ok('Ctrl+Alt+H turns it on', true), () => ok('Ctrl+Alt+H turns it on', false))
    await press(cmd, 'KeyH', 'h', 72, 3)
    await waitFor(`!document.documentElement.hasAttribute('data-share')`, 5000, 'chord off').then(() => ok('Ctrl+Alt+H turns it off', true), () => ok('Ctrl+Alt+H turns it off', false))

    await evaluate(`${S}.get('ui').paletteOpen = true; true`)
    await waitFor(`!!document.querySelector('[role="dialog"] input, [data-palette] input')`, 8000, 'palette')
    await evaluate(`(() => { const i = document.querySelector('[role="dialog"] input, [data-palette] input'); i.value = 'screen share'; i.dispatchEvent(new Event('input', { bubbles: true })); return true })()`)
    await sleep(500)
    const hit = await evaluate(`[...document.querySelectorAll('[role="option"], [data-palette-item]')].some((e) => /screen share mode/i.test(e.textContent))`)
    ok('Ctrl+K lists Screen share mode', hit)
    await evaluate(`${S}.get('ui').paletteOpen = false; true`)

    if (failed) await shot(`share-mode-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`share-mode-${ui}-error`).catch(() => undefined)
  } finally {
    app.stop()
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
