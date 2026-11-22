const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4512'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9512'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-shortcuts')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const P = `${S}.get('prompts')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CTRL = 2, ALT = 1, SHIFT = 8
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.shortcuts')`

async function run(ui) {
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  const ev = (type, k, code, vk, modifiers = 0) => cmd('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk, modifiers })
  const chord = async (k, code, vk, mods) => { await ev('rawKeyDown', k, code, vk, mods); await ev('keyUp', k, code, vk, mods) }
  const ctrlTab = async (shift = false) => { await ev('rawKeyDown', 'Tab', 'Tab', 9, CTRL | (shift ? SHIFT : 0)); await ev('keyUp', 'Tab', 'Tab', 9, CTRL | (shift ? SHIFT : 0)) }
  const ctrlDown = () => ev('rawKeyDown', 'Control', 'ControlLeft', 17, CTRL)
  const ctrlUp = () => ev('keyUp', 'Control', 'ControlLeft', 17, 0)
  const activeTab = () => evaluate(`${W}.activeTab?.key`)
  const stored = () => evaluate(`localStorage.getItem('ccm.shortcuts')`)
  const keyBtn = (id) => `document.querySelector('[data-shortcut-key="${id}"]')`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    for (let i = 0; i < 3; i++) await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.tabs.length === 3 && ${W}.panes.every((p) => p.state === 'running')`, 30000, 'three tabs')
    const tabs = await evaluate(`${W}.tabs.map((t) => t.key)`)
    await evaluate(`${W}.activateTab(${JSON.stringify(tabs[0])})`)
    await sleep(30)
    await evaluate(`${W}.activateTab(${JSON.stringify(tabs[2])})`)
    await sleep(30)
    await evaluate(`${W}.activateTab(${JSON.stringify(tabs[1])})`)
    await sleep(300)
    ok('starts on the second tab', (await activeTab()) === tabs[1])

    await ctrlDown()
    await ctrlTab()
    await sleep(150)
    const shown = await evaluate(`[...document.querySelectorAll('[data-mru-item]')].map((e) => e.dataset.mruItem)`)
    ok('holding Ctrl lists tabs by recent use', JSON.stringify(shown) === JSON.stringify([tabs[1], tabs[2], tabs[0]]), JSON.stringify(shown))
    ok('the previous tab is highlighted', (await evaluate(`document.querySelector('[data-mru-item][data-on]')?.dataset.mruItem`)) === tabs[2])
    ok('the tab does not change until Ctrl is released', (await activeTab()) === tabs[1])
    await ctrlUp()
    await sleep(200)
    ok('releasing Ctrl goes to the previous tab', (await activeTab()) === tabs[2], await activeTab())
    ok('the list closes on release', !(await evaluate(`!!document.querySelector('[data-mru-switcher]')`)))

    await ctrlDown(); await ctrlTab(); await ctrlUp()
    await sleep(200)
    ok('Ctrl+Tab again returns to the tab before', (await activeTab()) === tabs[1], await activeTab())

    await ctrlDown(); await ctrlTab(); await ctrlTab()
    await sleep(100)
    ok('a second Tab while held moves down the list', (await evaluate(`document.querySelector('[data-mru-item][data-on]')?.dataset.mruItem`)) === tabs[0])
    await chord('Escape', 'Escape', 27, CTRL)
    await ctrlUp()
    await sleep(200)
    ok('Esc cancels and keeps the tab', (await activeTab()) === tabs[1] && !(await evaluate(`!!document.querySelector('[data-mru-switcher]')`)))

    await evaluate(`(() => { const u = ${S}.get('ui'); u.settingsPage = 'workspace'; u.settingsOpen = true })(); true`)
    await waitFor(`!!${keyBtn('wsPrompts')}`, 10000, 'shortcut rows').then(() => ok('Settings lists the shortcuts', true), () => ok('Settings lists the shortcuts', false))
    ok('every row shows a default key', await evaluate(`[...document.querySelectorAll('[data-shortcut-key]')].every((b) => b.textContent.trim().length > 0)`))

    await evaluate(`${keyBtn('wsPrompts')}.click(); ${keyBtn('wsPrompts')}.focus(); true`)
    await sleep(100)
    ok('click starts recording', (await evaluate(`${keyBtn('wsPrompts')}.textContent.trim()`)) === 'Press the new keys')
    await chord('J', 'KeyJ', 74, CTRL | SHIFT)
    await sleep(200)
    ok('new keys are saved', /"wsPrompts":\["Ctrl","Shift","J"\]/.test((await stored()) ?? ''), await stored())
    ok('row shows the new keys and a reset button', (await evaluate(`${keyBtn('wsPrompts')}.textContent.trim()`)) === 'Ctrl+Shift+J' && (await evaluate(`!!document.querySelector('[data-shortcut-reset="wsPrompts"]')`)))
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    await sleep(300)

    await evaluate(`${P}.open = false; true`)
    await chord('J', 'KeyJ', 74, CTRL | SHIFT)
    await sleep(300)
    ok('the new keys work', await evaluate(`${P}.open === true`))
    await evaluate(`${P}.open = false; true`)
    await chord('P', 'KeyP', 80, ALT | SHIFT)
    await sleep(300)
    ok('the old keys stop working', await evaluate(`${P}.open === false`))

    await evaluate(`(() => { const u = ${S}.get('ui'); u.settingsPage = 'workspace'; u.settingsOpen = true })(); true`)
    await waitFor(`!!${keyBtn('wsNew')}`, 10000, 'rows again')
    await evaluate(`${keyBtn('wsNew')}.click(); ${keyBtn('wsNew')}.focus(); true`)
    await sleep(100)
    await chord('J', 'KeyJ', 74, CTRL | SHIFT)
    await sleep(200)
    const err = await evaluate(`document.querySelector('[data-shortcut-row="wsNew"] [data-shortcut-error]')?.textContent.trim()`)
    ok('a taken key names the command that uses it', /Reopen|prompt/i.test(err ?? '') || /Prompt library/.test(err ?? ''), err)
    ok('a taken key changes nothing', !/wsNew/.test((await stored()) ?? ''))
    await chord('W', 'KeyW', 87, CTRL | SHIFT)
    await sleep(200)
    const err2 = await evaluate(`document.querySelector('[data-shortcut-row="wsNew"] [data-shortcut-error]')?.textContent.trim()`)
    ok('the default key of another command is reported by name', /Close the focused pane/.test(err2 ?? ''), err2)
    await chord('C', 'KeyC', 67, CTRL)
    await sleep(200)
    const err3 = await evaluate(`document.querySelector('[data-shortcut-row="wsNew"] [data-shortcut-error]')?.textContent.trim()`)
    ok('a terminal key cannot be taken', /Terminals use this key/.test(err3 ?? ''), err3)
    ok('still recording after an error', (await evaluate(`${keyBtn('wsNew')}.textContent.trim()`)) === 'Press the new keys')
    await chord('Escape', 'Escape', 27, 0)
    await sleep(150)
    ok('Esc stops recording and keeps Settings open', (await evaluate(`${keyBtn('wsNew')}?.textContent.trim()`)) !== 'Press the new keys' && (await evaluate(`${S}.get('ui').settingsOpen`)) === true)

    await evaluate(`document.querySelector('[data-shortcut-reset="wsPrompts"]').click(); true`)
    await sleep(200)
    ok('reset on a row clears it', (await stored()) === null)
    await evaluate(`${keyBtn('wsPrompts')}.click(); ${keyBtn('wsPrompts')}.focus(); true`)
    await chord('J', 'KeyJ', 74, CTRL | SHIFT)
    await evaluate(`${keyBtn('wsFocusMode')}.click(); ${keyBtn('wsFocusMode')}.focus(); true`)
    await chord('K', 'KeyK', 75, CTRL | SHIFT)
    await sleep(200)
    ok('two rows changed', Object.keys(JSON.parse((await stored()) ?? '{}')).length === 2, await stored())
    await evaluate(`document.querySelector('[data-shortcut-reset-all]').click(); true`)
    await sleep(200)
    ok('reset all clears every row', (await stored()) === null && (await evaluate(`!document.querySelector('[data-shortcut-reset]')`)))
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    await sleep(300)
    await evaluate(`${P}.open = false; true`)
    await chord('P', 'KeyP', 80, ALT | SHIFT)
    await sleep(300)
    ok('defaults work again after reset', await evaluate(`${P}.open === true`))
    if (failed) await shot(`shortcuts-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`shortcuts-${ui}-error`).catch(() => undefined)
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
