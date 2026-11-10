process.env.CCM_E2E_SHELL = 'tauri'
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const http = require('node:http')
const REPO = path.resolve(__dirname, '../../..')
const { launch, OUT } = require(path.join(REPO, 'apps/desktop/e2e/cdp.cjs'))
const WS = require(path.join(REPO, 'node_modules/ws'))
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const DBG = Number(process.env.CCM_E2E_DBG) || 9333
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`)
  console.log(results.at(-1))
}
const DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-t1-'))
const BACKUP = path.join(DIR, 'backup.json')

const page = (title, body = '') => `<!doctype html><html><head><title>${title}</title></head><body style="margin:0"><h1>${title}</h1>${body}</body></html>`
const server = http.createServer((req, res) => {
  const u = req.url.split('?')[0]
  if (u === '/favicon.ico') return res.writeHead(404).end()
  const body = u.startsWith('/b') ? page('Fixture B') : page('Fixture A', '<script>console.error("boom-load")</script><a id="ext" href="https://example.invalid/" target="_blank">x</a>')
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' }).end(body)
})

const getJson = (url) =>
  new Promise((ok, no) =>
    http
      .get(url, (r) => {
        let b = ''
        r.on('data', (d) => (b += d))
        r.on('end', () => {
          try {
            ok(JSON.parse(b))
          } catch (e) {
            no(e)
          }
        })
      })
      .on('error', no),
  )

async function guestTarget(prefix) {
  const list = await getJson(`http://127.0.0.1:${DBG}/json`).catch(() => [])
  return list.filter((t) => t.type === 'page' && t.url.startsWith(prefix))
}

async function guestEval(prefix, expr) {
  const [t] = await guestTarget(prefix)
  if (!t) return undefined
  const ws = new WS(t.webSocketDebuggerUrl)
  await new Promise((r, j) => (ws.once('open', r), ws.once('error', j)))
  const out = await new Promise((r) => {
    ws.on('message', (raw) => {
      const m = JSON.parse(raw)
      if (m.id === 1) r(m.result?.result?.value)
    })
    ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: { expression: expr, returnByValue: true, awaitPromise: true } }))
  })
  ws.close()
  return out
}

