const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-projsize')
process.env.CCM_E2E_PORT ||= '4475'
process.env.CCM_E2E_DBG ||= '9475'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-2c')
const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-projsize-e2e-')))
const PROJ = path.join(root, 'proj-e2e')
const write = (rel, n) => {
  const f = path.join(PROJ, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
}
write('src/a.bin', 300_000)
write('src/deep/b.bin', 120_000)
write('assets/c.bin', 50_000)
write('node_modules/lib/index.js', 200_000)
write('package.json', 20)
write('package-lock.json', 20)
const old = new Date(Date.now() - 60 * 86_400_000)
for (const f of ['package.json', 'package-lock.json']) fs.utimesSync(path.join(PROJ, f), old, old)

;(async () => {
  const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-projsize-e2e-home-'))
  const app = await launch({ env: { CLAUDE_CONFIG_DIR: path.join(empty, 'claude'), CODEX_HOME: path.join(empty, 'codex'), CCM_CODEX: '0' } })
  const { evaluate, waitFor, shot, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const W = `${S}.get('workspace')`
  const D = 'window.ccmDesktop'
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  const rowOf = (name) => `[...document.querySelectorAll('[data-explorer-tree] [role=treeitem]')].find((r) => r.querySelector('[data-row-name]')?.textContent.trim() === ${JSON.stringify(name)})`
  const sizeOfRow = (name) => evaluate(`(() => { const r = ${rowOf(name)}; return r?.querySelector('[data-row-size]')?.textContent.trim() ?? null })()`)
  const openMenu = (sel) => evaluate(`(() => { const el = ${sel}; const r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 20, clientY: r.top + 5 })) })()`)
  const menuIds = () => evaluate(`[...document.querySelectorAll('[data-context-menu] [role=menuitem]')].map((b) => b.dataset.menuId)`)
  const pick = (id) => evaluate(`(() => { const b = document.querySelector('[data-context-menu] [data-menu-id="${id}"]'); if (!b) return false; b.click(); return true })()`)
  const shotTo = async (name) => {
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)
    await sleep(400)
    await shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const sizeLike = /^[0-9]+(\.[0-9]+)? (B|KB|MB|GB)$/
  const scanAndWait = async () => {
    await evaluate(`${D}.diskMapScan({ roots: [${JSON.stringify(root)}], level: 'light', excludes: [] })`)
    await waitFor(`${D}.diskMapStatus().then((s) => s.indexed && !s.scanning)`, 60000, 'scan done')
    await sleep(600)
  }

  try {
    await waitFor(`typeof ${D}?.diskMapSizes === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 950, deviceScaleFactor: 1, mobile: false })
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell')`)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(q('[data-launch-row]'), 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(PROJ)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(800)
    await evaluate(`if (!document.querySelector('[data-explorer-tree]')) document.querySelector('[data-explorer-toggle]').click()`)
    await waitFor(`!!(${rowOf('src')})`, 10000, 'explorer rows')
    await sleep(800)

    check('no index yet: Explorer shows no size', (await evaluate(`document.querySelectorAll('[data-explorer-tree] [data-row-size]').length`)) === 0)
    check('no index yet: bridge answers null without starting a scan', (await evaluate(`${D}.diskMapSizes({ paths: [${JSON.stringify(PROJ)}] })`)) === null)
    await openMenu(rowOf('src'))
    await waitFor(q('[data-context-menu]'), 3000, 'menu')
    const idsNone = await menuIds()
    await evaluate(`document.querySelector('[data-context-menu]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`)
    await sleep(250)
    check('no index yet: menu has Clean up but not Open in Disk', idsNone.includes('disk-clean') && !idsNone.includes('disk-open'), idsNone.join(','))
    await shotTo('explorer-no-index')

    await scanAndWait()
    await waitFor(`document.querySelectorAll('[data-explorer-tree] [data-row-size]').length > 0`, 15000, 'sizes appear after the scan')
    const srcText = await sizeOfRow('src')
    const assetsText = await sizeOfRow('assets')
    check('folder rows show a size after the scan', sizeLike.test(srcText ?? '') && sizeLike.test(assetsText ?? ''), `src=${srcText} assets=${assetsText}`)
    const truth = await evaluate(`${D}.diskMapChildren({ path: ${JSON.stringify(PROJ)}, sort: 'size', limit: 50 })`)
    const srcTrue = truth.items.find((i) => i.name === 'src')
    check('src size matches the index', !!srcTrue && srcTrue.size >= 420_000, `${srcTrue?.size}`)
    check('file rows show no size', (await evaluate(`(() => { const r = ${rowOf('package.json')}; return r ? r.querySelector('[data-row-size]') === null : false })()`)) === true)
    const rootSize = await evaluate(`document.querySelector('[data-explorer-tree] [data-row-index="0"] [data-row-size]')?.textContent.trim() ?? null`)
    check('root row shows the project size', sizeLike.test(rootSize ?? ''), rootSize)

    const upper = await evaluate(`${D}.diskMapSizes({ paths: [${JSON.stringify(PROJ.toUpperCase())}, ${JSON.stringify(path.join(PROJ, 'src'))}, ${JSON.stringify(path.join(PROJ, 'nope'))}] })`)
    check('size lookup ignores path case on Windows and gives null for unknown paths', process.platform !== 'win32' ? true : upper.sizes[PROJ.toUpperCase()] > 0 && upper.sizes[path.join(PROJ, 'nope')] === null, JSON.stringify(upper.sizes))

    await shotTo('explorer-sizes')

    await openMenu(rowOf('src'))
    await waitFor(q('[data-context-menu]'), 3000, 'menu')
    const ids = await menuIds()
    check('folder menu offers Open in Disk and Clean up this project', ids.includes('disk-open') && ids.includes('disk-clean'), ids.join(','))
    const labels = await evaluate(`[...document.querySelectorAll('[data-context-menu] [data-menu-id^="disk-"]')].map((b) => b.textContent.trim())`)
    check('labels are the English texts', labels.join('|') === 'Open in Disk|Clean up this project', labels.join('|'))
    await shotTo('explorer-menu')
    await pick('disk-open')
    await waitFor(`${S}.get('ui').view === 'disk'`, 5000, 'disk view')
    await waitFor(`!!${S}.get('diskMap').path`, 5000, 'path set')
    await sleep(1000)
    const at = await evaluate(`${S}.get('diskMap').path`)
    check('Open in Disk lands on the folder in Browse', at.toLowerCase() === path.join(PROJ, 'src').toLowerCase() && (await evaluate(`${S}.get('diskClean').active`)) === false, at)
    check('Browse lists the folder content', (await evaluate(`${S}.get('diskMap').items.map((i) => i.name).sort().join(',')`)) === 'a.bin,deep')
    await shotTo('disk-opened')

    await evaluate(`${S}.get('ui').setView('workspace')`)
    await waitFor(`!!(${rowOf('assets')})`, 8000, 'explorer back')
    await sleep(500)
    await openMenu(`document.querySelector('[data-explorer-tree] [data-row-index="0"]')`)
    await waitFor(q('[data-context-menu]'), 3000, 'root menu')
    await pick('disk-clean')
    await waitFor(`${S}.get('ui').view === 'disk' && ${S}.get('diskClean').active`, 5000, 'clean tab')
    check('Clean up this project opens the Clean tab filtered to the project', (await evaluate(`${S}.get('diskClean').project`)) === PROJ)
    await waitFor(`${S}.get('diskClean').scanned && !${S}.get('diskClean').scanning`, 60000, 'project scan').catch(() => {})
    await sleep(500)
    await shotTo('clean-filtered')

    await evaluate(`${S}.get('ui').setView('workspace')`)
    await waitFor(`!!(${rowOf('assets')})`, 8000, 'explorer back')
    await sleep(500)
    const tip = await evaluate(`document.querySelector('[data-side-head]')?.title ?? ''`)
    check('rail project tooltip carries the size and the cleanable junk', /Size [0-9]/.test(tip) && /Cleanable [0-9]/.test(tip), JSON.stringify(tip))

    fs.writeFileSync(path.join(PROJ, 'assets', 'big.bin'), Buffer.alloc(900_000, 3))
    const before = await sizeOfRow('assets')
    await scanAndWait()
    await waitFor(`(() => { const r = ${rowOf('assets')}; const t = r?.querySelector('[data-row-size]')?.textContent.trim(); return !!t && t !== ${JSON.stringify(before)} })()`, 15000, 'size refreshed after rescan').catch(() => {})
    const after = await sizeOfRow('assets')
    check('a rescan refreshes the shown size', !!after && after !== before, `${before} -> ${after}`)
    await shotTo('explorer-rescanned')

    const [sw, cw] = await evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
    check('no horizontal scroll', sw <= cw, `${sw} <= ${cw}`)
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    fs.rmSync(root, { recursive: true, force: true })
    fs.rmSync(empty, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
