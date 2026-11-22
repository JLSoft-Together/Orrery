const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const R = `${S}.get('remote')`
const G = `${S}.get('uiGen')`
const SET = `${S}.get('settings')`
const SHOTS = path.join(REPO, 'docs/qa/v98/WP-67')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-wp67-${process.pid}`)
  fs.rmSync(root, { recursive: true, force: true })
  const child = spawn(process.execPath, [path.join(REPO, 'scripts/demo-fixture.mjs'), `--root=${root}`, '--sessions=3', '--subagents=1'], { stdio: ['ignore', 'pipe', 'ignore'], windowsHide: true })
  await new Promise((resolve) => {
    let b = ''
    child.stdout.on('data', (d) => { b += d; if (/fixture root/.test(b)) resolve() })
    setTimeout(resolve, 8000)
  })
  return { root, child }
}

const NOW = Date.now()
const SEC = Math.floor(NOW / 1000)
const dev = (id, name, over = {}) => ({ deviceId: id, name, platform: 'android', pairedAt: NOW - 86_400_000, lastSeen: NOW - 60_000, state: 'idle', via: 'lan', push: 'fcm', pushOn: true, pushContent: true, ...over })
const status = (devices, over = {}) => ({ enabled: true, listening: true, addresses: ['192.168.1.20'], port: 4330, relay: 'online', keepAwake: true, devices, pairing: null, pairEnded: null, guardRev: 1, ...over })
const PHONES = [dev('d1', 'Pixel 8', { state: 'connected' }), dev('d2', 'iPhone 15', { platform: 'web', push: 'web', via: 'relay', state: 'connected' }), dev('d3', 'Galaxy Tab', { pushOn: false })]
const BLOCK = { key: '203.0.113.7', source: 'relay', why: 'token', count: 6, at: SEC - 120, until: SEC + 3600, level: 1 }

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const demo = await fixture()
  const app = await launch({ ui: 'next', env: { CLAUDE_CONFIG_DIR: demo.root, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, sleep, cmd, key } = app
  const size = async (width, height) => {
    try {
      const w = await cmd('Browser.getWindowForTarget')
      if (w.result?.windowId) {
        await cmd('Browser.setWindowBounds', { windowId: w.result.windowId, bounds: { windowState: 'normal', width, height } })
        return
      }
    } catch {}
    await cmd('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false })
  }
  const snap = async (name) => {
    await cmd('Page.bringToFront')
    await sleep(350)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const json = async (expr) => JSON.parse(await evaluate(`JSON.stringify(${expr})`))
  const setStatus = (s) => evaluate(`${R}.status = ${JSON.stringify(s)}; true`)
  const chip = () => json(`(() => { const c = document.querySelector('[data-next-rail-foot] [data-phone-chip]'); return c ? { tone: c.dataset.tone, connected: Number(c.dataset.connected), blocked: c.dataset.blocked ?? null, pushOff: c.dataset.pushOff ?? null, text: c.querySelector('[data-phone-chip-text]')?.textContent.trim() ?? '', label: c.getAttribute('aria-label'), badge: c.querySelector('[data-phone-chip-badge]')?.textContent.trim() ?? null, w: c.getBoundingClientRect().width } : null })()`)
  const theme = async (palette, mode) => {
    await evaluate(`${SET}.themeFollow = false; ${G}.setNextTheme(${JSON.stringify(palette)}); ${SET}.setThemeMode(${JSON.stringify(mode)}); true`)
    await sleep(500)
  }
  const scrollTo = (sel) => evaluate(`(() => { const row = document.querySelector(${JSON.stringify(sel)}); if (!row) return false; let p = row.parentElement; while (p && !(p.scrollHeight > p.clientHeight + 4 && /auto|scroll/.test(getComputedStyle(p).overflowY))) p = p.parentElement; if (!p) return false; p.scrollTop += row.getBoundingClientRect().top - p.getBoundingClientRect().top - 8; return true })()`).then(() => sleep(300))
  const closeSettings = async () => {
    await evaluate(`${S}.get('ui').settingsOpen = false`)
    await sleep(400)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.next.railCollapsed'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui') && document.documentElement.dataset.ui === 'next'`, 30000, 'reloaded next')
    await size(1600, 900)
    await sleep(800)
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`[...document.querySelectorAll('[data-launch-row]')].at(-1).click()`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await evaluate(`${W}.split(${W}.panes[0].key, 'row')`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'two shells')
    await theme('mist', 'light')

    check('remote store is available in this build', await evaluate(`${R}.supported === true`))
    await setStatus(status([], { enabled: false }))
    await sleep(400)
    check('no phone chip while remote control is off', (await chip()) === null)

    await setStatus(status([]))
    await sleep(500)
    let c = await chip()
    check('chip shows in the rail foot once remote control is on', !!c && c.tone === 'idle' && c.connected === 0, JSON.stringify(c))
    check('idle chip says Dien thoai and explains in the label', !!c && c.text === 'Điện thoại' && /Chưa ghép/.test(c.label), JSON.stringify(c))
    const foot = await json(`(() => { const f = document.querySelector('[data-next-rail-channels]'); const c = f?.querySelector('[data-phone-chip]'); return f && c ? { inFoot: !!c.closest('[data-next-rail-foot]'), footer: !!c.closest('[data-next-footer]') } : null })()`)
    check('chip sits with the remote channels in the rail foot, not in the quota footer', !!foot && foot.inFoot && !foot.footer, JSON.stringify(foot))

    await setStatus(status(PHONES))
    await sleep(600)
    c = await chip()
    check('two connected phones: live tone and count', !!c && c.tone === 'live' && c.connected === 2 && c.text === '2 điện thoại', JSON.stringify(c))
    check('label lists connected, push and how to open', !!c && /2\/3/.test(c.label) && /Push bật cho 1\/2|Push bật cho 2\/3/.test(c.label) && /Cài đặt/.test(c.label), c?.label)
    check('push still on for some phones: no push-off mark', !!c && c.pushOff === null)
    await snap('01-chip-live-mist-light')

    await setStatus(status(PHONES.map((d) => ({ ...d, pushOn: false }))))
    await sleep(400)
    c = await chip()
    check('push off on every phone shows the push-off mark', !!c && c.pushOff !== null, JSON.stringify(c))

    await setStatus(status(PHONES))
    await evaluate(`${R}.blocked = [${JSON.stringify(BLOCK)}, ${JSON.stringify({ ...BLOCK, key: '10.0.0.0/8', source: 'manual' })}]; true`)
    await sleep(500)
    c = await chip()
    check('a recent automatic block turns the chip into a warning with icon and text', !!c && c.tone === 'alert' && c.blocked === '1' && c.text === 'Chặn 1 IP', JSON.stringify(c))
    await snap('02-chip-alert-mist-light')

    await evaluate(`document.querySelector('[data-phone-chip]').click()`)
    await waitFor(`!!document.querySelector('[data-settings-remote]')`, 8000, 'settings remote')
    await sleep(700)
    const page = await json(`(() => { const d = document.querySelector('.mist-settings'); const r = document.querySelector('[data-settings-remote]'); return { glass: !!d, current: document.querySelector('[data-settings-page][aria-current="page"]')?.dataset.settingsPage ?? null, rows: document.querySelectorAll('[data-remote-devices] > *').length, bell: !!document.querySelector('[data-setting-row="remoteDevices"]'), blocked: !!document.querySelector('[data-setting-row="remoteBlocked"]'), flash: !!document.querySelector('[data-setting-row="remoteBlocked"].ccm-setting-flash'), over: r ? r.scrollWidth - r.clientWidth : -1 } })()`)
    check('chip click opens Settings on the Phone page in the glass dialog', page.glass && page.current === 'remote', JSON.stringify(page))
    check('Phone page lists the 3 paired phones and the blocked list', page.rows === 3 && page.bell && page.blocked, JSON.stringify(page))
    check('alert chip lands on the blocked IP section', page.flash, JSON.stringify(page))
    check('Phone page does not scroll sideways', page.over <= 0, JSON.stringify(page))
    const groups = await json(`[...document.querySelectorAll('[data-settings-remote] .ccm-group')].map((g) => { const s = getComputedStyle(g); return s.backgroundColor + '|' + s.borderTopColor }).slice(0, 2)`)
    check('Phone page groups use the next glass surface', groups.length > 0 && groups.every((g) => !/rgba?\(0, 0, 0, 0\)\|/.test(g)), JSON.stringify(groups))
    await scrollTo('[data-setting-row="remote"]')
    await snap('03-settings-phone-mist-light')
    await scrollTo('[data-setting-row="remoteDevices"]')
    await snap('04-settings-devices-mist-light')
    await closeSettings()

    await evaluate(`${R}.blocked = []; true`)
    await sleep(300)
    await evaluate(`document.querySelector('.xterm-helper-textarea')?.blur()`)
    await evaluate(`${S}.get('ui').paletteOpen = true`)
    await waitFor(`!!document.querySelector('[data-palette-ui="next"]')`, 5000, 'palette')
    await sleep(300)
    await app.type('ghép')
    await sleep(600)
    const hit = await json(`[...document.querySelectorAll('[data-palette-ui] [role=option]')].map((o) => o.textContent.replace(/\\s+/g, ' ').trim()).slice(0, 8)`)
    check('Ctrl+K finds Ghep dien thoai and Thiet bi da ghep', hit.some((t) => /Ghép điện thoại/.test(t)) && hit.some((t) => /Thiết bị đã ghép/.test(t) && /2\/3 đang nối/.test(t)), JSON.stringify(hit))
    await snap('05-palette-phone-mist-light')
    await evaluate(`[...document.querySelectorAll('[data-palette-ui] [role=option]')].find((o) => /Ghép điện thoại/.test(o.textContent))?.click()`)
    await waitFor(`!!document.querySelector('[data-remote-pair]')`, 8000, 'pair panel')
    await sleep(500)
    check('Ctrl+K Ghep dien thoai opens the pairing panel on the Phone page', await evaluate(`!!document.querySelector('[data-settings-remote] [data-remote-pair]') && ${R}.pairWanted === false`))
    await snap('06-settings-pair-mist-light')
    await closeSettings()

    for (const [pal, mode, name] of [['mist', 'dark', '07-chip-mist-dark'], ['mono', 'light', '08-chip-mono-light'], ['galaxy', 'dark', '09-chip-galaxy-dark']]) {
      await theme(pal, mode)
      c = await chip()
      check(`chip readable in ${pal} ${mode}`, !!c && c.tone === 'live' && c.w > 40, JSON.stringify(c))
      await snap(name)
    }
    await evaluate(`${R}.blocked = [${JSON.stringify(BLOCK)}]; true`)
    await sleep(400)
    await evaluate(`${S}.get('ui').settingsPage = 'remote'; ${S}.get('ui').settingsOpen = true`)
    await waitFor(`!!document.querySelector('[data-settings-remote]')`, 8000, 'settings remote dark')
    await sleep(600)
    await scrollTo('[data-setting-row="remoteDevices"]')
    await snap('10-settings-devices-galaxy-dark')
    await scrollTo('[data-setting-row="remoteBlocked"]')
    await snap('11-settings-guard-galaxy-dark')
    await closeSettings()
    await theme('mono', 'dark')
    await evaluate(`${S}.get('ui').settingsPage = 'remote'; ${S}.get('ui').settingsOpen = true`)
    await waitFor(`!!document.querySelector('[data-settings-remote]')`, 8000, 'settings remote mono')
    await sleep(600)
    await snap('12-settings-phone-mono-dark')
    await closeSettings()
    await evaluate(`${R}.blocked = [${JSON.stringify(BLOCK)}]; true`)

    await evaluate(`document.querySelector('[data-rail-collapse]')?.click()`)
    await sleep(700)
    c = await json(`(() => { const c = document.querySelector('[data-next-rail-foot] [data-phone-chip]'); return c ? { tone: c.dataset.tone, badge: c.querySelector('[data-phone-chip-badge]')?.textContent.trim() ?? null, text: !!c.querySelector('[data-phone-chip-text]'), w: c.getBoundingClientRect().width } : null })()`)
    check('collapsed rail keeps an icon chip with the blocked count badge', !!c && c.tone === 'alert' && c.badge === '1' && !c.text && c.w <= 34, JSON.stringify(c))
    await snap('13-chip-collapsed-mono-dark')
    await evaluate(`${R}.blocked = []; true`)
    await sleep(400)
    c = await json(`(() => { const c = document.querySelector('[data-next-rail-foot] [data-phone-chip]'); return c ? { tone: c.dataset.tone, badge: c.querySelector('[data-phone-chip-badge]')?.textContent.trim() ?? null } : null })()`)
    check('collapsed live chip shows the connected count', !!c && c.tone === 'live' && c.badge === '2', JSON.stringify(c))
    await evaluate(`document.querySelector('[data-rail-collapse]')?.click()`)
    await sleep(500)

    await evaluate(`${G}.setMotion('off')`)
    await setStatus(status([]))
    await sleep(200)
    await setStatus(status(PHONES))
    await sleep(200)
    const off = await json(`(() => { const c = document.querySelector('[data-next-rail-foot] [data-phone-chip]'); return c ? { tone: c.dataset.tone, motion: document.documentElement.dataset.motion, dur: getComputedStyle(c, '::after').animationDuration, sig: c.dataset.signal ?? null } : null })()`)
    check('motion off: chip still updates without a signal ring animation', !!off && off.tone === 'live' && off.motion === 'off' && (off.dur === '' || parseFloat(off.dur) < 0.01), JSON.stringify(off))
    await evaluate(`${G}.motionChoice = null; localStorage.removeItem('ccm.next.motion'); ${G}.setNextTheme('mist'); true`)
  } catch (e) {
    check('run', false, e.message)
    await snap('99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v98-remote-next.txt'), results.join('\n'))
    const failed = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - failed} PASS, ${failed} FAIL`)
    process.exit(0)
  }
})()
