const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const G = `${S}.get('uiGen')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-v102-demo-${process.pid}`)
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
  const demo = await fixture()
  const app = await launch({ ui: 'classic', env: { CLAUDE_CONFIG_DIR: demo.root, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const rect = async (sel) => JSON.parse(await evaluate(`JSON.stringify(document.querySelector(${JSON.stringify(sel)})?.getBoundingClientRect() ?? null)`))
  const leavesOf = () => evaluate(`JSON.stringify((function walk(n) { return 'pane' in n ? [n.pane] : [...walk(n.a), ...walk(n.b)] })(${W}.activeTab.root))`).then(JSON.parse)
  const mouse = (type, x, y, modifiers = 0) => cmd('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1, modifiers })
  async function drag(from, to, { during } = {}) {
    await mouse('mousePressed', from.x, from.y)
    for (let i = 1; i <= 10; i++) {
      await mouse('mouseMoved', from.x + ((to.x - from.x) * i) / 10, from.y + ((to.y - from.y) * i) / 10)
      await sleep(30)
    }
    await sleep(200)
    if (during) await during()
    await mouse('mouseReleased', to.x, to.y)
    await sleep(500)
  }
  const headOf = async (k) => {
    const hit = await evaluate(`(() => {
      const h = document.querySelector('[data-pane="${k}"] header'); const r = h.getBoundingClientRect(); const y = r.top + r.height / 2
      for (let x = r.left + 6; x < r.right - 6; x += 4) {
        const e = document.elementFromPoint(x, y)
        if (e && h.contains(e) && !e.closest('button, a, input, select, textarea, label, [role=button], [role=group], [data-pane-chip]')) return [x, y]
      }
      return null
    })()`)
    return { x: hit[0], y: hit[1] }
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
  const nodesKept = () => evaluate(`(() => { const els = [...document.querySelectorAll('.xterm')]; return els.length === 2 && els.every((el) => el.__wp11 > 0) })()`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await size(1600, 900)
    await sleep(800)
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`[...document.querySelectorAll('[data-launch-row]')].at(-1).click()`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await evaluate(`${W}.split(${W}.panes[0].key, 'row')`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'two shells')
    await sleep(800)
    const ids = await evaluate(`JSON.stringify(${W}.panes.map((p) => p.terminalId ?? p.ptyId ?? p.key))`)
    await evaluate(`document.querySelectorAll('.xterm').forEach((el, i) => { el.__wp11 = i + 1 })`)
    check('classic: no next shell', await evaluate(`document.documentElement.dataset.ui === 'classic' && !document.querySelector('[data-live-rail]')`))

    await evaluate(`${G}.setGeneration('next')`)
    await waitFor(`document.documentElement.dataset.ui === 'next'`, 5000, 'next')
    await sleep(800)
    check('classic -> next keeps terminal nodes (no remount)', await nodesKept())
    check('panes still running with the same ids', await evaluate(`${W}.panes.every((p) => p.state === 'running') && JSON.stringify(${W}.panes.map((p) => p.terminalId ?? p.ptyId ?? p.key)) === ${JSON.stringify(ids)}`))
    const rail = await rect('[data-live-rail]')
    check('rail visible at 1600 wide', !!rail && rail.width > 150 && rail.height > 200, JSON.stringify(rail))
    check('rail lists sessions', await evaluate(`document.querySelectorAll('[data-live-rail] [data-row-key]').length >= 2`))
    check('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))
    await shot('v102-next-01-workspace')

    await evaluate(`${G}.setGeneration('classic')`)
    await waitFor(`document.documentElement.dataset.ui === 'classic'`, 5000, 'classic')
    await sleep(800)
    check('next -> classic keeps terminal nodes', await nodesKept())
    check('classic: rail gone', await evaluate(`!document.querySelector('[data-live-rail]')`))
    await evaluate(`${G}.setGeneration('next')`)
    await waitFor(`document.documentElement.dataset.ui === 'next'`, 5000, 'next again')
    await sleep(800)

    await evaluate(`document.querySelector('.xterm-helper-textarea')?.blur()`)
    await key('E', 'KeyE', 69, 2 | 8)
    await sleep(500)
    check('Ctrl+Shift+E opens the Explorer drawer', await evaluate(`['open','pinned'].includes(document.querySelector('[data-explorer-drawer]')?.dataset.state)`), await evaluate(`document.querySelector('[data-explorer-drawer]')?.dataset.state`))
    await shot('v102-next-02-explorer')
    await key('Escape', 'Escape', 27)
    await sleep(400)
    check('Esc closes the drawer', await evaluate(`document.querySelector('[data-explorer-drawer]')?.dataset.state !== 'open'`))

    await key('k', 'KeyK', 75, 2)
    await waitFor(`!!document.querySelector('[data-palette-ui="next"]')`, 5000, 'palette')
    await sleep(400)
    const pal = JSON.parse(await evaluate(`JSON.stringify({ rows: document.querySelectorAll('[data-palette-ui] [data-palette-row="session"]').length, groups: [...document.querySelectorAll('[data-palette-ui] [role=group]')].map((g) => g.getAttribute('aria-label')) })`))
    check('Ctrl+K lists sessions in their own group', pal.rows > 0 && pal.groups.length > 0, JSON.stringify(pal))
    await shot('v102-next-03-palette')
    await key('Escape', 'Escape', 27)
    await waitFor(`!document.querySelector('[data-palette-ui]')`, 3000, 'palette closed')

    const [a, b] = await leavesOf()
    const ra = await rect(`[data-pane="${a}"]`)
    let mid = {}
    await drag(await headOf(b), { x: ra.left + ra.width / 2, y: ra.top + 50 }, {
      during: async () => {
        mid = JSON.parse(await evaluate(`JSON.stringify({ compass: !!document.querySelector('[data-drop-compass]'), tiles: document.querySelectorAll('[data-compass-zone]').length, edge: document.querySelector('[data-drop-preview]')?.dataset.edge })`))
        await shot('v102-next-04-compass')
      },
    })
    check('drop compass shows 5 tiles while dragging', mid.compass && mid.tiles === 5, JSON.stringify(mid))
    const after = await leavesOf()
    check('drop changed the layout', JSON.stringify(after) !== JSON.stringify([a, b]) || mid.edge === 'swap', JSON.stringify(after))
    await waitFor(`!!document.querySelector('[data-layout-toast]')`, 3000, 'undo toast').catch(() => null)
    check('undo toast after the drop', await evaluate(`!!document.querySelector('[data-layout-toast]') && !!document.querySelector('[data-layout-undo]')`))
    await shot('v102-next-05-toast')
    await evaluate(`document.querySelector('[data-layout-undo]')?.click()`)
    await sleep(600)
    check('Undo restores the layout', JSON.stringify(await leavesOf()) === JSON.stringify([a, b]), JSON.stringify(await leavesOf()))
    check('terminals still the same nodes after drag + undo', await nodesKept())

    await evaluate(`${S}.get('ui').settingsOpen = true`)
    await waitFor(`!!document.querySelector('[data-settings-page="appearance"]')`, 10000, 'settings')
    const nav = await evaluate(`[...document.querySelectorAll('[data-settings-page]')].map((b) => b.dataset.settingsPage).join(',')`)
    check('Settings nav has the merged Workspace and Data pages', /(^|,)workspace(,|$)/.test(nav) && /(^|,)data(,|$)/.test(nav) && !/(^|,)disk(,|$)/.test(nav), nav)
    await evaluate(`document.querySelector('[data-settings-page="appearance"]').click()`)
    await waitFor(`!!document.querySelector('[data-palette-card]')`, 5000, 'palette cards')
    const cards = await evaluate(`document.querySelectorAll('[data-palette-card]').length`)
    check('Appearance shows 10 palette cards, the last one custom', cards === 10 && (await evaluate(`[...document.querySelectorAll('[data-palette-card]')].at(-1).dataset.paletteCard`)) === 'custom', String(cards))
    await evaluate(`document.querySelector('[data-palette-card^="nebula"]').click()`)
    await sleep(500)
    check('picking a palette applies it', await evaluate(`${G}.nextTheme === 'nebula' && document.documentElement.dataset.nextTheme === 'nebula'`))
    await shot('v102-next-06-settings')
    await evaluate(`document.querySelector('[data-palette-card^="mist"]').click()`)
    await sleep(300)
    check('back to Suong kinh', await evaluate(`${G}.nextTheme === 'mist'`))
    await evaluate(`${S}.get('ui').settingsOpen = false`)

    await evaluate(`${S}.get('settings').themeFollow = false; ${G}.setNextTheme('galaxy'); ${S}.get('settings').setThemeMode('dark'); ${S}.get('settings').starsFx = 'meteors'`)
    await sleep(600)
    const fx = JSON.parse(await evaluate(`JSON.stringify({ pane: [...document.querySelectorAll('.ccm-panel, .ccm-sky-well')].map((e) => getComputedStyle(e).animationName).filter((n) => n.includes('meteor')).length, bg: getComputedStyle(document.body, '::before').animationName })`))
    check('M5: no meteor inside panels or terminal wells in next', fx.pane === 0, JSON.stringify(fx))
    check('M5: background meteors still run on Ngan ha', /meteor/.test(fx.bg), JSON.stringify(fx))
    await evaluate(`${G}.setMotion('off')`)
    await sleep(300)
    const motion = JSON.parse(await evaluate(`JSON.stringify({ attr: document.documentElement.dataset.motion, bg: getComputedStyle(document.body, '::before').animationName, dur: getComputedStyle(document.body, '::before').animationDuration })`))
    check('motion off stops loops', motion.attr === 'off' && (motion.bg === 'none' || /^0/.test(motion.dur) || parseFloat(motion.dur) < 0.01), JSON.stringify(motion))
    await evaluate(`${G}.motionChoice = null; localStorage.removeItem('ccm.next.motion'); ${S}.get('settings').starsFx = 'twinkle'; ${G}.setNextTheme('mist')`)
  } catch (e) {
    check('run', false, e.message)
    await shot('v102-next-99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v102-next.txt'), results.join('\n'))
    process.exit(0)
  }
})()
