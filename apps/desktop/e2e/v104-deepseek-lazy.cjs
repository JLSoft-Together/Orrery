const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = '4491'
process.env.CCM_E2E_DBG = '9491'
process.env.CCM_E2E_OUT = path.join(os.tmpdir(), 'ccm-e2e-v104g')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CHUNKS = `performance.getEntriesByType('resource').map((r) => r.name).filter((n) => /deepseek-/.test(n)).map((n) => n.split('/').pop())`
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

async function run(flag) {
  const app = await launch({ ui: 'next', env: flag ? { CCM_DEEPSEEK_FEATURE: '1' } : { CCM_DEEPSEEK_FEATURE: '' } })
  const { evaluate, waitFor, sleep, cmd } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`window.__errs = []; window.addEventListener('error', (e) => window.__errs.push(String(e.message))); window.addEventListener('unhandledrejection', (e) => window.__errs.push(String(e.reason))); const ce = console.error.bind(console); console.error = (...a) => { window.__errs.push(a.map(String).join(' ')); ce(...a) }; true`)
    const has = await evaluate(`typeof window.ccmDesktop.deepseekStatus === 'function'`)
    if (flag) {
      check('feature flag exposes the DeepSeek bridge', has === true)
      await waitFor(`${CHUNKS}.some((n) => /^deepseek-store/.test(n)) && ${CHUNKS}.some((n) => /^deepseek-i18n/.test(n))`, 15000, 'module chunks requested at start').then(() => check('store and i18n chunks load at start', true), () => check('store and i18n chunks load at start', false, JSON.stringify([])))
      check('Settings chunk waits for Settings', !(await evaluate(`${CHUNKS}.some((n) => /^deepseek-SettingsDeepSeek/.test(n))`)))
      await evaluate(`const ui = ${S}.get('ui'); ui.settingsPage = 'deepseek'; ui.settingsOpen = true; true`)
      await waitFor(`document.body.innerText.includes('DeepSeek CLI (Codewhale)')`, 15000, 'Settings DeepSeek rendered').then(() => check('Settings DeepSeek renders', true), () => check('Settings DeepSeek renders', false))
      check('Settings chunk requested on open', await evaluate(`${CHUNKS}.some((n) => /^deepseek-SettingsDeepSeek/.test(n))`))
      await sleep(600)
      const body = await evaluate(`document.body.innerText`)
      check('no raw i18n keys on the page', !/deepseekSetup\./.test(body))
      check('setup steps text is translated', /Set up/.test(body) && /Install the DeepSeek CLI/.test(body))
      await evaluate(`${S}.get('ui').settingsOpen = false; true`)
      await sleep(300)
      await evaluate(`${S}.get('ui').settingsPage = 'general'; true`)
      const note = await evaluate(`(async () => { const ws = ${S}.get('workspace'); ws.preset = 'deepseek'; return ws.preset })()`)
      check('deepseek preset selectable with the flag', note === 'deepseek')
      await sleep(500)
      const keys = await evaluate(`(async () => { const t = document.querySelector('#app').__vue_app__.config.globalProperties.$t; return ['deepseekSetup.launchNote.login', 'deepseekSetup.openSetup'].map((k) => t(k) === k) })()`)
      check('launcher keys resolve from the main bundle', keys.every((x) => x === false))
    } else {
      check('no flag, no DeepSeek bridge', has === false)
      await sleep(6000)
      const list = await evaluate(CHUNKS)
      check('no flag, no deepseek chunk requested', list.length === 0, JSON.stringify(list))
      await evaluate(`const ui = ${S}.get('ui'); ui.settingsPage = 'general'; ui.settingsOpen = true; true`)
      await sleep(800)
      const pages = await evaluate(`[...document.querySelectorAll('[data-settings-page], [role=tab]')].map((e) => e.textContent.trim()).join('|')`)
      check('no flag, Settings has no DeepSeek page', !/DeepSeek/.test(pages), pages.slice(0, 120))
      check('no flag, still no deepseek chunk', (await evaluate(CHUNKS)).length === 0)
    }
    const errs = await evaluate(`window.__errs`)
    check(flag ? 'no console errors with the flag' : 'no console errors without the flag', errs.length === 0, JSON.stringify(errs).slice(0, 300))
  } finally {
    app.stop()
  }
}

;(async () => {
  try {
    await run(true)
    await run(false)
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
