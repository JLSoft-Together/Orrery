const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-diskclean')
process.env.CCM_E2E_PORT ||= '4473'
process.env.CCM_E2E_DBG ||= '9473'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-2a')
const tag = `orrery-e2e-v101-2a-${process.pid}-${Date.now()}`
const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-diskclean-e2e-')))
const DAY = 86_400_000
const write = (rel, n, ageDays = 0) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
  if (ageDays) {
    const t = new Date(Date.now() - ageDays * DAY)
    fs.utimesSync(f, t, t)
  }
}
const OLD = path.join(root, `${tag}-old`)
const NEW = path.join(root, `${tag}-new`)
const OPEN = path.join(root, `${tag}-open`)
write(`${tag}-old/package.json`, 20, 60)
write(`${tag}-old/package-lock.json`, 20, 60)
write(`${tag}-old/node_modules/lib/index.js`, 300_000)
write(`${tag}-old/dist/app.js`, 50_000)
write(`${tag}-new/package.json`, 20)
write(`${tag}-new/pnpm-lock.yaml`, 20)
write(`${tag}-new/node_modules/lib/index.js`, 120_000)
write(`${tag}-open/pubspec.yaml`, 20, 90)
write(`${tag}-open/.dart_tool/cache.bin`, 80_000)
write(`${tag}-open/build/app.bin`, 200_000)
write('docs/build/page.html', 10_000)

const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
const binItems = (name) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $_.Name -like '${name}*' -or $_.Path -like '*${name}*' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const binByOrigin = (dir) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $b.GetDetailsOf($_, 1) -like '*${path.basename(dir)}*' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const purge = (paths) => {
  for (const p of paths) {
    const info = path.join(path.dirname(p), path.basename(p).replace(/^\$R/, '$I'))
    for (const f of [p, info]) fs.rmSync(f, { recursive: true, force: true })
  }
}

