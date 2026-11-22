const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
let app = null
let shots = 0

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-reorder-')))
const DIRS = ['alpha', 'beta', 'gamma'].map((n) => path.join(BASE, n))
for (const d of DIRS) fs.mkdirSync(d)

async function check(ui, name, ok, extra = '') {
  results.push(`${ok ? 'PASS' : 'FAIL'} [${ui}] ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
  if (!ok && app) await app.shot(`rail-reorder-${ui}-fail-${++shots}`).catch(() => {})
}

const HEADS = `[...document.querySelectorAll('[data-live-rail] [data-side-head], nav[aria-label=Projects] [data-side-head]')].filter((h) => h.offsetParent)`
const headByKey = (k) => `${HEADS}.find((h) => h.dataset.orderKey === ${JSON.stringify(k)})`

async function keysNow() {
  return JSON.parse(await app.evaluate(`JSON.stringify(${HEADS}.map((h) => h.dataset.orderKey))`))
}

async function gripOf(key) {
  return JSON.parse(
    await app.evaluate(`(() => {
      const h = ${headByKey(key)}
      const g = (h.querySelector('[data-rail-name]') ?? h.querySelector(':scope > span.truncate') ?? h).getBoundingClientRect()
      return JSON.stringify({ x: g.left + 8, y: g.top + g.height / 2 })
    })()`),
  )
}

const mouse = (type, p, buttons = 1) => app.cmd('Input.dispatchMouseEvent', { type, x: p.x, y: p.y, button: 'left', buttons, clickCount: type === 'mouseMoved' ? 0 : 1 })

async function press(from, to, steps = 10) {
  await mouse('mouseMoved', from, 0)
  await mouse('mousePressed', from)
  for (let i = 1; i <= steps; i++) {
    await mouse('mouseMoved', { x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps })
    await app.sleep(30)
  }
  await app.sleep(150)
}

async function release(to) {
  await mouse('mouseReleased', to, 0)
  await app.sleep(400)
}

async function prepare(ui) {
  const { evaluate, cmd, waitFor, sleep } = app
  await waitFor(`!!${S}.get('ui')`, 30000, 'app ready')
  await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.sidebar','1'); localStorage.removeItem('ccm.workspace.railOrder')`)
  await cmd('Page.reload')
  await sleep(1500)
  await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
  await waitFor(`document.documentElement.dataset.ui === ${JSON.stringify(ui)}`, 20000, `data-ui=${ui}`)
  await evaluate(`${S}.get('ui').view = 'workspace'`)
}

async function openAll(order) {
  for (const d of order) await app.evaluate(`${W}.open('shell', ${JSON.stringify(d)}, undefined, { tab: true })`)
  await app.waitFor(`${W}.panes.length === ${order.length} && ${W}.panes.every((p) => p.state === 'running')`, 25000, 'shells running')
  await app.waitFor(`${HEADS}.length === ${order.length}`, 10000, 'project heads')
  await app.sleep(800)
}

async function phaseOne(ui) {
  await prepare(ui)
  await openAll(DIRS)
  const before = await keysNow()
  await check(ui, 'rail shows one header per project', before.length === 3, JSON.stringify(before))

  const top = await gripOf(before[0])
  const from = await gripOf(before[2])
  const up = { x: from.x, y: top.y - 14 }
  await press(from, up)
  await check(ui, 'dragging a header inside the rail shows the reorder line', await app.evaluate(`!!document.querySelector('[data-rail-reorder-line]:not([hidden])')`))
  await check(ui, 'a reorder drag inside the rail shows no pane drop preview', await app.evaluate(`!document.querySelector('[data-drop-preview]')`))
  await release(up)
  const afterUp = await keysNow()
  await check(ui, 'drag up moves the project to the top', JSON.stringify(afterUp) === JSON.stringify([before[2], before[0], before[1]]), `${JSON.stringify(before)} -> ${JSON.stringify(afterUp)}`)
  await check(ui, 'the reorder line is gone after the drop', await app.evaluate(`!document.querySelector('[data-rail-reorder-line]')`))

  const mid = await gripOf(afterUp[1])
  const last = await gripOf(afterUp[2])
  const down = { x: mid.x, y: last.y + 40 }
  await press(mid, down)
  await release(down)
  const afterDown = await keysNow()
  await check(ui, 'drag down moves the project to the bottom', JSON.stringify(afterDown) === JSON.stringify([afterUp[0], afterUp[2], afterUp[1]]), JSON.stringify(afterDown))

  const a = await gripOf(afterDown[2])
  const b = { x: a.x, y: (await gripOf(afterDown[0])).y - 14 }
  await press(a, b)
  await app.key('Escape', 'Escape', 27)
  await release(b)
  await check(ui, 'Esc cancels a reorder drag', JSON.stringify(await keysNow()) === JSON.stringify(afterDown))

  await check(ui, 'reorder leaves panes and tabs alone', (await app.evaluate(`${W}.tabs.length`)) === 3 && (await app.evaluate(`${W}.panes.length`)) === 3)
  const stored = JSON.parse((await app.evaluate(`localStorage.getItem('ccm.workspace.railOrder')`)) ?? '[]')
  await check(ui, 'the order is stored', JSON.stringify(stored.slice(0, 3)) === JSON.stringify(afterDown), JSON.stringify(stored))
  return afterDown
}

