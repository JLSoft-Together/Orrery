const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4612'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9612'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112-header')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-hdr-')))
const WORK = path.join(BASE, 'work')
fs.mkdirSync(WORK, { recursive: true })

async function run(ui) {
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const ok = (s, v, extra) => check(`${ui}: ${s}`, !!v, extra)
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const esc = async () => {
    await cmd('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
    await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
    await sleep(300)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'; true`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(WORK)}, undefined, { tab: true }); true`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 25000, 'shell pane')
    await sleep(500)
    const shellKey = await evaluate(`${W}.panes.find((p) => (!p.kind || p.kind === 'term') && p.state === 'running').key`)
    const HEAD_S = `document.querySelector('[data-pane-of="${shellKey}"]')`
    ok('shell pane keeps Split in the header', await wait(`!!${HEAD_S}?.querySelector('[data-pane-split]')`, 6000, 'shell split'))
    ok('shell pane has no agent buttons', await evaluate(`!${HEAD_S}.querySelector('[data-pane-attach], [data-pane-chat-toggle], [data-pane-turn], [data-pane-tools]')`))

    const key = await evaluate(`(() => {
      ${T}.upsert({ id: 'e2e-hdr-a', title: 'Agent', status: 'idle', cwd: ${JSON.stringify(WORK)} })
      ${W}.update('${shellKey}', { preset: 'claude', terminalId: 'e2e-hdr-a' })
      return '${shellKey}'
    })()`)
    const SEC = `document.querySelector('[data-pane="${key}"]')`
    const HEAD = `document.querySelector('[data-pane-of="${key}"]')`
    const has = (sel) => `!!${HEAD}?.querySelector('${sel}')`
    await wait(has('[data-pane-attach]'), 8000, 'attach')
    ok('wide header pins Attach, Chat, Changes of this turn', (await evaluate(has('[data-pane-attach]'))) && (await evaluate(has('[data-pane-chat-toggle]'))) && (await evaluate(has('[data-pane-turn]'))))
    ok('Split and Worktree left the header of an agent pane', await evaluate(`!${HEAD}.querySelector('[data-pane-split], [data-pane-worktrees]')`))
    ok('Close stays in the header', await evaluate(has('[data-pane-close]')))
    await evaluate(`${HEAD}.querySelector('[data-pane-more]').click(); true`)
    await wait(`!!document.querySelector('[role=menu]')`, 5000, 'more menu')
    const items = await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].map((b) => b.textContent.trim())`)
    ok('"..." menu holds Split and the worktree entry', items.some((x) => /Split/i.test(x)) && items.some((x) => /worktree|branch/i.test(x)), JSON.stringify(items))
    await shot(`header-wide-menu-${ui}`)
    await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].find((b) => /Split/i.test(b.textContent)).click(); true`)
    ok('Split from the menu opens the split menu', await wait(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].some((b) => /Codex/.test(b.textContent))`, 5000, 'split menu'))
    await esc()
    await evaluate(`${HEAD}.querySelector('[data-pane-more]').click(); true`)
    await wait(`!!document.querySelector('[role=menu]')`, 5000, 'more menu 2')
    await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].find((b) => /worktree|branch/i.test(b.textContent)).click(); true`)
    ok('Worktree from the menu opens the worktree dialog', await wait(`!!document.querySelector('[role=dialog]')`, 15000, 'worktree dialog'))
    await esc()
    await evaluate(`document.activeElement?.blur(); true`)
    await shot(`header-wide-${ui}`)

    await evaluate(`${SEC}.style.maxWidth = '300px'; true`)
    ok('300px header keeps Attach and Chat, moves Changes and MCP to the menu', await wait(`!${HEAD}.querySelector('[data-pane-turn]') && !!${HEAD}.querySelector('[data-pane-chat-toggle]') && !!${HEAD}.querySelector('[data-pane-attach]')`, 5000, 'narrow'))
    await evaluate(`${HEAD}.querySelector('[data-pane-more]').click(); true`)
    await wait(`!!document.querySelector('[role=menu]')`, 5000, 'more narrow')
    const narrow = await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].map((b) => b.textContent.trim())`)
    ok('narrow menu lists the changes entry', narrow.some((x) => /changes/i.test(x)), JSON.stringify(narrow))
    await shot(`header-narrow-menu-${ui}`)
    await esc()
    await evaluate(`${SEC}.style.maxWidth = '210px'; true`)
    await sleep(500)
    ok('210px header keeps only Chat', await evaluate(`!!${HEAD}.querySelector('[data-pane-chat-toggle]') && !${HEAD}.querySelector('[data-pane-attach]')`))
    ok('narrow header does not overflow sideways', await evaluate(`${HEAD}.scrollWidth <= ${HEAD}.clientWidth + 1`), String(await evaluate(`${HEAD}.scrollWidth + '/' + ${HEAD}.clientWidth`)))
    await shot(`header-narrow-${ui}`)
    await evaluate(`${SEC}.style.maxWidth = ''; true`)

    const TAB = `document.querySelector('[data-tab-item] [data-tab]')`
    await evaluate(`${W}.update('${key}', { dirty: true }); ${T}.upsert({ id: 'e2e-hdr-a', title: 'Agent', status: 'waiting', cwd: ${JSON.stringify(WORK)} }); true`)
    ok('waiting pane puts one waiting signal on the tab', await wait(`document.querySelectorAll('[data-tab-item] [data-tab-signal]').length === 1 && ${TAB}.parentElement.querySelector('[data-tab-signal]').dataset.tabSignal === 'waiting'`, 5000, 'waiting signal'))
    ok('tab label carries the unsaved note too', await evaluate(`/Unsaved changes/.test(${TAB}.getAttribute('aria-label')) && /Waiting/i.test(${TAB}.getAttribute('aria-label'))`), String(await evaluate(`${TAB}.getAttribute('aria-label')`)))
    ok('tab shows no separate unsaved dot', await evaluate(`!document.querySelector('[data-tab-dirty], [data-tab-dirty-next]')`))
    await shot(`tab-waiting-${ui}`)
    await evaluate(`${W}.update('${key}', { dirty: false }); ${T}.upsert({ id: 'e2e-hdr-a', title: 'Agent', status: 'error', cwd: ${JSON.stringify(WORK)} }); true`)
    const errSignal = await wait(`${TAB}.parentElement.querySelector('[data-tab-signal]')?.dataset.tabSignal === 'error'`, 5000, 'error signal')
    ok('error status shows the error signal', errSignal)
    if (errSignal) await shot(`tab-error-${ui}`)
    await evaluate(`${T}.upsert({ id: 'e2e-hdr-a', title: 'Agent', status: 'working', cwd: ${JSON.stringify(WORK)} }); true`)
    ok('working status shows the working signal', await wait(`${TAB}.parentElement.querySelector('[data-tab-signal]')?.dataset.tabSignal === 'working'`, 5000, 'working signal'))
    await shot(`tab-working-${ui}`)
    await evaluate(`${T}.upsert({ id: 'e2e-hdr-a', title: 'Agent', status: 'idle', cwd: ${JSON.stringify(WORK)} }); true`)
    ok('idle tab shows no signal', await wait(`!document.querySelector('[data-tab-item] [data-tab-signal]')`, 5000, 'no signal'))
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`header-${ui}-error`).catch(() => undefined)
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
  fs.rmSync(BASE, { recursive: true, force: true })
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
