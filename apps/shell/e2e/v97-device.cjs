const os = require('node:os')
const path = require('node:path')
const fs = require('node:fs')
const http = require('node:http')
const REPO = path.resolve(__dirname, '../../..')
const OUT_DIR = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v97-device')
process.env.CCM_E2E_SHELL = 'tauri'
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4421'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9345'
process.env.CCM_E2E_OUT = OUT_DIR
const { launch } = require(path.join(REPO, 'apps/desktop/e2e/cdp.cjs'))
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const UI = `${S}.get('ui')`
const D = 'window.ccmDesktop'
const CTRL_PORT = Number(process.env.CCM_DEVICE_CTRL) || 4556
const REMOTE_PORT = Number(process.env.CCM_E2E_REMOTE_PORT) || 4331
const EXE = process.platform === 'win32' ? '.exe' : ''
const CORE = process.env.CCM_CORE_BIN || path.join(REPO, 'apps/core/target/debug/orrery-core' + EXE)
const GOOGLE_SERVICES = path.join(REPO, 'apps/mobile/android/app/google-services.json')
const SERVICE_ACCOUNT = process.env.CCM_SERVICE_ACCOUNT || 'D:\\work\\secrets\\orrery-remote-firebase-adminsdk-fbsvc-757994c02c.json'
const SHOTS = path.join(REPO, 'docs/qa/v97/WP-22')
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const safe = (v) => {
  const text = JSON.stringify(v ?? null)
  return text.replace(/-----BEGIN[^"]*/g, '[redacted]').slice(0, 4000)
}

const body = (req) =>
  new Promise((resolve) => {
    let data = ''
    req.on('data', (d) => (data += d))
    req.on('end', () => resolve(data))
  })

;(async () => {
  let app
  let server
  const shutdown = async () => {
    try {
      await app?.evaluate(`${D}.remoteEnable(false)`)
    } catch {}
    server?.close()
    app?.stop()
    process.exit(0)
  }
  try {
    if (!fs.existsSync(CORE)) throw new Error('thiếu binary: ' + CORE)
    app = await launch({ keep: !!process.env.CCM_DEVICE_KEEP, env: { CCM_E2E_CONFIRM: 'ok', CCM_REMOTE_PORT: String(REMOTE_PORT), CCM_CORE_BIN: CORE, CCM_KEYRING_SERVICE: 'Orrery E2E v97 device' } })
    const { evaluate, waitFor, shot } = app
    await waitFor(`!!${D} && !!${UI}`, 40000, 'app ready')
    const version = require(path.join(REPO, 'apps/desktop/package.json')).version
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.explorerOpen','0'); localStorage.setItem('ccm.whatsNew.seen','${version}')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${UI}`, 40000, 'reloaded')
    await evaluate(`${UI}.view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    const proj = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-device-proj-'))
    fs.writeFileSync(path.join(proj, 'hello.txt'), 'orrery a21s hello\nxin chào\n')
    fs.writeFileSync(path.join(proj, 'README.md'), '# demo\n')
    await evaluate(`${W}.open('shell', ${JSON.stringify(proj)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 25000, 'shell running')
    server = http.createServer(async (req, res) => {
      const url = new URL(req.url, 'http://x')
      const reply = (code, obj) => {
        res.writeHead(code, { 'content-type': 'application/json' })
        res.end(JSON.stringify(obj))
      }
      try {
        if (url.pathname === '/eval') return reply(200, { value: JSON.parse(safe(await evaluate(await body(req)))) })
        if (url.pathname === '/shot') {
          fs.mkdirSync(SHOTS, { recursive: true })
          const name = url.searchParams.get('name') || 'desktop'
          await shot(name)
          fs.copyFileSync(path.join(OUT_DIR, name + '.png'), path.join(SHOTS, name + '.png'))
          return reply(200, { ok: true })
        }
        if (url.pathname === '/info') return reply(200, { proj, remotePort: REMOTE_PORT })
        if (url.pathname === '/push-setup') {
          const gs = fs.readFileSync(GOOGLE_SERVICES, 'utf8')
          const sa = fs.readFileSync(SERVICE_ACCOUNT, 'utf8')
          const r = await evaluate(`${D}.remotePushSetup(${JSON.stringify(gs)}, ${JSON.stringify(sa)})`)
          return reply(200, { value: JSON.parse(safe(r)) })
        }
        if (url.pathname === '/quit') {
          reply(200, { ok: true })
          return shutdown()
        }
        reply(404, { error: 'unknown' })
      } catch (e) {
        reply(500, { error: String(e.message).replace(/-----BEGIN[^"]*/g, '[redacted]').slice(0, 400) })
      }
    })
    server.listen(CTRL_PORT, '127.0.0.1', () => console.log('READY ' + CTRL_PORT))
  } catch (e) {
    console.log('FAIL run:', String(e.stack || e.message).slice(0, 800))
    app?.stop()
    process.exit(1)
  }
})()
process.on('SIGINT', () => process.exit(0))
