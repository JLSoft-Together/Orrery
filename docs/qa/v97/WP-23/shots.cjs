const os = require('node:os')
const path = require('node:path')
const fs = require('node:fs')
const REPO = path.resolve(__dirname, '../../../..')
const OUT_DIR = path.join(os.tmpdir(), 'ccm-e2e-wp23')
process.env.CCM_E2E_SHELL = 'tauri'
process.env.CCM_E2E_PORT = '4432'
process.env.CCM_E2E_DBG = '9352'
process.env.CCM_E2E_OUT = OUT_DIR
const { launch } = require(path.join(REPO, 'apps/desktop/e2e/cdp.cjs'))
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const UI = `${S}.get('ui')`
const ST = `${S}.get('settings')`
const SHOTS = __dirname
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const remoteDir = path.join(OUT_DIR, 'run', 'data', 'remote')
fs.rmSync(path.join(OUT_DIR, 'run', 'data'), { recursive: true, force: true })
fs.mkdirSync(remoteDir, { recursive: true })
const now = Math.floor(Date.now() / 1000)
const key = Buffer.alloc(32, 7).toString('base64url')
const dev = (id, name, platform, ago) => ({ deviceId: id, name, platform, pubkey: key, pairedAt: now - 86400 * 3, lastSeen: now - ago, tokenExp: 0 })
fs.writeFileSync(path.join(remoteDir, 'devices.json'), JSON.stringify({ devices: [dev('p1', 'Pixel 7', 'android', 3600), dev('p2', 'iPhone 15 Pro', 'ios', 7200), dev('p3', 'SM-A217F', 'android', 90000)] }))

const realRm = fs.rmSync
const KEEP = path.join(OUT_DIR, 'run')
fs.rmSync = (p, ...rest) => (String(p) === KEEP ? undefined : realRm(p, ...rest))

;(async () => {
  const app = await launch({ env: { CCM_REMOTE_PORT: '4330', CCM_KEYRING_SERVICE: 'Orrery E2E WP23' } })
  const { evaluate, waitFor, shot, cmd } = app
  const names = []
  try {
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 40000, 'app ready')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1180, height: 900, deviceScaleFactor: 1, mobile: false })
    await evaluate(`for (const k of ['ccm.setupDone', 'ccm.tourDone', 'ccm.tourWorkspaceDone']) localStorage.setItem(k, '1')`)
    await cmd('Page.reload')
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 40000, 'reloaded')
    await evaluate(`[...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Để sau')?.click()`)
    await evaluate(`(async () => { await window.ccmDesktop.remoteEnable(true) })()`)
    await sleep(800)
    console.log('devices', await evaluate(`(async () => JSON.stringify((await window.ccmDesktop.remoteStatus()).devices.map((d) => [d.deviceId, d.name, d.originalName, d.alias])))()`))
    const mode = async (theme) => {
      await evaluate(`${ST}.setTheme('${theme}')`)
      await sleep(300)
    }
    const open = async () => {
      await evaluate(`${UI}.settingsPage = 'general'; ${UI}.settingsOpen = true`)
      await sleep(200)
      await evaluate(`${UI}.settingsPage = 'remote'`)
      await sleep(700)
    }
    const take = async (name) => {
      await evaluate(`document.querySelector('[data-remote-device="p1"]')?.scrollIntoView({ block: 'center' })`)
      await sleep(500)
      await shot(name)
      fs.copyFileSync(path.join(OUT_DIR, name + '.png'), path.join(SHOTS, name + '.png'))
      names.push(name)
      console.log('shot', name)
    }
    const type = async (id, text) => {
      await evaluate(`(() => { const e = document.querySelector('[data-remote-device="${id}"] [data-remote-rename-input]'); e.value = ${JSON.stringify(text)}; e.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await sleep(200)
    }
    const key2 = async (id, k) => {
      await evaluate(`document.querySelector('[data-remote-device="${id}"] [data-remote-rename-input]').dispatchEvent(new KeyboardEvent('keydown', { key: '${k}', bubbles: true, cancelable: true }))`)
      await sleep(700)
    }
    const edit = async (id) => {
      await evaluate(`document.querySelector('[data-remote-device="${id}"] [data-remote-rename]').click()`)
      await sleep(400)
    }
    for (const theme of ['orrery-light', 'galaxy']) {
      const tag = theme === 'galaxy' ? 'dark' : 'light'
      await mode(theme)
      await evaluate(`(async () => { for (const id of ['p1', 'p2', 'p3']) await window.ccmDesktop.remoteDeviceRename(id, '') })()`)
      await open()
      await take(`rename-1-list-${tag}`)
      await edit('p1')
      await type('p1', 'Work phone')
      await take(`rename-2-editing-${tag}`)
      await key2('p1', 'Enter')
      await take(`rename-3-saved-${tag}`)
      await edit('p2')
      await type('p2', 'My daily iPhone with a very long nickname 📱📱📱📱📱📱📱📱📱📱')
      await key2('p2', 'Enter')
      await take(`rename-4-long-${tag}`)
      await edit('p1')
      await type('p1', 'Discarded')
      await key2('p1', 'Escape')
      await take(`rename-5-esc-${tag}`)
      await edit('p1')
      await type('p1', '   ')
      await key2('p1', 'Enter')
      await take(`rename-6-restored-${tag}`)
    }
    console.log('final', await evaluate(`(async () => JSON.stringify((await window.ccmDesktop.remoteStatus()).devices.map((d) => [d.deviceId, d.name, d.alias])))()`))
    fs.writeFileSync(path.join(SHOTS, 'shots.txt'), names.join('\n') + '\n')
  } finally {
    app.stop()
  }
})().catch((e) => {
  console.error(e)
  process.exit(1)
})
