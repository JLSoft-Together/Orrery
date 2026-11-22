const { spawn, execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const SHOTS = path.join(REPO, 'docs/qa/v103/WP-70a')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

async function fixture() {
  const root = path.join(os.tmpdir(), `ccm-e2e-v103-70a-${process.pid}`)
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
    await sleep(250)
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.join(SHOTS, name + '.png'), Buffer.from(r.result.data, 'base64'))
  }
  const fit = async (label) => {
    const m = JSON.parse(await evaluate(`JSON.stringify((() => { const f = document.querySelector('[data-next-footer]').getBoundingClientRect(); const panes = [...document.querySelectorAll('[data-pane]')].map((e) => e.getBoundingClientRect().bottom); const rail = document.querySelector('[data-next-rail]')?.getBoundingClientRect(); return { sh: document.scrollingElement.scrollHeight, ih: window.innerHeight, sw: document.scrollingElement.scrollWidth, iw: window.innerWidth, footTop: f.top, footBottom: f.bottom, paneBottom: panes.length ? Math.max(...panes) : null, railBottom: rail ? rail.bottom : null } })())`))
    check(label + ': page does not scroll', m.sh <= m.ih && m.sw <= m.iw, JSON.stringify(m))
    check(label + ': panes end above the footer with a gap', m.paneBottom === null || m.footTop - m.paneBottom >= 6, JSON.stringify(m))
    check(label + ': rail ends above the footer', m.railBottom === null || m.footTop - m.railBottom >= 6, JSON.stringify(m))
  }
  const railW = async () => (await rect('[data-next-rail]'))?.width ?? 0
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.next.railCollapsed'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
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

    const btn = JSON.parse(await evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-explorer-toggle]'); return b ? { label: b.getAttribute('aria-label'), text: b.textContent.trim(), svg: b.querySelector('svg')?.getAttribute('class') ?? '' } : null })())`))
    check('Explorer button has its own icon, a visible label and a tooltip with the chord', !!btn && btn.text === 'Explorer' && /Explorer \(Ctrl\+Shift\+E\)/.test(btn.label) && /folder-tree/.test(btn.svg), JSON.stringify(btn))
    const layoutBtn = await evaluate(`!!document.querySelector('[data-pane-bar]')`)
    check('Explorer button differs from the layout button next to it', layoutBtn)
    await evaluate(`document.querySelector('[data-explorer-toggle]').click()`)
    await sleep(500)
    check('Explorer button opens the drawer and shows the pressed state', await evaluate(`document.querySelector('[data-explorer-toggle]').getAttribute('aria-pressed') === 'true' && ['open','pinned'].includes(document.querySelector('[data-explorer-drawer]')?.dataset.state)`))
    await key('Escape', 'Escape', 27)
    await sleep(400)

    check('needs box is gone from the rail', await evaluate(`!document.querySelector('[data-rail-needs]') && !document.querySelector('[data-rail-need]')`))

    const rows = JSON.parse(await evaluate(`JSON.stringify((() => { const head = document.querySelector('[data-side-head]'); const kids = document.querySelector('[data-rail-children]'); const row = kids?.querySelector('[data-side-row]'); if (!head || !kids || !row) return null; const h = head.getBoundingClientRect(); const k = kids.getBoundingClientRect(); const r = row.getBoundingClientRect(); return { head: h.left, kids: k.left, row: r.left, mono: head.querySelector('.ccm-next-rail-mono').getBoundingClientRect().left } })())`))
    check('sessions sit indented under their project', !!rows && rows.row - rows.head >= 16 && rows.kids > rows.head, JSON.stringify(rows))

    const full = await railW()
    check('rail is full width at 1600', full >= 224, String(full))
    const foot = JSON.parse(await evaluate(`JSON.stringify((() => { const f = document.querySelector('[data-next-footer]'); const r = f?.getBoundingClientRect(); return r ? { top: r.top, bottom: r.bottom, left: r.left, right: r.right, h: window.innerHeight, w: window.innerWidth } : null })())`))
    check('limits footer is the bottom strip of the window', !!foot && foot.h - foot.bottom <= 14 && foot.right - foot.left > foot.w * 0.9, JSON.stringify(foot))
    check('old quota block is gone from the rail', await evaluate(`!document.querySelector('[data-next-rail] [data-next-quota]')`))
    await evaluate(`${S}.get('extras').limits = null; ${S}.get('extras').codexLimits = null`)
    await sleep(400)
    check('footer is empty with no limits', await evaluate(`!!document.querySelector('[data-next-footer]') && !document.querySelector('[data-quota-empty]') && !document.querySelector('[data-quota-agent]')`))
    await evaluate(`(() => { const e = ${S}.get('extras'); const iso = (m) => new Date(Date.now() + m * 60000).toISOString(); e.limits = { updatedAt: new Date().toISOString(), fiveHour: { usedPct: 42, resetsAt: iso(95) }, sevenDay: { usedPct: 91, resetsAt: iso(3 * 1440) } }; e.codexLimits = { updatedAt: new Date().toISOString(), fiveHour: { usedPct: 7, resetsAt: iso(300) } } })()`)
    await sleep(500)
    const q = JSON.parse(await evaluate(`JSON.stringify([...document.querySelectorAll('[data-quota-agent]')].map((a) => ({ id: a.dataset.quotaAgent, text: a.textContent.replace(/\\s+/g, ' ').trim() })))`))
    check('footer shows Claude and Codex icon clusters with percent only', q.length === 2 && /42%.*91%/.test(q[0].text) && /7%/.test(q[1].text) && !/reset/.test(q[0].text), JSON.stringify(q))
    check('footer fits one line without page scroll', await evaluate(`document.querySelector('[data-next-footer]').getBoundingClientRect().height <= 32 && document.documentElement.scrollWidth <= document.documentElement.clientWidth`))
    check('no forecast icon without enough samples', await evaluate(`!document.querySelector('[data-quota-forecast-icon]')`))
    await evaluate(`(() => { const f = ${S}.get('quotaForecast'); const t = Date.now(); f.samples['codex:5h'] = [{ at: t - 20 * 60000, pct: 0 }, { at: t - 1000, pct: 7 }]; f.samples['claude:5h'] = [{ at: t - 20 * 60000, pct: 41 }, { at: t - 1000, pct: 42 }]; const e = ${S}.get('extras'); e.codexLimits = { ...e.codexLimits, updatedAt: new Date().toISOString() } })()`)
    await sleep(500)
    check('forecast icon only on the agent whose rate fills before reset', await evaluate(`!!document.querySelector('[data-quota-agent="codex"] [data-quota-forecast-icon]') && !document.querySelector('[data-quota-agent="claude"] [data-quota-forecast-icon]')`))
    await evaluate(`${S}.get('settings').setThemeMode('light')`)
    await sleep(500)
    await fit('full')
    await snap('01-light-full')

    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)
    const strip = await railW()
    check('collapse button shrinks the rail to the 56px strip', Math.abs(strip - 56) <= 1, String(strip))
    check('collapsed state is remembered', await evaluate(`localStorage.getItem('ccm.next.railCollapsed') === '1'`))
    check('collapsed rail keeps session icons and an expand button', await evaluate(`!!document.querySelector('[data-rail-collapse][aria-expanded="false"]') && document.querySelectorAll('[data-live-rail] button').length > 0`))
    await fit('collapsed')
    await snap('02-light-collapsed')
    await cmd('Page.reload')
    await waitFor(`document.documentElement.dataset.ui === 'next' && !!document.querySelector('[data-next-rail]')`, 30000, 'reloaded')
    await sleep(800)
    check('collapsed rail survives a reload', Math.abs((await railW()) - 56) <= 1, String(await railW()))
    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)
    check('expand button restores the full rail', (await railW()) >= 224, String(await railW()))
    check('expanded state is stored', await evaluate(`localStorage.getItem('ccm.next.railCollapsed') === null`))

    await evaluate(`document.querySelector('.xterm-helper-textarea')?.blur(); document.activeElement?.blur?.()`)
    await key('B', 'KeyB', 66, 2 | 8)
    await sleep(700)
    check('Ctrl+Shift+B collapses', Math.abs((await railW()) - 56) <= 1, String(await railW()))
    await key('B', 'KeyB', 66, 2 | 8)
    await sleep(700)
    check('Ctrl+Shift+B expands', (await railW()) >= 224, String(await railW()))

    await evaluate(`${S}.get('settings').setThemeMode('dark')`)
    await sleep(500)
    await fit('dark full')
    await snap('03-dark-full')
    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)
    await fit('dark collapsed')
    await snap('04-dark-collapsed')
    await evaluate(`document.querySelector('[data-rail-collapse]').click()`)
    await sleep(700)

    await size(1100, 800)
    await sleep(900)
    await fit('icon')
    check('icon mode by width has no collapse button', await evaluate(`!document.querySelector('[data-rail-collapse]')`))
    check('footer still spans the window at 1100', await evaluate(`document.querySelector('[data-next-footer]').getBoundingClientRect().right - document.querySelector('[data-next-footer]').getBoundingClientRect().left > window.innerWidth * 0.9`))
    await size(900, 800)
    await sleep(900)
    await fit('overlay')
    await size(1600, 900)
    await sleep(600)
    await evaluate(`${S}.get('settings').setThemeMode('system'); ${S}.get('extras').limits = null; ${S}.get('extras').codexLimits = null`)
  } catch (e) {
    check('run', false, e.message)
    await shot('v103-70a-99-error').catch(() => {})
  } finally {
    app.stop()
    try { process.platform === 'win32' ? execFileSync('taskkill', ['/T', '/F', '/PID', String(demo.child.pid)], { stdio: 'ignore' }) : demo.child.kill() } catch {}
    fs.writeFileSync(path.join(OUT, 'results-v103-70a.txt'), results.join('\n'))
    process.exit(0)
  }
})()
