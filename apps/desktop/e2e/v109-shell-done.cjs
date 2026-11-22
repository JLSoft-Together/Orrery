const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4517'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9517'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-shell-done')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const WIN = process.platform === 'win32'
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const DIR = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-shell-done-')))
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.shellIntegration','1'); localStorage.setItem('ccm.shellDone','1'); localStorage.setItem('ccm.shellDoneSeconds','1')`
const SLOW = WIN ? 'Start-Sleep -Milliseconds 1600; cmd /c exit 3' : "sleep 1.6; sh -c 'exit 3'"
const FAST = 'echo ok'

function freshCore() {
  const name = WIN ? 'orrery-core.exe' : 'orrery-core'
  const at = (p) => { try { return fs.statSync(p).mtimeMs } catch { return 0 } }
  const dbg = path.join(REPO, 'apps/core/target/debug', name)
  const rel = path.join(REPO, 'apps/core/target/release', name)
  return at(dbg) > at(rel) ? { CCM_CORE_BIN: dbg } : {}
}

async function main() {
  const app = await launch({ ui: 'classic', env: { CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', ...freshCore() } })
  const { evaluate, waitFor, shot, sleep, cmd, key, type } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(s, v, extra) }
  const run = async (k, text) => {
    await cmd('Page.bringToFront')
    await evaluate(`document.querySelector('[data-pane="${k}"] .xterm-helper-textarea')?.focus()`)
    await type(text)
    await key('Enter', 'Enter', 13)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    ok('threshold read from settings (1 s)', (await evaluate(`${S}.get('settings').shellDoneSeconds`)) === 1)

    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell A running')
    const a = await evaluate(`${W}.panes[0].key`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes.filter((p) => p.state === 'running').length >= 2`, 20000, 'shell B running')
    const b = await evaluate(`${W}.panes.find((p) => p.key !== '${a}').key`)
    await evaluate(`${W}.activate('${a}')`)
    await sleep(2500)

    await run(a, FAST)
    await waitFor(`${W}.panes.find((p) => p.key === '${a}')?.shellLast?.command === '${FAST}'`, 15000, 'fast cmd').catch(() => {})
    const fast = await evaluate(`JSON.stringify(${W}.panes.find((p) => p.key === '${a}').shellLast ?? null)`)
    ok('Rust PTY host reports the end mark with exit 0 (OSC 633 parity)', /"code":0/.test(fast) && fast.includes(FAST), fast)
    ok('a short command in sight sets no dot', await evaluate(`!${W}.panes.find((p) => p.key === '${a}').attention`))

    await run(a, SLOW)
    await sleep(150)
    await evaluate(`${W}.activate('${b}')`)
    await waitFor(`${W}.panes.find((p) => p.key === '${a}')?.shellLast?.code === 3`, 20000, 'slow cmd end').catch(() => {})
    const last = await evaluate(`JSON.stringify(${W}.panes.find((p) => p.key === '${a}').shellLast ?? null)`)
    const parsed = JSON.parse(last) ?? {}
    ok('long command end arrives with exit 3', parsed.code === 3, last)
    ok('duration measured by the host (at least 1.5 s, under 15 s)', parsed.ms >= 1500 && parsed.ms < 15000, String(parsed.ms))
    ok('command line comes from the shell (nonce trusted)', parsed.command === SLOW, parsed.command)
    ok('out of sight: attention dot latched on the pane', (await evaluate(`${W}.panes.find((p) => p.key === '${a}').attention`)) === 'done')
    ok('tab shows the failed tone', await evaluate(`!!document.querySelector('[data-tab-attention="fail"]')`))
    const focused = await evaluate(`document.visibilityState === 'visible' && document.hasFocus()`)
    if (focused) {
      await waitFor(`!!document.querySelector('[data-shell-done-toast="${a}"]')`, 5000, 'toast').catch(() => {})
      ok('in-app toast with exit 3 (window focused, other tab)', (await evaluate(`document.querySelector('[data-shell-done-toast="${a}"]')?.getAttribute('data-shell-done-exit')`)) === '3')
      ok('toast text names exit code and command', await evaluate(`(() => { const t = document.querySelector('[data-shell-done-toast="${a}"]')?.textContent ?? ''; return t.includes('exit 3') && (t.includes('Start-Sleep') || t.includes('sleep 1.6')) })()`))
    } else console.log('INFO window not focused under CDP: OS toast path (core toaster, CCM_TOAST=0 in E2E)')

    await evaluate(`${W}.activate('${a}')`)
    await sleep(600)
    ok('seeing the pane clears the dot', await evaluate(`!${W}.panes.find((p) => p.key === '${a}').attention`))
    await waitFor(`[...document.querySelectorAll('[data-pane="${a}"] [data-shell-mark]')].some((m) => m.dataset.mark === 'fail')`, 8000, 'gutter mark').catch(() => {})
    ok('gutter marks are on for the Tauri shell (ready.si)', await evaluate(`[...document.querySelectorAll('[data-pane="${a}"] [data-shell-mark]')].some((m) => m.dataset.mark === 'fail')`))

    await evaluate(`const u = ${S}.get('ui'); u.settingsPage = 'editor'; u.settingsOpen = true; true`)
    await waitFor(`!!document.querySelector('[data-setting-row="shellDone"]')`, 10000, 'setting row').then(() => ok('Settings Editor has the switch', true), () => ok('Settings Editor has the switch', false))
    ok('seconds field shows 1', (await evaluate(`document.querySelector('[data-setting="shell-done-seconds"]')?.value`)) === '1')
    await evaluate(`document.querySelector('[data-setting-row="shellDone"] [role="switch"]').click(); true`)
    await sleep(300)
    ok('switch off stores 0 and hides the seconds field', (await evaluate(`localStorage.getItem('ccm.shellDone')`)) === '0' && (await evaluate(`!document.querySelector('[data-setting="shell-done-seconds"]')`)))
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    ok('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= innerWidth`))
    if (failed) await shot('shell-done-fail')
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot('shell-done-error').catch(() => undefined)
  } finally {
    app.stop()
    await new Promise((r) => setTimeout(r, 800))
    try { fs.rmSync(DIR, { recursive: true, force: true }) } catch {}
  }
}

main().then(() => process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0))
