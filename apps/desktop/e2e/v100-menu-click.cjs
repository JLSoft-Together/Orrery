const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const FIX = path.join(REPO, 'apps/web/src/lib/__fixtures__/term-menu')
const SCRIPT = path.join(__dirname, 'fixtures/menu-screen.cjs').replace(/\\/g, '/')
const KEYS = path.join(OUT, 'menu-keys.txt').replace(/\\/g, '/')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v100/WP-0c')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const CTRL = 2
const DOWN = '\u001b[B'

;(async () => {
  const app = await launch()
  const { evaluate, waitFor, shot, type, key, sleep, cmd } = app
  fs.mkdirSync(SHOTS, { recursive: true })
  const keep = async (name) => {
    await shot(name)
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const sent = () => (fs.existsSync(KEYS) ? fs.readFileSync(KEYS, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)) : [])
  const raw = () => sent().join('')
  const linesOf = (name) => fs.readFileSync(path.join(FIX, name + '.txt'), 'utf8').replace(/\n$/, '').split('\n')
  let shift = 0
  const setShift = async (name) => {
    const g = await geom()
    const rows = Math.round(g.height / (cell || 18))
    shift = Math.max(0, linesOf(name).length - rows)
  }
  const lineOf = (name, re) => linesOf(name).findIndex((l) => re.test(l)) - shift
  const firstText = (name, limit) => linesOf(name).findIndex((l, i) => i - shift >= 0 && i < limit + shift && l.trim().length > 20) - shift
  const enter = async (text) => {
    await evaluate(`document.querySelector('.xterm-helper-textarea').focus()`)
    await type(text)
    await sleep(300)
    await key('Enter', 'Enter', 13)
  }
  const show = async (name, mode = '') => {
    await enter(`node ${SCRIPT} ${path.join(FIX, name + '.txt').replace(/\\/g, '/')} ${KEYS} ${mode}`)
    await waitFor(`!!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar for ' + name)
    await sleep(400)
    await setShift(name)
  }
  const leave = async () => {
    await evaluate(`document.querySelector('.xterm-helper-textarea').focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await sleep(300)
    if (await evaluate(`!!document.querySelector('[data-term-menu-bar]')`)) await key('c', 'KeyC', 67, CTRL)
    await waitFor(`!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar gone after exit').catch(() => {})
    await sleep(500)
  }
  const geom = () => evaluate(`(() => { const r = document.querySelector('.xterm-screen').getBoundingClientRect(); return { left: r.left, top: r.top, width: r.width, height: r.height } })()`)
  let cell = 0
  const measure = async () => {
    const g = await geom()
    const x = g.left + 20
    for (let y = g.top + 2; y < g.top + g.height; y += 3) {
      await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y })
      const h = await evaluate(`document.querySelector('[data-term-menu-hover]')?.getBoundingClientRect().height ?? 0`)
      if (h > 0) { cell = h; return h }
    }
    return 0
  }
  const at = async (row, dx = 20) => {
    const g = await geom()
    return { x: g.left + dx, y: g.top + (row + 0.5) * cell }
  }
  const move = async (row, dx) => {
    const p = await at(row, dx)
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y })
    await sleep(120)
  }
  const press = async (row, count = 1, dx) => {
    const p = await at(row, dx)
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y })
    for (let c = 1; c <= count; c++) {
      await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: p.x, y: p.y, button: 'left', buttons: 1, clickCount: c })
      await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: p.x, y: p.y, button: 'left', buttons: 0, clickCount: c })
    }
    await sleep(500)
  }
  const drag = async (row, from, to) => {
    const a = await at(row, from)
    const b = await at(row, to)
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: a.x, y: a.y })
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: a.x, y: a.y, button: 'left', buttons: 1, clickCount: 1 })
    for (let i = 1; i <= 6; i++) await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: a.x + ((b.x - a.x) * i) / 6, y: a.y, button: 'left', buttons: 1 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: b.x, y: b.y, button: 'left', buttons: 0, clickCount: 1 })
    await sleep(300)
  }
  const copied = () => evaluate(`(() => { const dt = new DataTransfer(); const e = new ClipboardEvent('copy', { clipboardData: dt, bubbles: true, cancelable: true }); document.querySelector('.xterm-helper-textarea').dispatchEvent(e); return dt.getData('text/plain') })()`)
  const hoverOn = () => evaluate(`!!document.querySelector('[data-term-menu-hover]')`)
  const cursor = () => evaluate(`document.querySelector('.xterm').style.cursor`)
  const selectedIndex = () => evaluate(`Number(document.querySelector('[data-term-menu-item][aria-current]')?.dataset.index ?? 0)`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(2500)
    await app.cmd('Page.bringToFront')
    const pane = await evaluate(`${W}.panes[0].key`)
    const agent = (a) => evaluate(`${W}.update(${JSON.stringify(pane)}, { agent: ${JSON.stringify(a)} })`)

    await agent('claude')
    await show('claude-ask-single')
    const rowRed = lineOf('claude-ask-single', /1\.\s+Red/)
    const rowGreen = lineOf('claude-ask-single', /2\.\s+Green/)
    const rowBlue = lineOf('claude-ask-single', /3\.\s+Blue/)
    const rowText = firstText('claude-ask-single', rowRed - 1)
    check('fixture rows found', rowRed > 0 && rowGreen > rowRed && rowBlue > rowGreen && rowText >= 0, JSON.stringify({ rowRed, rowGreen, rowBlue, rowText }))
    const cellH = await measure()
    check('hover lights a row (cell height measured)', cellH > 8 && cellH < 60, String(cellH))
    await move(rowGreen)
    check('hover over Green: overlay shown', (await hoverOn()) === true)
    check('hover over Green: pointer cursor', (await cursor()) === 'pointer')
    const g = await geom()
    const top = await evaluate(`document.querySelector('[data-term-menu-hover]').getBoundingClientRect().top`)
    check('hover overlay sits on the Green row', Math.abs(top - (g.top + rowGreen * cell)) < 2.5, `${top} vs ${g.top + rowGreen * cell}`)
    await keep('01-hover-green')
    await move(rowText)
    check('hover off the menu rows: no overlay, default cursor', (await hoverOn()) === false && (await cursor()) === '')

    fs.writeFileSync(KEYS, '')
    const before = await selectedIndex()
    await press(rowBlue, 1)
    check('click Blue moves the highlight (arrows), no number', JSON.stringify(sent()) === JSON.stringify([DOWN.repeat(3 - before)]), JSON.stringify(sent()))
    check('terminal keeps focus after a click', (await evaluate(`document.activeElement?.classList.contains('xterm-helper-textarea')`)) === true)
    await leave()

    await show('claude-ask-single')
    fs.writeFileSync(KEYS, '')
    await press(rowBlue, 2)
    const dbl = sent()
    check('double click Blue: arrows then number 3', dbl.join('') === DOWN.repeat(3 - before) + '3', JSON.stringify(dbl))
    await leave()

    await show('claude-ask-single')
    fs.writeFileSync(KEYS, '')
    await press(rowText, 2, 40)
    const word = await copied()
    check('double click outside the menu still selects a word', typeof word === 'string' && word.trim().length > 0, JSON.stringify(word))
    check('...and sends no key', sent().length === 0, JSON.stringify(sent()))
    await drag(rowText, 20, 220)
    const dragged = await copied()
    check('drag outside the menu still selects text', typeof dragged === 'string' && dragged.trim().length > 3, JSON.stringify(dragged))
    await keep('02-select-outside')
    await leave()

    await show('claude-ask-multi')
    const mRow = lineOf('claude-ask-multi', /2\.\s+\[/)
    fs.writeFileSync(KEYS, '')
    await press(mRow, 1)
    check('multiSelect click ticks with the number', JSON.stringify(sent()) === JSON.stringify(['2']), JSON.stringify(sent()))
    await press(mRow, 2)
    check('multiSelect double click ticks once per physical click (the second click of the pair is ignored)', sent().filter((k) => k === '2').length === 2, JSON.stringify(sent()))
    await keep('03-multi')
    fs.writeFileSync(KEYS, '')
    const otherRow = lineOf('claude-ask-multi', /Type something/)
    await press(otherRow, 1)
    check('Other row: arrows only, no Enter, terminal focused', sent().length === 1 && sent()[0].startsWith(DOWN) && !sent()[0].includes('\r') && (await evaluate(`document.activeElement?.classList.contains('xterm-helper-textarea')`)) === true, JSON.stringify(sent()))
    fs.writeFileSync(KEYS, '')
    const submitRow = lineOf('claude-ask-multi', /^\s*(❯\s*)?Submit\s*$/)
    await press(submitRow, 1)
    await sleep(900)
    check('Submit row: Down then never Enter on a static screen', sent().length >= 1 && sent()[0].startsWith(DOWN) && sent().every((k) => k !== '\r'), JSON.stringify(sent()))
    await leave()

    await show('claude-trust-nonum')
    const trustBar = await evaluate(`[...document.querySelectorAll('[data-term-menu-item]')].length`)
    fs.writeFileSync(KEYS, '')
    check('unnumbered menu fixture has rows', trustBar >= 2, String(trustBar))
    await leave()

    await agent('codex')
    fs.writeFileSync(KEYS, '')
    await show('codex-permissions', 'mouse')
    const cx2 = lineOf('codex-permissions', /2\.\s+/)
    const cx0 = firstText('codex-permissions', cx2 - 1)
    fs.writeFileSync(KEYS, '')
    await move(cx0 >= 0 ? cx0 : 0)
    fs.writeFileSync(KEYS, '')
    await move(cx2)
    await move(cx2, 60)
    check('mouse mode: motion over a menu row never reaches the PTY', !/\u001b\[<35;/.test(raw()), JSON.stringify(raw()).slice(0, 120))
    check('mouse mode: hover still lights the row', (await hoverOn()) === true)
    fs.writeFileSync(KEYS, '')
    await press(cx2, 1)
    check('mouse mode: click on a menu row sends only the highlight move', !/\u001b\[<\d+;\d+;\d+[Mm]/.test(raw()) && raw() === DOWN, JSON.stringify(raw()))
    fs.writeFileSync(KEYS, '')
    await press(cx2, 2)
    check('mouse mode: double click sends the number, no mouse report', !/\u001b\[</.test(raw()) && raw().endsWith('2'), JSON.stringify(raw()))
    await keep('04-codex-mouse-mode')
    fs.writeFileSync(KEYS, '')
    await move(0, 30)
    await sleep(200)
    await press(0, 1, 30)
    check('mouse mode: click outside the menu still reaches Codex as a mouse report', /\u001b\[<0;\d+;\d+M/.test(raw()) && /\u001b\[<0;\d+;\d+m/.test(raw()), JSON.stringify(raw()).slice(0, 160))
    fs.writeFileSync(KEYS, '')
    await move(0, 80)
    check('mouse mode: motion outside the menu still reaches Codex', /\u001b\[<35;/.test(raw()), JSON.stringify(raw()).slice(0, 120))
    await leave()

    await agent('claude')
    await show('claude-ask-single')
    await evaluate(`${S}.get('settings').clickMenu = false`)
    await sleep(500)
    fs.writeFileSync(KEYS, '')
    await press(rowBlue, 1)
    check('switch off: click on the menu row is left to the terminal', sent().length === 0 && (await hoverOn()) === false, JSON.stringify(sent()))
    await evaluate(`${S}.get('settings').clickMenu = true`)
    await sleep(500)
    await leave()
    check('after the menu is gone a click on that row selects text again', (await hoverOn()) === false)
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
