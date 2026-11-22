const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
let app = null
let shots = 0

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-pmenu-')))
const DIRS = ['plain', 'droid'].map((n) => path.join(BASE, n))
for (const d of DIRS) fs.mkdirSync(d)

async function check(ui, name, ok, extra = '') {
  results.push(`${ok ? 'PASS' : 'FAIL'} [${ui}] ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
  if (!ok && app) await app.shot(`v109-project-menu-${ui}-fail-${++shots}`).catch(() => {})
}

const HEADS = `[...document.querySelectorAll('[data-live-rail] [data-side-head], nav[aria-label=Projects] [data-side-head]')].filter((h) => h.offsetParent)`
const MENU = `document.querySelector('[data-context-menu]')`
const rightClick = (sel) => app.evaluate(`(() => { const el = ${sel}; const r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 20, clientY: r.top + 10, button: 2 })); return true })()`)
const menuIds = () => app.evaluate(`JSON.stringify([...document.querySelectorAll('[data-context-menu] [data-menu-id]')].map((e) => e.dataset.menuId))`).then(JSON.parse)
const subIds = () => app.evaluate(`JSON.stringify([...document.querySelectorAll('[data-context-submenu] [data-menu-id]')].map((e) => e.dataset.menuId))`).then(JSON.parse)
const closeMenu = async () => {
  await app.cmd('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
  await app.cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
  await app.sleep(300)
}
const headOf = (name) => `${HEADS}.find((h) => h.textContent.includes(${JSON.stringify(name)}))`

async function run(ui) {
  const { evaluate, cmd, waitFor, sleep } = app
  await waitFor(`!!${S}.get('ui')`, 30000, 'app ready')
  await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.sidebar','1')`)
  await cmd('Page.reload')
  await sleep(1500)
  await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
  await waitFor(`document.documentElement.dataset.ui === ${JSON.stringify(ui)}`, 20000, `data-ui=${ui}`)
  await evaluate(`${S}.get('ui').view = 'workspace'`)
  for (const d of DIRS) await evaluate(`${W}.open('shell', ${JSON.stringify(d)}, undefined, { tab: true })`)
  await waitFor(`${W}.panes.length === ${DIRS.length} && ${W}.panes.every((p) => p.state === 'running')`, 25000, 'shells running')
  await waitFor(`${HEADS}.length === ${DIRS.length}`, 10000, 'project heads')
  await sleep(800)

  const key = (d) => JSON.stringify(d.toLowerCase().replace(/\//g, '\\'))
  await evaluate(`(() => { const r = ${S}.get('run'); r.projects[${key(DIRS[1])}] = { projects: [{ kind: 'android', dir: ${JSON.stringify(DIRS[1])}, name: 'droid', framework: 'native', status: 'no-app', modules: [] }] }; r.projects[${key(DIRS[0])}] = { projects: [] }; return true })()`)

  await rightClick(headOf('plain'))
  await waitFor(`!!${MENU}`, 5000, 'project menu')
  const ids = await menuIds()
  const levelOne = ids.filter((i) => !['scripts', 'plan', 'env', 'i18n', 'perms', 'build-clean'].includes(i))
  await check(ui, 'level one keeps the everyday actions', ['new', 'copy-path', 'move-up', 'move-down', 'toggle', 'close-project'].every((i) => ids.includes(i)), JSON.stringify(ids))
  await check(ui, 'level one holds one tools entry, not the separate tools', ids.includes('tools') && !['scripts', 'plan', 'env', 'i18n', 'perms', 'build-clean'].some((i) => ids.includes(i)), JSON.stringify(levelOne))
  await check(ui, 'level one is short', levelOne.length <= 14, String(levelOne.length))
  let sub = []
  for (let i = 0; i < 3 && !sub.length; i++) {
    await app.evaluate(`document.querySelector('[data-context-menu] [data-menu-id="tools"]').click()`)
    await sleep(600)
    sub = await subIds()
  }
  await check(ui, 'Project tools opens a submenu with the tools', sub.includes('env') && sub.includes('i18n') && sub.includes('plan'), JSON.stringify(sub))
  await check(ui, 'non Android project has no Android permissions entry', !sub.includes('perms'), JSON.stringify(sub))
  await closeMenu()
  await closeMenu()
  await waitFor(`!${MENU}`, 3000, 'menu closed with Esc')
  await check(ui, 'Esc closes the menu', !(await evaluate(`!!${MENU}`)))

  await rightClick(headOf('droid'))
  await waitFor(`!!${MENU}`, 5000, 'droid menu')
  await app.evaluate(`document.querySelector('[data-context-menu] [data-menu-id="tools"]').click()`)
  await sleep(400)
  const subD = await subIds()
  await check(ui, 'Android project submenu lists the tools', subD.includes('env') && subD.includes('i18n'), JSON.stringify(subD))
  await closeMenu()
  await closeMenu()
}

;(async () => {
  const exe = process.argv[2] && !process.argv[2].startsWith('--') ? { exe: process.argv[2] } : {}
  const only = process.argv.find((a) => a.startsWith('--ui='))?.slice(5)
  for (const ui of only ? [only] : ['classic', 'next']) {
    try {
      app = await launch({ ...exe, ui })
      await run(ui)
    } catch (e) {
      await check(ui, 'run', false, e.message)
    } finally {
      app?.stop()
      app = null
      await new Promise((r) => setTimeout(r, 1000))
    }
  }
  fs.writeFileSync(path.join(OUT, 'results-v109-project-menu.txt'), results.join('\n'))
  try {
    fs.rmSync(BASE, { recursive: true, force: true })
  } catch {}
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
