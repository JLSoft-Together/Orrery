const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4624'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9624'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112d')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-112d-')))
const WORK = path.join(BASE, 'work')
fs.mkdirSync(WORK, { recursive: true })
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','claude'); localStorage.setItem('ccm.workspace.recent', JSON.stringify([${JSON.stringify(WORK)}])); localStorage.removeItem('ccm.launchTemplates'); localStorage.removeItem('ccm.projectDefaults'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

async function run(ui) {
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const ok = (s, v, extra) => check(`${ui}: ${s}`, !!v, extra)
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const setField = (sel, value) => evaluate(`(() => { const el = document.querySelector('${sel}'); const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : el.tagName === 'SELECT' ? HTMLSelectElement.prototype : HTMLInputElement.prototype; Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, ${JSON.stringify(value)}); el.dispatchEvent(new Event(el.tagName === 'SELECT' ? 'change' : 'input', { bubbles: true })); return true })()`)
  const key = async (k, code, vk, text) => {
    await cmd('Input.dispatchKeyEvent', { type: 'keyDown', key: k, code, windowsVirtualKeyCode: vk, ...(text ? { text } : {}) })
    await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: vk })
    await sleep(250)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'; true`)
    ok('launcher shows', await wait(`!!document.querySelector('[data-launch-templates]')`, 15000, 'launcher'))
    await evaluate(`document.querySelector('[data-launch-tpl-new]').click(); true`)
    ok('new template form opens', await wait(`!!document.querySelector('[data-launch-tpl-form]')`, 5000, 'form'))
    await setField('[data-launch-tpl-name]', 'Plan review')
    await setField('[data-launch-tpl-model]', 'opus; calc')
    ok('bad model blocks save', await wait(`document.querySelector('[data-launch-tpl-save]').disabled === true`, 3000, 'blocked'))
    await setField('[data-launch-tpl-model]', 'opus')
    await setField('[data-launch-tpl-permission]', 'plan')
    await setField('[data-launch-tpl-effort]', 'high')
    await setField('[data-launch-tpl-prompt]', 'Read the repo and propose a plan')
    await shot(`template-form-${ui}`)
    await evaluate(`document.querySelector('[data-launch-tpl-save]').click(); true`)
    ok('template saved to ccm.launchTemplates', await wait(`(JSON.parse(localStorage.getItem('ccm.launchTemplates') || '[]')[0] || {}).permissionMode === 'plan'`, 5000, 'saved'))
    ok('template chip selected with its args shown', await wait(`document.querySelector('[data-launch-tpl][aria-pressed="true"]') && /--permission-mode plan/.test(document.querySelector('[data-launch-tpl-args]')?.textContent || '')`, 5000, 'chip'))
    await shot(`template-selected-${ui}`)
    await evaluate(`document.querySelector('[data-launch-recent]').focus(); true`)
    await key('Enter', 'Enter', 13, '\r')
    ok('Enter on the recent project opens a claude pane', await wait(`${W}.panes.some((p) => p.preset === 'claude' && p.cwd === ${JSON.stringify(WORK)})`, 15000, 'pane'))
    const pane = await evaluate(`JSON.stringify(${W}.panes.find((p) => p.preset === 'claude'))`)
    const p = JSON.parse(pane || '{}')
    ok('pane carries model, permission and effort of the template', p.launch && p.launch.model === 'opus' && p.launch.permission === 'plan' && p.launch.effort === 'high', JSON.stringify(p.launch))
    ok('first prompt is prefilled and not submitted', typeof p.prefill === 'string' && /propose a plan/.test(p.prefill) && !p.prefillSubmit)
    const def = await evaluate(`JSON.parse(localStorage.getItem('ccm.projectDefaults') || '{}')`)
    const entry = Object.values(def)[0] || {}
    ok('project default remembers agent and template', entry.agent === 'claude' && !!entry.template, JSON.stringify(def))
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`launch-${ui}-error`).catch(() => undefined)
  } finally {
    app.stop()
  }
}

;(async () => {
  try {
    await run('next')
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  fs.rmSync(BASE, { recursive: true, force: true })
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
