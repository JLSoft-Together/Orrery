const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const SHOTS = path.join(REPO, 'docs/qa/v103/WP-71')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const LONG = 'a-really-long-file-name-that-needs-horizontal-scrolling-in-a-deep-tree.component.test.ts'
const CARD = '[data-explorer-drawer-card]'
const TREE = '[data-explorer-tree]'
const CANVAS = '[data-next-canvas]'

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-v103-71-${process.pid}`)
  fs.rmSync(root, { recursive: true, force: true })
  const child = spawn(process.execPath, [path.join(REPO, 'scripts/demo-fixture.mjs'), `--root=${root}`, '--sessions=3', '--subagents=1'], { stdio: ['ignore', 'pipe', 'ignore'], windowsHide: true })
  await new Promise((resolve) => {
    let b = ''
    child.stdout.on('data', (d) => { b += d; if (/fixture root/.test(b)) resolve() })
    setTimeout(resolve, 8000)
  })
  const proj = path.join(os.tmpdir(), `ccm-e2e-v103-71-proj-${process.pid}`)
  fs.rmSync(proj, { recursive: true, force: true })
  let dir = proj
  const names = []
  for (let i = 1; i <= 9; i++) { dir = path.join(dir, `level-${i}`); names.push(`level-${i}`) }
  fs.mkdirSync(dir, { recursive: true })
  fs.writeFileSync(path.join(dir, LONG), 'x')
  fs.writeFileSync(path.join(proj, 'short.ts'), 'x')
  return { root, child, proj, names }
}

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const demo = await fixture()
  const app = await launch({ ui: 'next', env: { CLAUDE_CONFIG_DIR: demo.root, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, sleep, cmd, key, shot } = app
  const rect = async (sel) => JSON.parse(await evaluate(`JSON.stringify(document.querySelector(${JSON.stringify(sel)})?.getBoundingClientRect() ?? null)`))
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
    await sleep(300)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const state = () => evaluate(`document.querySelector('[data-explorer-drawer]')?.dataset.state`)
  const pageFits = () => evaluate(`document.scrollingElement.scrollWidth <= window.innerWidth && document.scrollingElement.scrollHeight <= window.innerHeight`)
  const paneRights = async () => JSON.parse(await evaluate(`JSON.stringify([...document.querySelectorAll('[data-pane]')].map((e) => e.getBoundingClientRect().right))`))
  const freeCanvas = () => evaluate(`document.querySelector('${CANVAS}').clientWidth - parseFloat(getComputedStyle(document.querySelector('${CANVAS}')).paddingRight)`)
  const gripX = async () => Math.round((await rect('[data-explorer-grip]')).left + 4)
  const drag = async (delta) => {
    const from = await gripX()
    const y = 400
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from, y })
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: from, y, button: 'left', buttons: 1, clickCount: 1 })
    const steps = 8
    for (let i = 1; i <= steps; i++) {
      await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from + (delta * i) / steps, y, button: 'left', buttons: 1 })
      await sleep(30)
    }
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: from + delta, y, button: 'left', buttons: 0, clickCount: 1 })
    await sleep(300)
  }
  const rowByName = (n) => `[...document.querySelectorAll('${TREE} [data-row-name]')].find((e) => e.textContent === ${JSON.stringify(n)})`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.next.explorerPinned'); localStorage.removeItem('ccm.next.explorerWidth'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui') && document.documentElement.dataset.ui === 'next'`, 30000, 'reloaded next')
    await size(1700, 950)
    await sleep(800)
    await evaluate(`${S}.get('settings').setThemeMode('light')`)
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(demo.proj)})`)
    await waitFor(`${W}.panes.length >= 1 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'project shell')
    await sleep(800)

    await evaluate(`document.querySelector('[data-explorer-toggle]').click()`)
    await waitFor(`document.querySelector('[data-explorer-drawer]')?.dataset.state === 'open'`, 5000, 'drawer open')
    await sleep(500)
    check('grip separator exists with a width value', await evaluate(`(() => { const g = document.querySelector('[data-explorer-grip]'); return !!g && g.getAttribute('role') === 'separator' && Number(g.getAttribute('aria-valuenow')) === 288 })()`))
    let r = await rect(CARD)
    check('drawer opens from the right edge of the canvas', Math.abs(r.width - 288) <= 1 && (await evaluate(`Math.abs(document.querySelector('${CARD}').getBoundingClientRect().right - document.querySelector('${CANVAS}').getBoundingClientRect().right) <= 1`)), JSON.stringify(r))

    await drag(-100)
    r = await rect(CARD)
    check('dragging the left edge widens the floating drawer', Math.abs(r.width - 388) <= 2, String(r.width))
    check('drawer width is stored', await evaluate(`localStorage.getItem('ccm.next.explorerWidth') === '${Math.round(r.width)}'`), await evaluate(`localStorage.getItem('ccm.next.explorerWidth')`))
    await drag(600)
    r = await rect(CARD)
    check('dragging right stops at the 220px minimum', Math.abs(r.width - 220) <= 1, String(r.width))
    await drag(-900)
    r = await rect(CARD)
    check('dragging left stops at the 640px maximum', Math.abs(r.width - 640) <= 1, String(r.width))
    const gx = await gripX()
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: gx, y: 400 })
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: gx, y: 400, button: 'left', buttons: 1, clickCount: 2 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: gx, y: 400, button: 'left', buttons: 0, clickCount: 2 })
    await sleep(300)
    r = await rect(CARD)
    check('double click on the edge resets to 288', Math.abs(r.width - 288) <= 1, String(r.width))
    await evaluate(`document.querySelector('[data-explorer-grip]').focus()`)
    await key('ArrowLeft', 'ArrowLeft', 37)
    await sleep(250)
    check('ArrowLeft on the focused edge widens by 16', Math.abs((await rect(CARD)).width - 304) <= 1)
    await evaluate(`${S}.get('uiGen').setExplorerWidth(288)`)
    await sleep(200)

    await waitFor(`!!document.querySelector('${TREE} [data-row-index]')`, 10000, 'tree rows')
    for (const name of demo.names) {
      await waitFor(`!!(${rowByName(name)})`, 8000, 'row ' + name)
      await evaluate(`(${rowByName(name)}).closest('[role=treeitem]').click()`)
      await sleep(250)
    }
    await waitFor(`!!(${rowByName(LONG)})`, 8000, 'long file row')
    const sc = JSON.parse(await evaluate(`JSON.stringify((() => { const t = document.querySelector('${TREE}'); const n = ${rowByName(LONG)}; return { cw: t.clientWidth, sw: t.scrollWidth, ox: getComputedStyle(t).overflowX, nameFull: n.scrollWidth <= n.clientWidth + 1, rowW: n.closest('[role=treeitem]').getBoundingClientRect().width } })())`))
    check('deep tree makes the tree area scroll horizontally', sc.ox === 'auto' && sc.sw > sc.cw + 40, JSON.stringify(sc))
    check('long name is not cut with an ellipsis', sc.nameFull, JSON.stringify(sc))
    check('rows fill the full content width', Math.abs(sc.rowW - sc.sw) <= 2, JSON.stringify(sc))
    check('page itself does not scroll horizontally', await pageFits())
    await evaluate(`document.querySelector('${TREE}').scrollLeft = 1e5`)
    await sleep(300)
    const end = JSON.parse(await evaluate(`JSON.stringify((() => { const t = document.querySelector('${TREE}'); const n = ${rowByName(LONG)}; const rr = n.getBoundingClientRect(); const tr = t.getBoundingClientRect(); return { right: rr.right, treeRight: tr.right, sl: t.scrollLeft } })())`))
    check('scrolled to the end the whole long name is visible', end.sl > 0 && end.right <= end.treeRight + 1, JSON.stringify(end))
    check('file rows keep their tooltip', await evaluate(`!!document.querySelector('${TREE} [role=treeitem][title]')`))
    const head = JSON.parse(await evaluate(`JSON.stringify((() => { const t = document.querySelector('${TREE}'); const r = t.querySelector('[aria-level="1"]'); const rr = r.getBoundingClientRect(); const tr = t.getBoundingClientRect(); const btns = [...r.querySelectorAll('button')].map((b) => b.getBoundingClientRect()); return { rowLeft: rr.left, treeLeft: tr.left, rowW: rr.width, cw: t.clientWidth, sl: t.scrollLeft, btns: btns.length, btnsInside: btns.every((b) => b.right <= tr.right + 1 && b.left >= tr.left - 1) } })())`))
    check('tree header row stays fixed while the tree scrolls sideways', head.sl > 0 && Math.abs(head.rowLeft - head.treeLeft) <= 1 && Math.abs(head.rowW - head.cw) <= 2 && head.btns > 0 && head.btnsInside, JSON.stringify(head))
    await snap('01-light-drawer-scrolled')
    await evaluate(`document.querySelector('${TREE}').scrollLeft = 0`)

    await evaluate(`document.querySelector('[data-explorer-pin]').click()`)
    await sleep(700)
    check('pin makes the Explorer a fixed column', (await state()) === 'pinned' && (await evaluate(`localStorage.getItem('ccm.next.explorerPinned') === '1'`)), await state())
    const col = await rect(CARD)
    const cv = await rect(CANVAS)
    check('pinned column sits at the right edge', Math.abs(col.right - cv.right) <= 1 && Math.abs(col.width - 288) <= 1, JSON.stringify({ col, cv }))
    const panes = await paneRights()
    check('panes shrink and do not sit under the column', panes.length > 0 && Math.max(...panes) <= col.left + 1, JSON.stringify({ panes, left: col.left }))
    check('pinned column is solid, not glass', await evaluate(`(() => { const s = getComputedStyle(document.querySelector('${CARD}')); return (s.backdropFilter === 'none' || s.backdropFilter === '') && !/\\/ 0\\.[0-8]/.test(s.backgroundColor) })()`), await evaluate(`getComputedStyle(document.querySelector('${CARD}')).backgroundColor + ' ' + getComputedStyle(document.querySelector('${CARD}')).backdropFilter`))
    check('page does not scroll when pinned', await pageFits())
    check('no scrim over the panes when pinned', await evaluate(`!document.querySelector('[data-explorer-scrim]')`))

    await drag(-120)
    const col2 = await rect(CARD)
    const panes2 = await paneRights()
    check('dragging the pinned edge widens the column', Math.abs(col2.width - 408) <= 2, String(col2.width))
    check('panes follow the new column width', Math.max(...panes2) <= col2.left + 1 && Math.max(...panes2) < Math.max(...panes), JSON.stringify({ panes2, left: col2.left }))
    await drag(-900)
    const wide = await rect(CARD)
    const free = await freeCanvas()
    check('pinned column never leaves the canvas under 560px', free >= 559 && wide.width <= 640, JSON.stringify({ free, w: wide.width }))
    check('page does not scroll at the widest column', await pageFits())
    await snap('02-light-pinned')

    await evaluate(`${S}.get('uiGen').setExplorerWidth(333)`)
    await sleep(300)
    await cmd('Page.reload')
    await waitFor(`document.documentElement.dataset.ui === 'next' && !!document.querySelector('[data-explorer-drawer]')`, 30000, 'reloaded')
    await sleep(1500)
    check('width and pin are stored', (await evaluate(`localStorage.getItem('ccm.next.explorerWidth')`)) === '333' && (await evaluate(`localStorage.getItem('ccm.next.explorerPinned')`)) === '1')
    await waitFor(`${W}.panes.length > 0`, 20000, 'panes back')
    await sleep(1200)
    const back = `${await state()} ${(await rect(CARD))?.width}`
    check('reload restores the pinned column at 333px', (await state()) === 'pinned' && Math.abs((await rect(CARD)).width - 333) <= 1, back)

    for (const w of [1366, 1280]) {
      await size(w, 800)
      await sleep(1200)
      const c = await rect(CARD)
      const fr = await freeCanvas()
      const pr = await paneRights()
      check('pinned at window width ' + w, (await state()) === 'pinned', await state())
      check(w + ': canvas keeps at least 560px and the page does not scroll', fr >= 559 && (await pageFits()), String(fr))
      check(w + ': panes stay left of the column', pr.length > 0 && Math.max(...pr) <= c.left + 1, JSON.stringify({ pr, left: c.left }))
      await snap('05-light-pinned-' + w)
    }
    await evaluate(`${S}.get('uiGen').setExplorerWidth(640)`)
    await sleep(500)
    const tight = await rect(CARD)
    check('column shrinks to fit at 1280 and keeps the stored width', (await state()) === 'pinned' && tight.width < 640 && tight.width >= 220 && (await freeCanvas()) >= 559 && (await evaluate(`localStorage.getItem('ccm.next.explorerWidth')`)) === '640', JSON.stringify({ w: tight.width, free: await freeCanvas() }))
    await cmd('Emulation.setDeviceMetricsOverride', { width: 700, height: 800, deviceScaleFactor: 1, mobile: false })
    await sleep(1200)
    check('too narrow to pin: falls back to the drawer with a blocked pin tooltip', (await state()) !== 'pinned' && (await evaluate(`(() => { const b = document.querySelector('[data-explorer-pin]'); return !b || (b.dataset.pinBlocked === 'canvas' && /560/.test(b.title)) })()`)), await state())
    await cmd('Emulation.clearDeviceMetricsOverride')
    await size(1700, 950)
    await sleep(1000)
    await evaluate(`${S}.get('uiGen').setExplorerWidth(333)`)
    await sleep(500)
    await evaluate(`${S}.get('settings').setThemeMode('dark')`)
    await sleep(600)
    await snap('03-dark-pinned')
    await evaluate(`document.querySelector('[data-explorer-pin]').click()`)
    await sleep(700)
    check('unpin returns to the floating drawer', (await state()) === 'open', await state())
    check('canvas padding is released after unpin', await evaluate(`parseFloat(getComputedStyle(document.querySelector('${CANVAS}')).paddingRight) < 20`))
    await snap('04-dark-drawer')
    await key('E', 'KeyE', 69, 2 | 8)
    await sleep(500)
    const st1 = await state()
    await key('E', 'KeyE', 69, 2 | 8)
    await sleep(500)
    const st2 = await state()
    check('Ctrl+Shift+E toggles the drawer', st1 !== st2, `${st1} -> ${st2}`)
    await evaluate(`${S}.get('settings').setThemeMode('system')`)
  } catch (e) {
    check('run', false, e.message)
    await shot('v103-71-99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    try { fs.rmSync(demo.proj, { recursive: true, force: true }) } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v103-71.txt'), results.join('\n'))
    process.exit(0)
  }
})()