async function phaseTwo(ui, want) {
  await app.waitFor(`!!${S}.get('ui')`, 30000, 'relaunched')
  await app.evaluate(`${S}.get('ui').view = 'workspace'`)
  await app.sleep(2500)
  const storedNow = JSON.parse((await app.evaluate(`localStorage.getItem('ccm.workspace.railOrder')`)) ?? '[]')
  await check(ui, 'the stored order survives the restart', JSON.stringify(storedNow.slice(0, 3)) === JSON.stringify(want), `${JSON.stringify(storedNow)}`)
  const restored = await app.evaluate(`${W}.panes.length`)
  if (restored < 3) {
    for (const p of JSON.parse(await app.evaluate(`JSON.stringify(${W}.panes.map((p) => p.key))`))) await app.evaluate(`${W}.remove(${JSON.stringify(p)})`)
    await openAll(DIRS)
  } else await app.waitFor(`${HEADS}.length === 3`, 10000, 'project heads after restart')
  const keys = await keysNow()
  await check(ui, 'after a restart the rail keeps the dragged order', JSON.stringify(keys) === JSON.stringify(want), `${JSON.stringify(want)} vs ${JSON.stringify(keys)}`)

  const target = await app.evaluate(`${W}.activeTab ? JSON.stringify(${W}.activeTab.focus) : 'null'`).then(JSON.parse)
  const src = JSON.parse(await app.evaluate(`JSON.stringify((() => { const own = JSON.stringify(${W}.activeTab.root); const p = ${W}.panes.find((x) => !own.includes(JSON.stringify(x.key))); return p ? { key: p.key, project: String(p.project ?? p.cwd ?? '').toLowerCase() } : null })())`))
  const srcPane = src?.key ?? null
  const srcKey = keys.find((k) => src && k.includes(src.project)) ?? keys[0]
  const from = await gripOf(srcKey)
  const to = JSON.parse(await app.evaluate(`(() => { const r = document.querySelector('[data-pane=' + JSON.stringify(${JSON.stringify(target)}) + ']').getBoundingClientRect(); return JSON.stringify({ x: r.right - 12, y: r.top + r.height / 2 }) })()`))
  const tabsBefore = await app.evaluate(`${W}.tabs.length`)
  await press(from, to, 14)
  await check(ui, 'dragging a header out over a pane shows the split preview', await app.evaluate(`!!document.querySelector('[data-drop-preview]')`))
  await check(ui, 'the reorder line hides outside the rail', await app.evaluate(`!document.querySelector('[data-rail-reorder-line]:not([hidden])')`))
  await release(to)
  await app.sleep(400)
  const tabOf = (k) => `${W}.tabs.findIndex((t) => JSON.stringify(t.root).includes(${JSON.stringify(k)}))`
  const merged = srcPane ? await app.evaluate(`${tabOf(srcPane)} === ${tabOf(target)}`) : (await app.evaluate(`${W}.tabs.length`)) < tabsBefore
  await check(ui, 'dropping the header on a pane splits the project beside it', !!merged, `src ${srcPane} tabs ${tabsBefore} -> ${await app.evaluate(`${W}.tabs.length`)}`)
  await check(ui, 'a split drop does not reorder the rail', JSON.stringify(await keysNow()).length > 0 && JSON.stringify(JSON.parse((await app.evaluate(`localStorage.getItem('ccm.workspace.railOrder')`)) ?? '[]').slice(0, 3)) === JSON.stringify(want))
}

async function quit() {
  for (const p of JSON.parse(await app.evaluate(`JSON.stringify(${W}.panes.map((p) => p.key))`))) await app.evaluate(`${W}.remove(${JSON.stringify(p)})`)
  await app.sleep(1500)
  app.cmd('Browser.close').catch(() => undefined)
  for (let i = 0; i < 40 && app.child.exitCode === null; i++) await app.sleep(250)
  app.stop()
}

;(async () => {
  const exe = process.argv[2] && !process.argv[2].startsWith('--') ? { exe: process.argv[2] } : {}
  const only = process.argv.find((a) => a.startsWith('--ui='))?.slice(5)
  for (const ui of only ? [only] : ['classic', 'next']) {
    try {
      app = await launch({ ...exe, ui })
      const want = await phaseOne(ui)
      await quit()
      await new Promise((r) => setTimeout(r, 1500))
      app = await launch({ ...exe, ui, keep: true })
      await phaseTwo(ui, want)
    } catch (e) {
      await check(ui, 'run', false, e.message)
    } finally {
      app?.stop()
      app = null
      await new Promise((r) => setTimeout(r, 1000))
    }
  }
  fs.writeFileSync(path.join(OUT, 'results-rail-reorder.txt'), results.join('\n'))
  try {
    fs.rmSync(BASE, { recursive: true, force: true })
  } catch {}
  process.exit(0)
})()
