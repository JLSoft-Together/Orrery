const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4527'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9527'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-term-search')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CTRL = 2, ALT = 1
const WIN = process.platform === 'win32'
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.shortcuts')`
const echo = (a, b) => (WIN ? `echo ("${a}" + "${b}")` : `echo ${a}""${b}`)

async function run(ui) {
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  const ev = (type, k, code, vk, modifiers = 0) => cmd('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk, modifiers })
  const chord = async (k, code, vk, mods) => { await ev('rawKeyDown', k, code, vk, mods); await ev('keyUp', k, code, vk, mods) }
  const typeIn = async (key, text) => {
    await evaluate(`${W}.activate(${JSON.stringify(key)}); true`)
    await sleep(300)
    await evaluate(`document.querySelector('[data-pane="${key}"] .xterm-helper-textarea')?.focus(); true`)
    await cmd('Input.insertText', { text })
    await chord('Enter', 'Enter', 13, 0)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await cmd('Page.bringToFront')
    for (let i = 0; i < 2; i++) await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.tabs.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 30000, 'two shells')
    const keys = await evaluate(`${W}.panes.map((p) => p.key)`)
    await sleep(2500)
    await typeIn(keys[0], echo('ALPHA', 'QZX7'))
    await sleep(800)
    await typeIn(keys[1], echo('BRAVO', 'QZX8'))
    await sleep(1500)
    ok('second pane is focused before the search', (await evaluate(`${W}.activeKey`)) === keys[1])

    await chord('f', 'KeyF', 70, CTRL | ALT)
    await waitFor(`!!document.querySelector('[data-term-search-input]')`, 5000, 'panel').then(() => ok('Ctrl+Alt+F opens the search', true), () => ok('Ctrl+Alt+F opens the search', false))
    await cmd('Input.insertText', { text: 'qzx' })
    await waitFor(`document.querySelectorAll('[data-term-search-group]').length >= 2`, 8000, 'groups').catch(() => undefined)
    const groups = await evaluate(`[...document.querySelectorAll('[data-term-search-group]')].map((g) => g.dataset.termSearchGroup)`)
    ok('results are grouped under both panes', keys.every((k) => groups.includes(k)), JSON.stringify(groups))
    const text = await evaluate(`document.querySelector('#ts-list')?.textContent ?? ''`)
    ok('both outputs are found', /ALPHAQZX7/.test(text) && /BRAVOQZX8/.test(text), text.slice(0, 200))
    ok('status counts the matches', /matching line/.test(await evaluate(`document.querySelector('[data-term-search-status]')?.textContent ?? ''`)))

    const target = await evaluate(`(() => { const hit = [...document.querySelectorAll('[data-term-search-hit]')].find((h) => /ALPHAQZX7/.test(h.querySelector('mark')?.parentElement?.textContent ?? '')); if (!hit) return null; const g = hit.closest('[data-term-search-group]').dataset.termSearchGroup; hit.click(); return g })()`)
    ok('the ALPHA hit sits under the first pane', target === keys[0], String(target))
    await sleep(800)
    ok('clicking a hit closes the panel', !(await evaluate(`!!document.querySelector('[data-term-search]')`)))
    ok('clicking a hit focuses that pane', (await evaluate(`${W}.activeKey`)) === keys[0])
    ok('its tab is shown', (await evaluate(`${W}.activeTab?.key`)) === (await evaluate(`${W}.tabOf(${JSON.stringify(keys[0])})?.key`)))
    const find = await evaluate(`document.querySelector('[data-pane="${keys[0]}"] [role="search"] input')?.value ?? null`)
    ok('the pane find bar opens with the query', find === 'qzx', String(find))

    await chord('f', 'KeyF', 70, CTRL | ALT)
    await waitFor(`!!document.querySelector('[data-term-search-input]')`, 5000, 'panel again')
    await evaluate(`document.querySelector('[data-term-search] button[aria-label="Regular expression"]')?.click(); true`)
    await evaluate(`(() => { const i = document.querySelector('[data-term-search-input]'); i.value = 'QZX('; i.dispatchEvent(new Event('input')); i.focus(); return true })()`)
    await sleep(500)
    ok('a broken regex is reported', /Invalid regular expression/.test(await evaluate(`document.querySelector('[data-term-search-status]')?.textContent ?? ''`)))
    await chord('Escape', 'Escape', 27, 0)
    await chord('Escape', 'Escape', 27, 0)
    await sleep(300)
    ok('Esc closes the panel', !(await evaluate(`!!document.querySelector('[data-term-search]')`)))
    if (failed) await shot(`term-search-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`term-search-${ui}-error`).catch(() => undefined)
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
