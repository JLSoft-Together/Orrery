const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const FIX = path.join(REPO, 'apps/web/src/lib/__fixtures__/term-menu')
const SCRIPT = path.join(__dirname, 'fixtures/menu-screen-clear.cjs').replace(/\\/g, '/')
const KEYS_A = path.join(OUT, 'queue-keys-a.txt').replace(/\\/g, '/')
const KEYS_B = path.join(OUT, 'queue-keys-b.txt').replace(/\\/g, '/')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v100/WP-0d')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CTRL = 2, ALT = 1, SHIFT = 8

;(async () => {
  const app = await launch()
  const { evaluate, waitFor, shot, type, key, sleep, cmd } = app
  fs.mkdirSync(SHOTS, { recursive: true })
  const keep = async (name) => {
    await shot(name)
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const sent = (file) => (fs.existsSync(file) ? fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)) : [])
  const chipText = () => evaluate(`document.querySelector('[data-menu-queue-chip]')?.innerText.replace(/\\s+/g, ' ').trim() ?? null`)
  const activeKey = () => evaluate(`${W}.activeKey`)
  const enter = async (text) => {
    await evaluate(`[...document.querySelectorAll('.xterm-helper-textarea')].find((t) => t.offsetParent !== null)?.focus()`)
    await type(text)
    await sleep(300)
    await key('Enter', 'Enter', 13)
  }
  const click = async (selector) => {
    const r = await evaluate(`(() => { const b = document.querySelector(${JSON.stringify(selector)}); if (!b) return null; const r = b.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`)
    if (!r) throw new Error('no element ' + selector)
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r.x, y: r.y })
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: r.x, y: r.y, button: 'left', clickCount: 1 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: r.x, y: r.y, button: 'left', clickCount: 1 })
    await sleep(500)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell A running')
    const a = await evaluate(`${W}.panes[0].key`)
    await sleep(2000)
    await evaluate(`${W}.update(${JSON.stringify(a)}, { agent: 'claude' })`)
    await app.cmd('Page.bringToFront')
    await enter(`node ${SCRIPT} ${path.join(FIX, 'claude-ask-single.txt').replace(/\\/g, '/')} ${KEYS_A}`)
    await waitFor(`!!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar A')
    await sleep(800)

    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes[1].state === 'running'`, 20000, 'shell B running')
    const b = await evaluate(`${W}.panes[1].key`)
    await sleep(2000)
    await evaluate(`${W}.update(${JSON.stringify(b)}, { agent: 'claude' })`)
    await enter(`node ${SCRIPT} ${path.join(FIX, 'claude-permission.txt').replace(/\\/g, '/')} ${KEYS_B}`)
    await waitFor(`${W}.activeKey === ${JSON.stringify(b)} && !!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar B')
    check('two panes in two tabs', (await evaluate(`${W}.tabs.length`)) === 2)

    await evaluate(`${W}.update(${JSON.stringify(a)}, { agent: 'claude' })`); await waitFor(`/2/.test(document.querySelector('[data-menu-queue-chip]')?.innerText ?? '')`, 8000, 'chip 2')
    check('chip counts both panes (one of them hidden)', /2/.test(await chipText()), await chipText())

    await click('[data-menu-queue-chip]')
    const rows = await evaluate(`[...document.querySelectorAll('[data-asked]')].map((r) => ({ key: r.dataset.asked, text: r.innerText.replace(/\\s+/g, ' ').trim() }))`)
    const order = await evaluate(`${S}.get('menuQueue').entries.map((e) => e.paneKey)`)
    const sinces = await evaluate(`${S}.get('menuQueue').entries.map((e) => e.since)`)
    check('popover lists both panes in the order of the queue (oldest first)', rows.length === 2 && JSON.stringify(rows.map((r) => r.key)) === JSON.stringify(order) && sinces[0] <= sinces[1] && new Set(order).size === 2 && order.includes(a) && order.includes(b), JSON.stringify({ rows, order, sinces }))
    check('rows show the question', rows.some((r) => /Do you want to proceed\?/.test(r.text)) && rows.some((r) => /m.u\?/.test(r.text.normalize('NFC'))), JSON.stringify(rows))
    await keep('01-queue-popover')
    await click(`[data-asked="${a}"]`)
    check('click a row jumps to that pane', (await activeKey()) === a, await activeKey())
    check('popover closes after the jump', !(await evaluate(`!!document.querySelector('[data-menu-queue-popover]')`)))

    await key('q', 'KeyQ', 81, CTRL | ALT)
    await sleep(500)
    check('Ctrl+Alt+Q jumps to the next waiting pane', (await activeKey()) === b, await activeKey())
    await key('q', 'KeyQ', 81, CTRL | ALT)
    await sleep(500)
    check('Ctrl+Alt+Q again wraps around', (await activeKey()) === a, await activeKey())

    await evaluate(`${W}.activate(${JSON.stringify(b)})`)
    await waitFor(`${W}.activeKey === ${JSON.stringify(b)} && !!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar B active')
    check('no Mission Control is left to answer from', !(await evaluate(`!!document.querySelector('[data-mission]')`)))
    await click('[data-term-menu-bar] [data-term-menu-item][data-index="2"]')
    await sleep(800)
    check('answering from the pane bar sends the key to that pane', JSON.stringify(sent(KEYS_B)) === JSON.stringify(['2']), JSON.stringify(sent(KEYS_B)))
    check('the other pane got nothing', sent(KEYS_A).length === 0, JSON.stringify(sent(KEYS_A)))
    await waitFor(`/1/.test(document.querySelector('[data-menu-queue-chip]')?.innerText ?? '')`, 8000, 'chip 1')
    check('chip drops to 1', /1/.test(await chipText()), await chipText())
    await keep('03-after-answer')

    await key('q', 'KeyQ', 81, CTRL | ALT)
    await waitFor(`${W}.activeKey === ${JSON.stringify(a)} && !!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar A active')
    await click('[data-term-menu-bar] [data-term-menu-item][data-index="3"]')
    await sleep(800)
    check('second answer goes to the other pane', JSON.stringify(sent(KEYS_A)) === JSON.stringify(['3']), JSON.stringify(sent(KEYS_A)))
    await waitFor(`!document.querySelector('[data-menu-queue-chip]')`, 8000, 'chip gone')
    check('chip disappears when nothing waits', true)
    await key('Escape', 'Escape', 27)
    await sleep(400)

    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length === 3 && ${W}.panes[2].state === 'running'`, 20000, 'shell C running')
    const c = await evaluate(`${W}.panes[2].key`)
    await sleep(2000)
    await evaluate(`${W}.update(${JSON.stringify(c)}, { agent: 'claude' })`)
    await enter(`node ${SCRIPT} ${path.join(FIX, 'claude-ask-single.txt').replace(/\\/g, '/')} ${KEYS_A}`)
    await waitFor(`/1/.test(document.querySelector('[data-menu-queue-chip]')?.innerText ?? '')`, 8000, 'chip for C')
    await evaluate(`${W}.remove(${JSON.stringify(c)})`)
    await waitFor(`!document.querySelector('[data-menu-queue-chip]')`, 8000, 'chip gone after the pane closed')
    check('closing a pane that waits removes it from the queue', true)
  } catch (e) {
    check('run', false, e.message)
    await shot('99-error').catch(() => {})
  } finally {
    await evaluate(`(() => { const w = ${W}; [...w.panes].forEach((p) => w.remove(p.key)) })()`).catch(() => {})
    await sleep(1500)
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
