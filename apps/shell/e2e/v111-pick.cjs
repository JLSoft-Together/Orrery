const { spawn } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const { launch, OUT, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CTRL = 2, SHIFT = 8
const FIX = path.join(__dirname, 'fixtures', 'vue-pick')
const VITE_PORT = 5199
const SHOTS = process.env.CCM_111H_SHOTS || path.join(REPO, 'docs', 'qa', 'v111', 'WP-111h')

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const vite = spawn(process.execPath, [path.join(REPO, 'node_modules', 'vite', 'bin', 'vite.js'), '--port', String(VITE_PORT)], { cwd: FIX, stdio: 'ignore' })
  const app = await launch({ env: process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {} })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const box = `document.querySelector('[data-kind="browser"]')`
  const guestJs = (js) => `(async () => { const w = ${box}?.querySelector('webview'); return w ? await w.executeJavaScript(${JSON.stringify(js)}) : null })()`
  const guest = (js) => evaluate(guestJs(js))
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.workspace.explorerOpen','0'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.theme','dark')`)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(FIX)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(1500)
    const agentKey = await evaluate(`${W}.panes[0].key`)
    await evaluate(`${W}.panes[0].preset = 'claude'; window.__ins = []; window.__att = []; ${W}.$onAction(({ name, args }) => { if (name === 'insertInto') window.__ins.push(String(args[1])) }); ${S}.get('attachments').$onAction(({ name, args }) => { if (name === 'add') window.__att.push(args[1].map((a) => ({ kind: a.kind, data: a.data, thumb: a.thumb }))) })`)

    await waitFor(`fetch('http://127.0.0.1:${VITE_PORT}/').then((r) => r.ok).catch(() => false)`, 30000, 'vite up').catch(() => null)
    await evaluate(`${W}.openBrowser('http://127.0.0.1:${VITE_PORT}/', { cwd: ${JSON.stringify(FIX)} })`)
    await waitFor(`${box}?.querySelector('webview')`, 10000, 'browser pane')
    await waitFor(`${guestJs("document.querySelector('#go') ? 'ok' : ''")}.then((v) => v === 'ok')`, 30000, 'fixture rendered')
    const browserKey = await evaluate(`${W}.panes.find((p) => p.kind === 'browser').key`)
    await evaluate(`${W}.activate(${JSON.stringify(browserKey)})`)
    await sleep(3000)

    const hasButton = await evaluate(`!!${box}.querySelector('[data-pick-element]')`)
    check('address bar has the Pick element button', hasButton)
    await shot('pick-01-toolbar')

    await cmd('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'C', code: 'KeyC', windowsVirtualKeyCode: 67, modifiers: CTRL | SHIFT })
    await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: 'C', code: 'KeyC', windowsVirtualKeyCode: 67, modifiers: CTRL | SHIFT })
    const keyStarted = await waitFor(`/Click an element/.test(document.body.innerText)`, 10000, 'ctrl+shift+c').catch(() => false)
    check('Ctrl+Shift+C (real key event, host focus) starts picking', !!keyStarted)
    await shot('pick-02-picking')
    await key('Escape', 'Escape', 27)
    const escaped = await waitFor(`!/Click an element/.test(document.body.innerText)`, 4000, 'esc').catch(() => false)
    check('Esc cancels picking', !!escaped)

    await sleep(1000)
    let forwarded = false
    for (let i = 0; i < 3 && !forwarded; i++) {
      await guest(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'C', code: 'KeyC', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))`)
      forwarded = await waitFor(`/Click an element/.test(document.body.innerText)`, 5000, 'guest chord').catch(() => false)
    }
    check('Ctrl+Shift+C while the page has focus is forwarded and starts picking', !!forwarded)
    await key('Escape', 'Escape', 27)
    await waitFor(`!/Click an element/.test(document.body.innerText)`, 4000, 'esc 2').catch(() => null)

    await evaluate(`${box}.querySelector('[data-pick-element]').click()`)
    await waitFor(`/Click an element/.test(document.body.innerText)`, 10000, 'button pick')
    check('Pick element button starts picking', true)
    const pos = await guest(`(() => { const r = document.querySelector('#go').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, h: r.height, vw: innerWidth } })()`)
    await guest(`(() => { const t = document.querySelector('#go'); const o = { bubbles: true, cancelable: true, clientX: ${pos.x}, clientY: ${pos.y} }; t.dispatchEvent(new MouseEvent('mousemove', o)); t.dispatchEvent(new MouseEvent('click', o)); return document.querySelectorAll('div[style*=fixed]').length + ' ' + true })()`)
    await waitFor(`window.__ins.length > 0`, 15000, 'insertInto').catch(async (e) => { console.log('DBG', await evaluate('JSON.stringify(window.__dbg)'), await evaluate('JSON.stringify(window.__att).slice(0,100)')); throw e })
    const text = await evaluate(`window.__ins[0]`)
    console.log(text)
    check('prefill has selector, text and URL', /Element on http:\/\/127\.0\.0\.1:5199\//.test(text) && /selector: .*#go/.test(text) && /text: "Sign in"/.test(text))
    check('prefill has source: relative path of the Vue component', /\nsource: src\/components\/LoginForm\.vue\n/.test(text), (text.match(/source: .*/) ?? [''])[0])
    check('prefill has css line with colour and font-size', /\ncss: .*color: rgb\(255, 255, 255\).*font-size: 18px/.test(text), (text.match(/css: .*/) ?? [''])[0])
    check('prefill is not submitted (no newline at the end)', !/\n$/.test(text))
    await waitFor(`window.__att.length > 0`, 10000, 'attachment')
    const att = await evaluate(`window.__att[0]`)
    check('cropped image added to the agent pane attachments', att.length === 1 && att[0].kind === 'image' && att[0].data.startsWith('data:image/png;base64,') && att[0].thumb.startsWith('data:image/png'))
    const dim = await evaluate(`new Promise((r) => { const i = new Image(); i.onload = () => r({ w: i.naturalWidth, h: i.naturalHeight }); i.src = window.__att[0][0].data })`)
    const k = dim.w / (pos.w + 16)
    check('crop is the element plus an 8px margin (not the whole page)', dim.w < pos.vw * 0.8 && dim.w >= pos.w && Math.abs(dim.h / (pos.h + 16) - k) < 0.2, JSON.stringify({ dim, el: [pos.w, pos.h] }))
    fs.writeFileSync(path.join(SHOTS, 'pick-crop.png'), Buffer.from(att[0].data.split(',')[1], 'base64'))
    const inAtt = await evaluate(`${S}.get('attachments').list(${JSON.stringify(agentKey)}).length`)
    check('attachments store holds the image for the agent pane', inAtt === 1, String(inAtt))
    await sleep(800)
    await shot('pick-03-after')
    fs.copyFileSync(path.join(OUT, 'pick-01-toolbar.png'), path.join(SHOTS, 'pick-01-toolbar.png'))
    fs.copyFileSync(path.join(OUT, 'pick-02-picking.png'), path.join(SHOTS, 'pick-02-picking.png'))
    fs.copyFileSync(path.join(OUT, 'pick-03-after.png'), path.join(SHOTS, 'pick-03-after.png'))
  } catch (e) {
    check('run', false, e.message)
    await shot('pick-99-error').catch(() => {})
  } finally {
    app.stop()
    try { vite.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v111-pick.txt'), results.join('\n'))
    process.exit(0)
  }
})()
