// Click bar under an agent pane: a menu screen is printed by a fixture script in a shell pane marked as Claude / Codex,
// the bar must list its options and a click must send the right keys (read back from the file the script writes).
const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const FIX = path.join(REPO, 'apps/web/src/lib/__fixtures__/term-menu')
const SCRIPT = path.join(__dirname, 'fixtures/menu-screen.cjs').replace(/\\/g, '/')
const KEYS = path.join(OUT, 'menu-keys.txt').replace(/\\/g, '/')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v100/WP-0b')
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
  const bar = () => evaluate(`(() => { const b = document.querySelector('[data-term-menu-bar]'); return b ? [...b.querySelectorAll('[data-term-menu-item]')].map((x) => x.innerText.replace(/\\s+/g, ' ').trim()) : null })()`)
  const enter = async (text) => {
    await evaluate(`document.querySelector('.xterm-helper-textarea').focus()`)
    await type(text)
    await sleep(300)
    await key('Enter', 'Enter', 13)
  }
  const show = async (name) => {
    await enter(`node ${SCRIPT} ${path.join(FIX, name + '.txt').replace(/\\/g, '/')} ${KEYS}`)
    await waitFor(`!!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar for ' + name)
    await sleep(300)
  }
  const leave = async () => {
    await evaluate(`document.querySelector('.xterm-helper-textarea').focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await waitFor(`!document.querySelector('[data-term-menu-bar]')`, 8000, 'bar gone after exit').catch(() => {})
    await sleep(500)
  }
  const click = async (index) => {
    const r = await evaluate(`(() => { const b = document.querySelector('[data-term-menu-item][data-index="${index}"]'); const r = b.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`)
    await cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r.x, y: r.y })
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x: r.x, y: r.y, button: 'left', clickCount: 1 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x: r.x, y: r.y, button: 'left', clickCount: 1 })
    await sleep(600)
  }
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
    const single = await bar()
    check('bar lists the 5 Claude options', Array.isArray(single) && single.length === 5, JSON.stringify(single))
    check('numbers and labels shown', !!single && /3.*Blue/.test(single[2]), JSON.stringify(single))
    await keep('01-claude-ask-single')
    await click(3)
    check('click Blue sends "3"', JSON.stringify(sent()) === JSON.stringify(['3']), JSON.stringify(sent()))
    const focused = await evaluate(`document.activeElement?.classList.contains('xterm-helper-textarea')`)
    check('terminal keeps focus after a click', focused === true)
    await leave()
    check('bar hidden after the screen leaves the menu', (await bar()) === null)

    await show('claude-ask-single')
    await click(4)
    const other = sent()
    check('Other moves the highlight only (3 x Down, no Enter)', other.length === 1 && other[0] === DOWN.repeat(3), JSON.stringify(other))
    const focusedOther = await evaluate(`document.activeElement?.classList.contains('xterm-helper-textarea')`)
    check('terminal focused for typing after Other', focusedOther === true)
    await leave()

    await show('claude-ask-multi')
    await keep('02-claude-ask-multi')
    await click(2)
    check('multiSelect click ticks with the number', JSON.stringify(sent()) === JSON.stringify(['2']), JSON.stringify(sent()))
    const items = await bar()
    const submitIndex = await evaluate(`[...document.querySelectorAll('[data-term-menu-item]')].find((b) => /Submit/.test(b.innerText))?.dataset.index`)
    check('Submit button present', !!submitIndex, JSON.stringify(items))
    await click(Number(submitIndex))
    await sleep(900)
    const afterSubmit = sent()
    check('Submit goes Down and never presses Enter until the screen shows it highlighted', afterSubmit.slice(1).every((k) => k !== '\r') && afterSubmit.slice(1).some((k) => k.startsWith(DOWN)), JSON.stringify(afterSubmit))
    await leave()

    await show('claude-trust-nonum')
    await keep('03-claude-trust-nonum')
    const count = (await bar()).length
    await click(count)
    const trust = sent()
    check('unnumbered menu: Down x n then Enter', trust.length === 1 && trust[0].endsWith('\r') && trust[0].startsWith(DOWN), JSON.stringify(trust))
    await leave()

    await agent('codex')
    await show('codex-permissions')
    await keep('04-codex-permissions')
    await click(2)
    check('Codex numbered menu sends the number', JSON.stringify(sent()) === JSON.stringify(['2']), JSON.stringify(sent()))
    await leave()

    await agent('claude')
    await show('claude-ask-single')
    await evaluate(`${S}.get('settings').clickMenu = false`)
    await sleep(500)
    check('switch off hides the bar', (await bar()) === null)
    await evaluate(`${S}.get('settings').clickMenu = true`)
    await sleep(500)
    check('switch on shows it again', Array.isArray(await bar()))
    await evaluate(`${W}.update(${JSON.stringify(pane)}, { agent: undefined })`)
    await sleep(500)
    check('plain shell pane has no bar', (await bar()) === null)
    await leave()
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