;(async () => {
  const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-diskclean-e2e-home-'))
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1', CLAUDE_CONFIG_DIR: path.join(empty, 'claude'), CODEX_HOME: path.join(empty, 'codex'), CCM_CODEX: '0' } })
  const { evaluate, waitFor, shot, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const D = 'window.ccmDesktop'
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  const click = (sel) => evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.click(); return true })()`)
  const shotTo = async (name) => {
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)
    await sleep(400)
    await shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const rowsState = () =>
    evaluate(`[...document.querySelectorAll('[data-clean-row]')].map((r) => ({ path: r.dataset.cleanRow, locked: r.dataset.cleanLocked || '', checked: r.querySelector('[data-clean-check]').checked, disabled: r.querySelector('[data-clean-check]').disabled, group: r.closest('[data-clean-group]').dataset.cleanGroup }))`)
  const mine = (rows) => rows.filter((r) => r.path.includes(tag))
  const relOf = (p) => path.relative(root, p).split(path.sep).join('/').replace(`${tag}-`, '')
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')

  try {
    await waitFor(`typeof ${D}?.diskCleanScan === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    const i18nSrc = fs.readFileSync(path.join(REPO, 'apps/web/src/i18n/disk-clean-v101.ts'), 'utf8').replace(': typeof en', '').replace('export default', 'return')
    const msgs = new Function(i18nSrc)()
    const merged = await evaluate(`(() => { const app = document.querySelector('#app').__vue_app__; const pv = app._context.provides; const inst = Object.getOwnPropertySymbols(pv).map((k) => pv[k]).find((v) => v && v.global && v.global.mergeLocaleMessage); const g = inst ? inst.global : app.config.globalProperties.$i18n; const m = ${JSON.stringify(msgs)}; if (!g || !g.mergeLocaleMessage) return false; g.mergeLocaleMessage('en', m.en); g.mergeLocaleMessage('vi', m.vi); return true })()`)
    check('clean texts merged for this run (en / vi do not import them yet)', merged)

    const cats = await evaluate(`${D}.diskCleanCategories()`)
    check('categories list the dev groups', ['node_modules', 'build', 'cache'].every((id) => cats.some((c) => c.id === id)), cats.map((c) => c.id).join(','))

    await evaluate(`(() => { const w = ${S}.get('workspace'); w.open('shell', ${JSON.stringify(OPEN)}); return w.panes.length })()`)
    await sleep(800)
    await evaluate(`(() => { ${S}.get('ui').setView('disk'); return true })()`)
    await waitFor(q('[data-disk-mode]'), 15000, 'disk tabs')
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-disk-mode] [role=radio]')]; r[r.length - 1].click(); return r.length })()`)
    await waitFor(q('[data-disk-clean]'), 15000, 'clean tab')
    check('Disk view has a Clean up tab', true)
    await waitFor(`!${S}.get('diskClean').scanning`, 120000, 'first scan settles')
    await evaluate(`(() => { const s = ${S}.get('diskClean'); s.folders = []; s.addFolder(${JSON.stringify(root)}); return true })()`)
    await sleep(200)
    await click('[data-clean-scan]')
    await waitFor(`${S}.get('diskClean').scanning`, 5000, 'scan started').catch(() => undefined)
    await waitFor(`${S}.get('diskClean').scanned && !${S}.get('diskClean').scanning`, 120000, 'scan done')
    await sleep(500)

    const rows = mine(await rowsState())
    const by = Object.fromEntries(rows.map((r) => [relOf(r.path), r]))
    check('found exactly the junk of the 3 projects', Object.keys(by).sort().join(',') === ['new/node_modules', 'old/dist', 'old/node_modules', 'open/.dart_tool', 'open/build'].join(','), Object.keys(by).sort().join(','))
    check('build/ without a project marker is not offered', !(await rowsState()).some((r) => r.path.includes(`${path.sep}docs${path.sep}build`)))
    check('old project (60 days) is ticked', by['old/node_modules']?.checked === true && by['old/dist']?.checked === true)
    check('new project is not ticked', by['new/node_modules']?.checked === false && !by['new/node_modules']?.locked)
    check('open project is locked and cannot be ticked', by['open/build']?.locked === 'open' && by['open/build']?.disabled && by['open/.dart_tool']?.locked === 'open')
    const restore = await evaluate(`Object.fromEntries(${S}.get('diskClean').items.filter((i) => i.path.includes(${JSON.stringify(tag)})).map((i) => [i.path, i.restore]))`)
    check('restore command follows the lockfile', restore[path.join(NEW, 'node_modules')] === 'pnpm install' && restore[path.join(OLD, 'node_modules')] === 'npm install' && restore[path.join(OPEN, 'build')] === 'flutter pub get', JSON.stringify(Object.values(restore)))
    check('tool caches wait for an explicit Measure', !!(await evaluate(`!!${q('[data-clean-group="cache"] [data-clean-measure]')}`)))
    await click('[data-clean-group="cache"] [data-clean-measure]')
    const t0 = Date.now()
    await waitFor(`${S}.get('diskClean').items.some((i) => i.category === 'cache') || !${S}.get('diskClean').scanning`, 120000, 'first cache measured')
    const firstMs = Date.now() - t0
    const caches = await evaluate(`${S}.get('diskClean').items.filter((i) => i.category === 'cache').map((i) => ({ path: i.path, kind: i.kind, size: i.size }))`)
    const cacheTicked = await evaluate(`${S}.get('diskClean').items.filter((i) => i.category === 'cache' && ${S}.get('diskClean').selected.has(i.path)).length`)
    check('global caches are read for size only and never ticked', cacheTicked === 0, `${caches.length} so far in ${firstMs} ms: ${caches.map((c) => `${c.kind} ${Math.round(c.size / 1e6)} MB`).join(', ')}`)
    if (await evaluate(`${S}.get('diskClean').scanning`)) {
      await click('[data-clean-cancel]')
      await waitFor(`!${S}.get('diskClean').scanning`, 60000, 'cache measure cancelled')
      check('measuring caches can be cancelled', true)
    }

    await evaluate(`(() => { const s = ${S}.get('diskClean'); const keep = new Set([...s.selected].filter((p) => p.includes(${JSON.stringify(tag)}))); s.selected = keep; return keep.size })()`)
    await sleep(300)
    await evaluate(`${q('[data-clean-group="node_modules"]')}?.scrollIntoView({ block: 'start' })`)
    await sleep(300)
    await shotTo('clean-tab')

    const locked = await evaluate(`${D}.diskCleanDelete({ paths: [${JSON.stringify(path.join(OPEN, 'build'))}], permanent: true, open: [${JSON.stringify(OPEN)}] })`)
    check('desktop refuses a folder of an open project', locked.failed?.[0]?.reason === 'locked:open' && fs.existsSync(path.join(OPEN, 'build')), JSON.stringify(locked.failed))
    const forged = await evaluate(`${D}.diskCleanDelete({ paths: [${JSON.stringify(path.join(root, 'docs'))}], permanent: true, open: [] })`)
    check('a path outside the last scan is refused', forged.failed?.[0]?.reason === 'invalid' && fs.existsSync(path.join(root, 'docs')), JSON.stringify(forged.failed))

    await click('[data-clean-trash]')
    await waitFor(q('[data-clean-report]'), 30000, 'delete report')
    check('ticked folders of the old project are gone', !fs.existsSync(path.join(OLD, 'node_modules')) && !fs.existsSync(path.join(OLD, 'dist')))
    check('new and open projects are untouched', fs.existsSync(path.join(NEW, 'node_modules')) && fs.existsSync(path.join(OPEN, 'build')) && fs.existsSync(path.join(OPEN, '.dart_tool')))
    const inBin = binByOrigin(OLD)
    check('they are in the Recycle Bin', process.platform !== 'win32' || inBin.length === 2, inBin.join(' | '))
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 5 })`)
    check('history has the clean', hist.length >= 1 && hist[0].action === 'trash' && hist[0].count === 2 && hist[0].paths.every((p) => p.includes(tag)), JSON.stringify(hist[0]))
    await waitFor(`${q('[data-disk-history-empty]')} === null`, 5000, 'history row')
    check('history list shows a row', true)
    await shotTo('clean-done')
    purge(inBin)

    await evaluate(`(() => { ${S}.get('diskClean').openFor(${JSON.stringify(NEW)}); return true })()`)
    await waitFor(`${S}.get('diskClean').scanned && !${S}.get('diskClean').scanning && ${S}.get('diskClean').project === ${JSON.stringify(NEW)}`, 30000, 'project scan')
    await sleep(300)
    const only = mine(await rowsState()).map((r) => relOf(r.path))
    check('opening for one project filters to it', only.join(',') === 'new/node_modules' && !!(await evaluate(`!!${q('[data-clean-project]')}`)), only.join(','))
    await shotTo('clean-project')

    await evaluate(`(() => { ${S}.get('diskClean').openFor(null); ${S}.get('ui').setView('monitor'); return true })()`)
    await sleep(300)
    await evaluate(`(() => { ${S}.get('ui').buildClean = { dir: ${JSON.stringify(OLD)} }; return true })()`)
    await waitFor(`${S}.get('ui').view === 'disk' && ${S}.get('diskClean').project === ${JSON.stringify(OLD)}`, 10000, 'v77 redirect')
    check('old Build cleanup entry opens the Clean up tab filtered to the project', !(await evaluate(`!!document.querySelector('[data-build-cancel], [aria-label][role=dialog] [data-build-empty]')`)))

    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(600)
    const [s, c] = await sw()
    check('no horizontal scroll at 900px', s <= c, `${s} <= ${c}`)
    await shotTo('clean-900')
    await cmd('Emulation.clearDeviceMetricsOverride')
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    purge(binByOrigin(OLD))
    purge(binItems(tag))
    fs.rmSync(root, { recursive: true, force: true })
    fs.rmSync(empty, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