;(async () => {
  await new Promise((r) => server.listen(0, '127.0.0.1', r))
  const FX = `http://127.0.0.1:${server.address().port}`
  const app = await launch({ env: { CCM_E2E_BACKUP_FILE: BACKUP, CCM_E2E_CONFIRM: 'cancel' } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const box = `document.querySelector('[data-kind="browser"]')`
  const wv = `${box}?.querySelector('webview')`
  const exec = (js) => evaluate(`${wv}.executeJavaScript(${JSON.stringify(js)})`)
  const until = async (fn, ms = 8000) => {
    const end = Date.now() + ms
    while (Date.now() < end) {
      if (await fn().catch(() => false)) return true
      await sleep(200)
    }
    return false
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.explorerOpen','0'); localStorage.setItem('ccm.whatsNew.seen','${require(path.join(REPO, 'apps/desktop/package.json')).version}')`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')

    await evaluate(`${W}.openBrowser(${JSON.stringify(FX + '/a')}, { cwd: ${JSON.stringify(DIR)} })`)
    await waitFor(`!!${wv}`, 8000, 'webview element')
    const loaded = await until(async () => (await exec('document.title')) === 'Fixture A', 15000)
    check('child webview loads the page, executeJavaScript reads it', loaded)
    check('getURL() is the page URL', (await evaluate(`${wv}.getURL()`)) === FX + '/a', await evaluate(`${wv}.getURL()`))
    check('executeJavaScript awaits a promise', (await exec('new Promise((r) => setTimeout(() => r(42), 50))')) === 42)
    const rejected = await evaluate(`${wv}.executeJavaScript('Promise.reject(new Error("nope"))').then(() => 'resolved', (e) => String(e))`)
    check('executeJavaScript rejects on a page error', /nope/.test(rejected), rejected)
    const titled = await until(async () => (await evaluate(`${W}.panes.find((p) => p.kind === 'browser').title`)) === 'Fixture A')
    check('page-title-updated reaches the pane title', titled)
    const [target] = await guestTarget(FX)
    check('guest page is visible over CDP (shared data dir)', !!target)
    const sizes = await evaluate(`(() => { const r = ${wv}.getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)] })()`)
    const inner = await guestEval(FX, '[innerWidth, innerHeight, document.visibilityState]')
    check('guest follows the element size', inner && Math.abs(inner[0] - sizes[0]) <= 2 && Math.abs(inner[1] - sizes[1]) <= 2 && inner[2] === 'visible', JSON.stringify({ sizes, inner }))
    await shot('t1-01-browser')

    await evaluate(`window.__con = []; ${wv}.addEventListener('console-message', (e) => window.__con.push(e.level + ':' + e.message))`)
    await exec('console.error("boom-x")')
    check('console errors arrive as console-message', await until(async () => (await evaluate('window.__con.join("|")')).includes('error:boom-x')))

    await exec(`history.pushState({}, '', '/a/deeper')`)
    check('in-page navigation updates getURL and the address', await until(async () => (await evaluate(`${wv}.getURL()`)).endsWith('/a/deeper') && (await evaluate(`${box}.querySelector('input[aria-label="Address"]')?.value ?? ''`)).includes('/a/deeper')))

    await exec(`location.href = '/b'`)
    check('navigation to another page loads it', await until(async () => (await exec('document.title').catch(() => '')) === 'Fixture B'))
    check('canGoBack after a navigation', await until(async () => await evaluate(`${wv}.canGoBack()`)))
    await evaluate(`${wv}.goBack()`)
    check('goBack returns to the first page', await until(async () => (await exec('document.title').catch(() => '')) === 'Fixture A'))
    check('canGoForward after going back', await until(async () => await evaluate(`${wv}.canGoForward()`)))

    await exec(`location.href = 'file:///C:/Windows/win.ini'`).catch(() => undefined)
    await sleep(1200)
    check('navigation to a file: URL is blocked', (await evaluate(`${wv}.getURL()`)).startsWith(FX), await evaluate(`${wv}.getURL()`))
    check('window.open is denied', (await exec(`window.open('https://example.invalid/') === null`)) === true)

    await evaluate(`${box}.querySelector('[data-pane-more]').click()`)
    await waitFor(`!!document.querySelector('[data-context-menu]')`, 5000, 'more menu')
    const hidden = await until(async () => (await guestEval(FX, 'document.visibilityState')) === 'hidden', 5000)
    check('guest hides while an HTML menu is open', hidden)
    await shot('t1-02-menu-over-guest')
    await key('Escape', 'Escape', 27)
    check('guest shows again after the menu closes', await until(async () => (await guestEval(FX, 'document.visibilityState')) === 'visible', 5000))
    await sleep(500)

    await evaluate(`${box}.querySelector('[data-pane-more]').click()`)
    await waitFor(`!!document.querySelector('[data-context-menu] [data-menu-id="device"]')`, 5000, 'more menu')
    await evaluate(`document.querySelector('[data-context-menu] [data-menu-id="device"]').click()`)
    await waitFor(`!!document.querySelector('[role=menu] [role=menuitemradio]')`, 5000, 'device menu')
    await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitemradio]')].find((b) => /iPhone/.test(b.textContent)).click()`)
    const phone = await until(async () => (await guestEval(FX, 'innerWidth')) === 390, 6000)
    check('device frame: guest lays out at 390 px under the scaled frame', phone, JSON.stringify([await guestEval(FX, '[innerWidth, devicePixelRatio, outerWidth]'), await evaluate(`(() => { const r = ${wv}.getBoundingClientRect(); return [r.width, ${wv}.offsetWidth] })()`)]))
    await shot('t1-03-iphone')
    await sleep(500)
    await evaluate(`${box}.querySelector('[data-pane-more]').click()`)
    await waitFor(`!!document.querySelector('[data-context-menu] [data-menu-id="device"]')`, 5000, 'more menu again')
    await evaluate(`document.querySelector('[data-context-menu] [data-menu-id="device"]').click()`)
    await waitFor(`!!document.querySelector('[role=menu] [role=menuitemradio]')`, 5000, 'device menu again')
    await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitemradio]')].find((b) => /Responsive/.test(b.textContent)).click()`)
    await key('Escape', 'Escape', 27)
    await sleep(500)

    const copied = await evaluate(`window.ccmDesktop.captureGuest(${wv}.getWebContentsId())`)
    const types = await evaluate(`navigator.clipboard.read().then((items) => items.flatMap((i) => i.types).join(','), (e) => 'err ' + e)`)
    check('captureGuest puts a PNG of the guest on the clipboard', copied === true && /image\/png/.test(types), `${copied} ${types}`)
    const snap = await evaluate(`${wv}.capturePage().then((i) => i.isEmpty() ? '' : i.toDataURL().slice(0, 22))`)
    check('capturePage returns a PNG data URL', snap === 'data:image/png;base64,', snap)

    await exec(`dispatchEvent(new KeyboardEvent('keydown', { key: 'P', code: 'KeyP', ctrlKey: true, shiftKey: true, bubbles: true }))`)
    const palette = await until(async () => await evaluate(`!!document.querySelector('[role=combobox]')`), 5000)
    check('Ctrl+Shift+P inside the guest is forwarded to the app (quick open)', palette)
    if (palette) await key('Escape', 'Escape', 27)

    const rect = await evaluate(`window.ccmDesktop.captureRect({ x: 0, y: 0, width: 200, height: 100 }).then(async (b) => { if (!b?.byteLength) return null; const u = new Uint8Array(b); const img = await createImageBitmap(new Blob([u], { type: 'image/png' })); return [u[1], u[2], u[3], img.width, img.height] })`)
    check('captureRect (dust) returns a PNG at CSS size', Array.isArray(rect) && rect[0] === 80 && rect[3] === 200 && rect[4] === 100, JSON.stringify(rect))
    const bad = await evaluate(`window.ccmDesktop.captureRect({ x: 0, y: 0, width: 'a', height: 1 })`)
    check('captureRect refuses a bad rect', bad === null, JSON.stringify(bad))

    const exported = await evaluate(`window.ccmDesktop.backup.export()`)
    const text = fs.existsSync(BACKUP) ? fs.readFileSync(BACKUP, 'utf8') : ''
    check('backup export runs in core-desk and writes the file', exported?.ok === true && exported.file === BACKUP && /"format": "orrery-backup"/.test(text), JSON.stringify(exported))
    check('backup holds the page localStorage (readLocal host)', text.includes('ccm.locale'))
    const preview = await evaluate(`window.ccmDesktop.backup.pick()`)
    check('backup pick previews the file', preview?.ok === true && typeof preview.token === 'string', JSON.stringify(preview).slice(0, 160))
    const applied = await evaluate(`window.ccmDesktop.backup.apply(${JSON.stringify(preview?.token ?? '')}, ['settings'])`)
    const asked = fs.existsSync(path.join(OUT, 'run', 'data', 'e2e-confirms.log')) ? fs.readFileSync(path.join(OUT, 'run', 'data', 'e2e-confirms.log'), 'utf8') : ''
    check('backup apply asks a confirm first (answered Cancel)', applied?.ok === false && applied.error === 'cancelled' && /Import 1 section and restart Orrery\?/.test(asked), `${JSON.stringify(applied)} ${asked.trim()}`)
    const scan = await evaluate(`window.ccmDesktop.buildScan([${JSON.stringify(DIR)}]).then((v) => Array.isArray(v) ? 'array' : JSON.stringify(v), (e) => 'err ' + e)`)
    check('build scan runs in core-desk', scan === 'array', scan)
    const unknown = await evaluate(`window.__TAURI_INTERNALS__.invoke('ccm', { channel: 'desk:backup-apply-pending', args: [] }).then(() => 'ran', (e) => String(e))`)
    check('internal desk channels are not reachable from the page', /not ported/.test(unknown), unknown)
    const denied = await guestEval(FX, `window.__TAURI_INTERNALS__.invoke('ccm', { channel: 'ccm:zoom', args: ['get'] }).then(() => 'ran', (e) => 'denied ' + e)`)
    check('a guest page cannot call the app channel', /denied|not allowed/i.test(String(await denied)), String(denied))

    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reload with guest')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await evaluate(`${W}.activate(${W}.panes.find((p) => p.kind === 'browser').key)`)
    check('restored browser pane opens its guest again', await until(async () => (await exec('document.title').catch(() => '')) === 'Fixture A', 15000))
    check('the old guest was closed on reload (one target left)', await until(async () => (await guestTarget(FX)).length === 1, 6000), String((await guestTarget(FX)).length))

    await evaluate(`${W}.remove(${W}.panes.find((p) => p.kind === 'browser').key)`)
    check('closing the pane closes the guest', await until(async () => (await guestTarget(FX)).length === 0, 8000))
  } catch (e) {
    check('run', false, e.message)
    await shot('t1-99-error').catch(() => {})
  } finally {
    app.stop()
    server.close()
    fs.writeFileSync(path.join(OUT, 'results-t1.txt'), results.join('\n'))
    try {
      fs.rmSync(DIR, { recursive: true, force: true })
    } catch {}
    process.exit(0)
  }
})()
