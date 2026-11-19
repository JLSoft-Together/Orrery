const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const SHOTS = path.join(REPO, 'docs/qa/v103/WP-70b')
const CTRL = 2
const SHIFT = 8
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const app = await launch({ ui: 'next' })
  const { evaluate, waitFor, sleep, cmd, key } = app
  const snap = async (name) => {
    await cmd('Page.bringToFront')
    await sleep(300)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const mouseAt = async (x, y) => {
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 })
  }
  const menuOpen = () => evaluate(`!!document.querySelector('[data-glass-menu]')`)
  const openMore = async () => {
    await evaluate(`document.querySelector('[data-more-views]').click()`)
    await waitFor(`!!document.querySelector('[data-glass-menu]')`, 5000, 'more menu')
  }
  const gone = async (label) => {
    await waitFor(`!document.querySelector('[data-glass-menu]')`, 3000, label).then(() => true, () => false)
    return !(await menuOpen())
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui') && document.documentElement.dataset.ui === 'next'`, 30000, 'reloaded next')
    await sleep(800)

    await openMore()
    await snap('01-more-open')
    check('more menu opens', await menuOpen())
    const canvas = JSON.parse(await evaluate(`JSON.stringify(document.querySelector('[data-next-canvas]').getBoundingClientRect())`))
    await mouseAt(Math.round(canvas.left + canvas.width / 2), Math.round(canvas.top + canvas.height / 2))
    check('click outside on the canvas closes the more menu', await gone('closed by click'))

    await openMore()
    await evaluate(`document.querySelector('[data-more-views]').click()`)
    check('click on the opener closes it and does not reopen it', await gone('closed by opener'))
    await sleep(400)

    await openMore()
    await key('Escape', 'Escape', 27)
    check('Esc closes the menu', await gone('closed by Esc'))
    await sleep(400)

    await openMore()
    await evaluate(`window.dispatchEvent(new Event('blur'))`)
    check('window losing focus closes the menu', await gone('closed by blur'))
    await sleep(400)

    await openMore()
    await evaluate(`document.querySelector('[data-next-search]')?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))`)
    check('pointerdown on the rail closes the menu too', await gone('closed by rail'))
    await sleep(400)

    await openMore()
    await evaluate(`document.querySelector('[data-menu-id="compact"]').click()`)
    await waitFor(`!!document.querySelector('[data-compact-bar]')`, 5000, 'compact view')
    check('compact view carries a visible way back', await evaluate(`!!document.querySelector('[data-compact-exit]') && document.querySelector('[data-compact-exit]').getBoundingClientRect().width > 0`))
    await snap('02-compact')
    await evaluate(`document.querySelector('[data-compact-exit]').click()`)
    await waitFor(`!document.querySelector('[data-compact-bar]') && !!document.querySelector('[data-next-rail]')`, 5000, 'full view back')
    check('the button returns to the full view with the rail', await evaluate(`${S}.get('ui').compact === false && !!document.querySelector('[data-next-rail]')`))
    check('the URL no longer says compact', await evaluate(`!location.search.includes('mode=compact')`))

    await evaluate(`${S}.get('ui').toggleCompact()`)
    await waitFor(`!!document.querySelector('[data-compact-bar]')`, 5000, 'compact again')
    await sleep(600)
    await key('Escape', 'Escape', 27)
    await waitFor(`!document.querySelector('[data-compact-bar]')`, 5000, 'Esc left compact')
    check('Esc leaves the compact view', await evaluate(`${S}.get('ui').compact === false`))

    await evaluate(`${S}.get('ui').toggleCompact()`)
    await waitFor(`!!document.querySelector('[data-compact-bar]')`, 5000, 'compact third')
    await sleep(600)
    await key('k', 'KeyK', 75, CTRL)
    await sleep(500)
    await cmd('Input.insertText', { text: 'Full view' })
    await sleep(500)
    await key('Enter', 'Enter', 13)
    await sleep(600)
    check('Ctrl+K "Full view" also leaves the compact view', await evaluate(`${S}.get('ui').compact === false`))
    await evaluate(`${S}.get('ui').paletteOpen = false`)

    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await sleep(400)
    await key('o', 'KeyO', 79, CTRL | SHIFT)
    await sleep(500)
    check('Ctrl+Shift+O opens nothing (Mission Control is gone)', await evaluate(`!document.querySelector('[data-mission]')`))
    await evaluate(`${S}.get('ui').paletteOpen = true`)
    await sleep(400)
    await cmd('Input.insertText', { text: 'Mission' })
    await sleep(500)
    check('Ctrl+K has no Mission Control entry', await evaluate(`!/Mission Control/i.test(document.body.innerText)`))
    await evaluate(`${S}.get('ui').paletteOpen = false`)
    await sleep(300)

    await evaluate(`localStorage.setItem('ccm.uiGeneration', 'classic'); localStorage.removeItem('ccm.uiMigration')`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reload after reset')
    check('a machine on classic with no migration key boots next', await evaluate(`document.documentElement.dataset.ui === 'next' && localStorage.getItem('ccm.uiGeneration') === 'next' && localStorage.getItem('ccm.uiMigration') === '4.1-next'`))
    await evaluate(`${S}.get('uiGen').setGeneration('classic')`)
    await sleep(300)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reload classic')
    check('a later classic choice sticks across a reload', await evaluate(`document.documentElement.dataset.ui === 'classic' && localStorage.getItem('ccm.uiGeneration') === 'classic'`))
    await snap('03-classic-after-choice')
    await evaluate(`${S}.get('uiGen').setGeneration('next')`)
    await sleep(300)
    check('Appearance can still switch back to next', await evaluate(`document.documentElement.dataset.ui === 'next'`))
    await snap('04-next')
  } catch (err) {
    check('script error', false, String(err?.message ?? err))
    await snap('99-error').catch(() => {})
  } finally {
    await app.stop()
    const fail = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fail} PASS, ${fail} FAIL`)
    process.exit(fail ? 1 : 0)
  }
})()
