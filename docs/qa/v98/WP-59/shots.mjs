import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'

const here = path.dirname(fileURLToPath(import.meta.url))
const repo = path.resolve(here, '../../../..')
const require = createRequire(path.join(repo, 'scripts/spike-v98/wasm/package.json'))
const { chromium } = require('playwright')
const PORT = 5199
const now = Math.floor(Date.now() / 1000)

const STATE = {
  rev: 7,
  trusted: ['192.168.1.0/24', '2001:db8:aa::/48'],
  blocked: [
    { key: '198.51.100.0/24', source: 'manual', why: 'manual', count: 0, at: now - 7200, until: null, level: 0 },
    { key: '203.0.113.66', source: 'relay', why: 'ticket', count: 20, at: now - 600, until: now + 3000, level: 1 },
    { key: '2001:db8:1:2::/64', source: 'relay', why: 'token', count: 23, at: now - 3600, until: now + 82_800, level: 2 },
    { key: '192.168.1.77', source: 'lan', why: 'code', count: 20, at: now - 120, until: now + 3480, level: 1 },
  ],
  log: [
    { at: now - 30, ip: '203.0.113.66', via: 'relay', country: 'RU', result: 'fail', why: 'ticket' },
    { at: now - 120, ip: '192.168.1.77', via: 'lan', result: 'blocked', why: 'code' },
    { at: now - 300, ip: '192.168.1.23', via: 'lan', device: 'Pixel 9', result: 'ok', why: 'session' },
    { at: now - 900, ip: '198.51.100.4', via: 'relay', country: 'VN', device: 'iPhone 16', result: 'ok', why: 'paired' },
    { at: now - 1000, ip: '198.51.100.9', via: 'relay', country: 'VN', device: 'Galaxy', result: 'denied', why: 'confirm' },
  ],
}

const MOCK = `(() => {
  for (const k of ['ccm.setupDone', 'ccm.tourDone', 'ccm.tourWorkspaceDone']) localStorage.setItem(k, '1')
  const state = ${JSON.stringify(STATE)}
  const status = { enabled: true, listening: true, addresses: ['192.168.1.2'], port: 4320, relay: 'online', keepAwake: true, devices: [], guardRev: 7 }
  window.ccmDesktop = {
    platform: 'win32',
    remoteStatus: async () => status,
    onRemoteChanged: () => () => {},
    remoteEnable: async () => status,
    remoteGuard: async () => ({ ok: true, state }),
  }
})()`

const vite = spawn(process.execPath, [path.join(repo, 'node_modules/vite/bin/vite.js'), '--port', String(PORT), '--strictPort'], { cwd: path.join(repo, 'apps/web'), stdio: 'ignore' })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
try {
  await sleep(6000)
  const browser = await chromium.launch({ channel: 'chrome' })
  for (const theme of ['orrery-light', 'galaxy']) {
    const page = await browser.newPage({ viewport: { width: 900, height: 1500 } })
    await page.addInitScript(MOCK)
    await page.addInitScript(`localStorage.setItem('ccm.theme', '${theme}'); localStorage.setItem('ccm.locale', 'vi')`)
    await page.goto(`http://127.0.0.1:${PORT}/`)
    await page.waitForFunction(() => !!document.querySelector('#app')?.__vue_app__, null, { timeout: 30000 })
    await page.evaluate(() => {
      const s = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s
      const ui = s.get('ui')
      ui.settingsOpen = true
      ui.settingsPage = 'remote'
      ui.whatsNewOpen = false
    })
    await page.waitForSelector('[data-remote-guard-blocked]', { timeout: 15000 })
    await sleep(1500)
    await page.evaluate(() => { document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('ui').whatsNewOpen = false })
    await sleep(500)
    const tone = theme.includes('light') ? 'light' : 'dark'
    for (const id of ['remoteBlocked', 'remoteTrusted', 'remoteAccess']) {
      const el = page.locator(`[data-setting-row="${id}"]`)
      await el.scrollIntoViewIfNeeded()
      await sleep(300)
      await el.screenshot({ path: path.join(here, `${id}-${tone}.png`) })
    }
    await page.close()
  }
  await browser.close()
  console.log('ok')
} finally {
  vite.kill()
}
