const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const SHOTS = path.join(REPO, 'docs/qa/v114/WP-114a')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-v114-114a-${process.pid}`)
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
  const { evaluate, waitFor, sleep, cmd, key, shot } = app
  const move = (x, y) => cmd('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 })
  const centerOf = (sel) => evaluate(`(() => { const r = document.querySelector(${JSON.stringify(sel)})?.getBoundingClientRect(); return r ? { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) } : null })()`)
  const snap = async (name) => {
    await cmd('Page.bringToFront')
    await sleep(250)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const peeks = () => evaluate(`document.querySelectorAll('[data-rail-peek]').length`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.next.railCollapsed'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui') && document.documentElement.dataset.ui === 'next'`, 30000, 'reloaded next')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`[...document.querySelectorAll('[data-launch-row]')].at(-1).click()`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await evaluate(`${W}.split(${W}.panes[0].key, 'row')`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'two shells')
    await sleep(800)

    check('no popover mounted before hovering', (await peeks()) === 0)
    const row = await centerOf('[data-live-rail] [data-side-row]')
    await move(row.x, row.y)
    await sleep(250)
    check('no popover before the 400 ms dwell', (await peeks()) === 0)
    await sleep(350)
    check('popover opens after the dwell', (await peeks()) === 1)
    check('popover sits to the right of the rail row', await evaluate(`(() => { const p = document.querySelector('[data-rail-peek]').getBoundingClientRect(); const r = document.querySelector('[data-live-rail] [data-side-row]').getBoundingClientRect(); return p.left >= r.right })()`))
    check('row native title is dropped while open', await evaluate(`!document.querySelector('[data-live-rail] [data-side-row] [data-rail-open]').hasAttribute('title')`))
    check('shell pane shows name, kind and cwd, no reply control', await evaluate(`(() => { const p = document.querySelector('[data-rail-peek]'); return !!p.querySelector('[data-peek-title]') && !!p.querySelector('[data-peek-kind]') && !p.querySelector('input, textarea') })()`))
    check('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))
    await snap('01-peek-shell')

    await key('Escape', 'Escape', 27)
    await sleep(300)
    check('Esc closes the popover', (await peeks()) === 0)
    check('row title is restored after close', await evaluate(`document.querySelector('[data-live-rail] [data-side-row] [data-rail-open]').hasAttribute('title')`))

    await move(row.x, row.y)
    await sleep(500)
    check('popover reopens on a new hover', (await peeks()) === 1)
    await move(900, 700)
    await sleep(500)
    check('leaving the row and the popover closes it', (await peeks()) === 0)

    await evaluate(`document.querySelector('[data-live-rail] [data-rail-open]').focus()`)
    await key('i', 'KeyI', 73)
    await sleep(300)
    check('key i opens the popover and links aria-describedby', await evaluate(`(() => { const p = document.querySelector('[data-rail-peek]'); return !!p && document.querySelector('[data-live-rail] [data-rail-open]').getAttribute('aria-describedby') === p.id })()`))
    await key('i', 'KeyI', 73)
    await sleep(300)
    check('key i again closes it', (await peeks()) === 0)

    await evaluate(`window.__longTasks = []; new PerformanceObserver((l) => window.__longTasks.push(...l.getEntries().map((e) => e.duration))).observe({ entryTypes: ['longtask'] })`)
    const rows = await evaluate(`[...document.querySelectorAll('[data-live-rail] [data-side-row]')].map((el) => { const r = el.getBoundingClientRect(); return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) } })`)
    let maxPeeks = 0
    for (let round = 0; round < 8; round++) {
      for (const r of rows) {
        await move(r.x, r.y)
        await sleep(30)
        maxPeeks = Math.max(maxPeeks, await peeks())
      }
    }
    const longTasks = await evaluate(`window.__longTasks.filter((d) => d > 50).length`)
    check('sweeping the rows causes 0 long tasks over 50 ms', longTasks === 0, `${longTasks}`)
    check('at most one popover is mounted while sweeping', maxPeeks <= 1, `${maxPeeks}`)

    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)
    const mono = await centerOf('[data-rail-collapsed] [data-rail-project]')
    if (mono) {
      await evaluate(`document.querySelector('[data-rail-collapsed] [data-rail-project]').click()`)
      await waitFor(`!!document.querySelector('[data-rail-flyout] [data-side-row]')`, 3000, 'flyout')
      const fr = await centerOf('[data-rail-flyout] [data-side-row]')
      await move(fr.x, fr.y)
      await sleep(600)
      check('popover opens from a flyout row in the collapsed rail', (await peeks()) === 1)
      await snap('02-peek-collapsed')
      await key('Escape', 'Escape', 27)
      await sleep(300)
    } else check('collapsed rail has a project button', false)
    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(500)
  } catch (e) {
    check('run', false, e.message)
    await shot('v114-114a-99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v114-114a.txt'), results.join('\n'))
    process.exit(0)
  }
})()
