const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4511'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9511'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-footer-sys')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CHIP = `document.querySelector('[data-footer-sys]')`
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.footerSys')`

async function run(ui) {
  const footerSel = ui === 'next' ? '[data-next-footer]' : '[data-ws-footer]'
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')

    await waitFor(`!!${CHIP}`, 20000, 'chip').then(() => ok('chip shows in the footer', true), () => ok('chip shows in the footer', false))
    ok('chip sits inside the footer', await evaluate(`!!${CHIP}?.closest('${footerSel}')`))
    ok('chip is on the right half of the footer', await evaluate(`(() => { const c = ${CHIP}?.getBoundingClientRect(); const f = document.querySelector('${footerSel}')?.getBoundingClientRect(); return !!c && !!f && c.left > f.left + f.width / 2 })()`))

    await waitFor(`!!${CHIP}?.querySelector("[data-sys-cpu]")`, 15000, "cpu sample").catch(() => undefined)
    const vals = await evaluate(`({ cpu: ${CHIP}?.querySelector('[data-sys-cpu]')?.textContent.trim(), ram: ${CHIP}?.querySelector('[data-sys-ram]')?.textContent.trim() })`)
    const inRange = (s) => { const m = /^(\d{1,3})%$/.exec(s ?? ''); return !!m && Number(m[1]) >= 0 && Number(m[1]) <= 100 }
    ok('cpu is a number 0 to 100', inRange(vals.cpu), JSON.stringify(vals))
    ok('ram is a number 0 to 100', inRange(vals.ram), JSON.stringify(vals))
    const labels = await evaluate(`${CHIP}?.textContent.replace(/\\s+/g, ' ').trim()`)
    ok('chip has CPU and RAM labels', /CPU/.test(labels) && /RAM/.test(labels), labels)
    const tip = await evaluate(`${CHIP}?.getAttribute('title')`)
    ok('tooltip has threads and GB', /^CPU \d+%, \d+ threads\nRAM [\d.]+ \/ [\d.]+ GB$/.test(tip ?? ''), JSON.stringify(tip))

    await evaluate(`${S}.get('sysStats').sample = { cpu: 91, ram: 40, totalBytes: 17179869184, usedBytes: 6871947674, threads: 8 }; true`)
    await sleep(120)
    const warn = await evaluate(`({ cpu: ${CHIP}?.querySelector('[data-sys-cpu]')?.dataset.sysLevel, ram: ${CHIP}?.querySelector('[data-sys-ram]')?.dataset.sysLevel, icon: !!${CHIP}?.querySelector('[data-sys-cpu] svg') })`)
    ok('85 percent and above shows the warning level with an icon', warn.cpu === 'warn' && warn.ram === 'ok' && warn.icon === true, JSON.stringify(warn))

    await evaluate(`${CHIP}.click(); true`)
    await waitFor(`${S}.get('resources').open === true`, 8000, 'resources open').then(() => ok('click opens Resources', true), () => ok('click opens Resources', false))
    await evaluate(`${S}.get('resources').open = false; true`)

    await evaluate(`const u = ${S}.get('ui'); u.settingsPage = 'appearance'; u.settingsOpen = true; true`)
    await waitFor(`!!document.querySelector('[data-setting-row="footerSys"]')`, 10000, 'setting row').then(() => ok('Settings Appearance has the switch', true), () => ok('Settings Appearance has the switch', false))
    await evaluate(`document.querySelector('[data-setting-row="footerSys"] [role="switch"]').click(); true`)
    await sleep(500)
    ok('switch off hides the chip', await evaluate(`!${CHIP}`))
    ok('switch off stores 0', (await evaluate(`localStorage.getItem('ccm.footerSys')`)) === '0')
    ok('switch off drops the sample', await evaluate(`${S}.get('sysStats').sample === null`))
    await sleep(4500)
    ok('no sampling while switched off', await evaluate(`${S}.get('sysStats').sample === null`))
    await evaluate(`document.querySelector('[data-setting-row="footerSys"] [role="switch"]').click(); true`)
    await waitFor(`!!${CHIP}`, 10000, 'chip back').then(() => ok('switch on brings the chip back', true), () => ok('switch on brings the chip back', false))
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    if (failed) await shot(`footer-sys-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`footer-sys-${ui}-error`).catch(() => undefined)
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
