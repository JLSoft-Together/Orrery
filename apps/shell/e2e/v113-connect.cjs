const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const R = `${S}.get('remote')`
const SHOTS = path.join(REPO, 'docs/qa/v113/WP-113e2')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const NOW = Date.now()
const status = (devices, over = {}) => ({ enabled: true, listening: true, addresses: ['192.168.1.20'], port: 4330, relay: 'online', keepAwake: true, devices, pairing: null, pairEnded: null, guardRev: 1, ...over })
const phone = (over = {}) => ({ deviceId: 'dq1', name: 'Galaxy A33', platform: 'android', pairedAt: NOW, lastSeen: NOW, state: 'connected', via: 'lan', push: 'fcm', pushOn: true, pushContent: true, ...over })

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const app = await launch({ ui: 'next', env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, sleep, cmd } = app
  const snap = async (name) => {
    await cmd('Page.bringToFront')
    await sleep(350)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const click = async (sel) => { await evaluate(`document.querySelector(${JSON.stringify(sel)})?.click()`); await sleep(400) }
  const has = (sel) => evaluate(`!!document.querySelector(${JSON.stringify(sel)})`)
  const wizState = () => evaluate(`document.querySelector('[data-connect-wizard]')?.dataset.connectState ?? null`)
  const setStatus = (s) => evaluate(`${R}.status = ${JSON.stringify(s)}; true`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })

    await setStatus(status([phone()]))
    await sleep(500)
    await evaluate(`document.querySelector('[data-phone-chip]')?.click()`)
    await waitFor(`!!document.querySelector('[data-settings-remote]')`, 10000, 'settings remote')
    check('hero card replaces the old toggle block and keeps data-remote-enable', await has('[data-connect-hero] [data-remote-enable]'))
    check('device list and old selectors stay', (await has('[data-remote-devices]')) && (await has('[data-setting-row="remoteDevices"]')))
    check('advanced fold is closed by default', (await evaluate(`document.querySelector('[data-remote-more]')?.getAttribute('aria-expanded')`)) === 'false')
    check('log list stays in the DOM inside the fold', await has('[data-remote-log]'))
    await snap('01-hero-ready')

    await click('[data-remote-more]')
    check('advanced fold opens on click', (await evaluate(`document.querySelector('[data-remote-more]')?.getAttribute('aria-expanded')`)) === 'true')
    await snap('02-advanced-open')
    await click('[data-remote-more]')

    await setStatus(status([], { enabled: false, listening: false }))
    await sleep(400)
    await click('[data-connect-start]')
    check('consent shows when remote control is off', (await wizState()) === 'consent')
    await snap('03-consent')
    await click('[data-connect-consent-no]')
    check('cancel returns to the hero', (await wizState()) === null && (await has('[data-connect-hero]')))

    await setStatus(status([]))
    await sleep(400)
    await click('[data-connect-start]')
    check('install step shows when remote control is already on', (await wizState()) === 'install')
    check('no Choose APK in the main flow', !(await evaluate(`/Chọn APK|Choose APK/.test(document.querySelector('[data-connect-wizard]')?.textContent ?? '')`)))
    check('QR tile is offered', await has('[data-connect-qr-show]'))
    await snap('04-install')

    await click('[data-connect-qr-show]')
    const qrState = await wizState()
    check('QR path reaches qrInstall or reports a clear error', qrState === 'qrInstall', String(qrState))
    if (qrState === 'qrInstall') {
      check('QR link carries the install page and a countdown', (await has('[data-connect-qr-code]')) || (await has('[data-connect-qr-error]')))
      await snap('05-qr-install')
    }
    await click('[data-connect-back]')
    await click('[data-connect-installed]')
    check('already installed goes to the pairing QR with old selectors', (await has('[data-remote-pair]')) || (await wizState()) === 'pairWait')
    await snap('06-pair-wait')

    await setStatus(status([phone({ pairedAt: NOW + 5000 })]))
    await sleep(600)
    check('a new device moves the wizard to paired', ['paired', 'testing', 'testDone', 'testWarn'].includes(await wizState()))
    await snap('07-paired')
    await sleep(2500)
    check('then the test step runs', ['testing', 'testDone', 'testWarn'].includes(await wizState()))
    await snap('08-test')

    await setStatus(status([], { enabled: false, listening: false }))
    await sleep(500)
    check('turning remote control off elsewhere closes the wizard', (await wizState()) === null)
  } catch (err) {
    check('run completed without throwing', false, String(err && err.message ? err.message : err))
  } finally {
    console.log(results.join('\n'))
    app.stop()
    process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
  }
})()
