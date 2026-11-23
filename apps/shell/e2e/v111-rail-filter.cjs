const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const SHOTS = path.join(REPO, 'docs/qa/v111/WP-112f')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-v111-112f-${process.pid}`)
  fs.rmSync(root, { recursive: true, force: true })
  const child = spawn(process.execPath, [path.join(REPO, 'scripts/demo-fixture.mjs'), `--root=${root}`, '--sessions=4', '--subagents=2'], { stdio: ['ignore', 'pipe', 'ignore'], windowsHide: true })
  await new Promise((resolve) => {
    let b = ''
    child.stdout.on('data', (d) => { b += d; if (/fixture root/.test(b)) resolve() })
    setTimeout(resolve, 8000)
  })
  return { root, child }
}

;(async () => {
  fs.mkdirSync(SHOTS, { recursive: true })
  const demo = await fixture()
  const app = await launch({ ui: 'next', env: { CLAUDE_CONFIG_DIR: demo.root, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const snap = async (name) => {
    await cmd('Page.bringToFront')
    await sleep(250)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
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
  const type = async (text) => cmd('Input.insertText', { text })
  const rows = () => evaluate(`document.querySelectorAll('[data-rail-list] [data-side-row]').length`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.next.railCollapsed'); localStorage.removeItem('ccm.next.railGroup'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui') && document.documentElement.dataset.ui === 'next'`, 30000, 'reloaded next')
    await size(1600, 900)
    await sleep(800)
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`[...document.querySelectorAll('[data-launch-row]')].at(-1).click()`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await evaluate(`${W}.split(${W}.panes[0].key, 'row')`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'two shells')
    await sleep(800)

    check('filter input and group button share the first row', await evaluate(`(() => { const i = document.querySelector('[data-live-filter]')?.getBoundingClientRect(); const g = document.querySelector('[data-live-group]')?.getBoundingClientRect(); return !!i && !!g && Math.abs(i.top + i.height / 2 - (g.top + g.height / 2)) < 6 })()`))
    check('segmented group row is gone', await evaluate(`!document.querySelector('[data-live-group][role=radiogroup]') && document.querySelectorAll('[data-live-group]').length === 1`))
    check('filter input has a label', await evaluate(`!!document.querySelector('[data-live-filter]').getAttribute('aria-label')`))
    const before = await rows()
    check('rail lists the two shells', before >= 2, String(before))
    await snap('01-rail-open')

    await evaluate(`document.querySelector('[data-live-filter]').focus()`)
    await type('SHEL')
    await sleep(300)
    check('typing filters in place (still matches both shells)', (await rows()) === before)
    await type('lzzq')
    await sleep(300)
    check('no match shows the message and the palette hint', await evaluate(`!!document.querySelector('[data-rail-no-match]') && !!document.querySelector('[data-rail-no-match-all]') && document.querySelectorAll('[data-rail-list] [data-side-row]').length === 0`))
    await snap('02-rail-no-match')
    await key('Escape', 'Escape', 27)
    await sleep(300)
    check('Esc clears the filter and brings the list back', await evaluate(`document.querySelector('[data-live-filter]').value === '' && document.querySelectorAll('[data-rail-list] [data-side-row]').length === ${before}`))

    await evaluate(`document.querySelector('[data-live-filter]').focus()`)
    await type('xyz')
    await sleep(300)
    await evaluate(`document.querySelector('[data-live-filter-clear]').click()`)
    await sleep(300)
    check('clear button empties the filter and keeps focus in it', await evaluate(`document.querySelector('[data-live-filter]').value === '' && document.activeElement === document.querySelector('[data-live-filter]')`))

    await key('ArrowDown', 'ArrowDown', 40)
    await sleep(200)
    check('ArrowDown from the filter moves into the list', await evaluate(`!!document.activeElement?.closest('[data-rail-list]')`))
    await key('f', 'KeyF', 70, 2)
    await sleep(250)
    check('Ctrl+F inside the rail focuses the filter', await evaluate(`document.activeElement === document.querySelector('[data-live-filter]')`))

    await evaluate(`document.querySelector('[data-live-group]').click()`)
    await waitFor(`!!document.querySelector('[data-glass-menu]')`, 3000, 'group menu')
    check('group menu offers Project and Status with the current one checked', await evaluate(`document.querySelector('[data-menu-id="project"]').getAttribute('aria-checked') === 'true' && document.querySelector('[data-menu-id="status"]').getAttribute('aria-checked') === 'false'`))
    await snap('03-group-menu')
    await evaluate(`document.querySelector('[data-menu-id="status"]').click()`)
    await sleep(400)
    check('picking Status groups by status and remembers it', await evaluate(`!!document.querySelector('[data-rail-list] [data-status]') && localStorage.getItem('ccm.next.railGroup') === 'status'`))
    await evaluate(`document.querySelector('[data-live-filter]').focus()`)
    await type('shel')
    await sleep(300)
    check('filter keeps the status grouping', await evaluate(`!!document.querySelector('[data-rail-list] [data-status]') && document.querySelectorAll('[data-rail-list] [data-side-row]').length >= 2`))
    await key('Escape', 'Escape', 27)
    await evaluate(`document.querySelector('[data-live-group]').click()`)
    await waitFor(`!!document.querySelector('[data-glass-menu]')`, 3000, 'group menu')
    await evaluate(`document.querySelector('[data-menu-id="project"]').click()`)
    await sleep(300)

    check('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))

    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)
    check('collapsed rail has the magnifier at its head', await evaluate(`!!document.querySelector('[data-rail-search-open]') && !document.querySelector('[data-live-filter]')`))
    await evaluate(`document.querySelector('[data-rail-search-open]').click()`)
    await waitFor(`!!document.querySelector('[data-rail-search-pop]')`, 3000, 'popover')
    await sleep(300)
    check('popover focuses its input and lists sessions without widening the rail', await evaluate(`document.activeElement === document.querySelector('[data-rail-search-input]') && document.querySelectorAll('[data-rail-search-results] [data-side-row]').length >= 2 && Math.abs(document.querySelector('[data-next-rail]').getBoundingClientRect().width - 56) <= 1`))
    await snap('04-rail-collapsed-popover')
    await type('zzqq')
    await sleep(300)
    check('popover shows the empty message', await evaluate(`!!document.querySelector('[data-rail-search-empty]')`))
    await key('Escape', 'Escape', 27)
    await sleep(200)
    check('first Esc clears the popover filter', await evaluate(`document.querySelector('[data-rail-search-input]').value === '' && !!document.querySelector('[data-rail-search-pop]')`))
    await type('shel')
    await sleep(300)
    await evaluate(`document.querySelector('[data-rail-search-results] [data-rail-open]').click()`)
    await sleep(400)
    check('picking a result closes the popover and keeps the rail collapsed', await evaluate(`!document.querySelector('[data-rail-search-pop]') && Math.abs(document.querySelector('[data-next-rail]').getBoundingClientRect().width - 56) <= 1`))
    await snap('05-rail-collapsed')
    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(500)
  } catch (e) {
    check('run', false, e.message)
    await shot('v111-112f-99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v111-112f.txt'), results.join('\n'))
    process.exit(0)
  }
})()
