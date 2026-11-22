const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-diskclean-system')
process.env.CCM_E2E_PORT ||= '4476'
process.env.CCM_E2E_DBG ||= '9476'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-2b')
const tag = `orrery-e2e-v101-2b-${process.pid}-${Date.now()}`
const fake = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-sysclean-e2e-')))
const DAY = 86_400_000
const at = (...p) => path.join(fake, ...p)
const write = (abs, n, ageDays = 0) => {
  fs.mkdirSync(path.dirname(abs), { recursive: true })
  fs.writeFileSync(abs, Buffer.alloc(n, 7))
  if (ageDays) {
    const t = new Date(Date.now() - ageDays * DAY)
    fs.utimesSync(abs, t, t)
  }
  return abs
}
const LOCAL = at('home', 'AppData', 'Local')
const TEMP = at('Temp')
const CHROME = path.join(LOCAL, 'Google', 'Chrome', 'User Data', 'Default')
const EDGE = path.join(LOCAL, 'Microsoft', 'Edge', 'User Data', 'Default')
const PROJ = at(`${tag}-old`)
const OPEN = at(`${tag}-open`)
const f = {
  oldLog: write(path.join(TEMP, 'old.log'), 40_000, 3),
  uni: write(path.join(TEMP, 'tệp cũ ✓.tmp'), 20_000, 5),
  newLog: write(path.join(TEMP, 'new.log'), 10_000),
  locked: write(path.join(TEMP, 'locked.tmp'), 10_000, 4),
  chromeCache: write(path.join(CHROME, 'Cache', 'Cache_Data', 'f_000001'), 300_000),
  chromeCode: write(path.join(CHROME, 'Code Cache', 'js', 'index'), 50_000),
  cookies: write(path.join(CHROME, 'Network', 'Cookies'), 2_000),
  history: write(path.join(CHROME, 'History'), 2_000),
  edgeCache: write(path.join(EDGE, 'Cache', 'Cache_Data', 'f_000001'), 120_000),
  dump: write(path.join(LOCAL, 'CrashDumps', 'app.exe.42.dmp'), 80_000),
  minidump: write(at('Windows', 'Minidump', 'x.dmp'), 70_000),
  winTemp: write(at('Windows', 'Temp', 'w.tmp'), 9_000, 9),
  update: write(at('Windows', 'SoftwareDistribution', 'Download', 'kb', 'u.cab'), 60_000),
  setup: write(at('home', 'Downloads', 'Setup-1.0.exe'), 90_000, 45),
  docs: write(at('home', 'Documents', 'thesis.docx'), 1_000, 400),
  bin1: write(at('Bin', '$R1.txt'), 33_000),
  bin2: write(at('Bin', '$R2', 'a'), 7_000),
  pkg: write(path.join(PROJ, 'package.json'), 20, 60),
  lock: write(path.join(PROJ, 'package-lock.json'), 20, 60),
  nm: write(path.join(PROJ, 'node_modules', 'lib', 'index.js'), 200_000),
  openPkg: write(path.join(OPEN, 'package.json'), 20, 90),
  openNm: write(path.join(OPEN, 'node_modules', 'lib', 'index.js'), 100_000),
}
fs.writeFileSync(at('procs.txt'), 'msedge.exe\nexplorer.exe\n')
const held = fs.openSync(f.locked, fs.constants.O_RDWR | 0x10000000)

const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
const binByOrigin = (dir) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $b.GetDetailsOf($_, 1) -like '*${path.basename(dir)}*' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const binFrom = (dir) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $b.GetDetailsOf($_, 1) -eq '${dir.replace(/'/g, "''")}' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const realBinCount = () => (process.platform === 'win32' ? Number(ps('(New-Object -ComObject Shell.Application).NameSpace(10).Items().Count')) : 0)
const purge = (paths) => {
  for (const p of paths) {
    const info = path.join(path.dirname(p), path.basename(p).replace(/^\$R/, '$I'))
    for (const x of [p, info]) fs.rmSync(x, { recursive: true, force: true })
  }
}

;(async () => {
  const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-sysclean-e2e-home-'))
  const binBefore = realBinCount()
  const app = await launch({
    env: { CCM_E2E_DISKMAP_CONFIRM: '1', CCM_E2E_DISKCLEAN_SYS: fake, CLAUDE_CONFIG_DIR: path.join(empty, 'claude'), CODEX_HOME: path.join(empty, 'codex'), CCM_CODEX: '0' },
  })
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
    evaluate(
      `[...document.querySelectorAll('[data-clean-row]')].map((r) => ({ path: r.dataset.cleanRow, locked: r.dataset.cleanLocked || '', checked: r.querySelector('[data-clean-check]').checked, disabled: r.querySelector('[data-clean-check]').disabled, group: r.closest('[data-clean-group]').dataset.cleanGroup, size: r.querySelector('[data-clean-size]').textContent.trim(), emptyBin: !!r.querySelector('[data-clean-empty-bin]') }))`,
    )
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')

  try {
    await waitFor(`typeof ${D}?.diskCleanEmptyBin === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    const i18nSrc = fs.readFileSync(path.join(REPO, 'apps/web/src/features/disk/i18n/disk-clean-v101.ts'), 'utf8').replace(': typeof en', '').replace('export default', 'return')
    const msgs = new Function(i18nSrc)()
    const merged = await evaluate(
      `(() => { const app = document.querySelector('#app').__vue_app__; const pv = app._context.provides; const inst = Object.getOwnPropertySymbols(pv).map((k) => pv[k]).find((v) => v && v.global && v.global.mergeLocaleMessage); const g = inst ? inst.global : app.config.globalProperties.$i18n; const m = ${JSON.stringify(msgs)}; if (!g || !g.mergeLocaleMessage) return false; g.mergeLocaleMessage('en', m.en); g.mergeLocaleMessage('vi', m.vi); g.locale.value = 'vi'; return true })()`,
    )
    check('clean texts merged for this run (en / vi do not import them yet)', merged)

    const cats = await evaluate(`${D}.diskCleanCategories()`)
    const sysIds = ['temp', 'browser', 'crash', 'logs', 'thumbs', 'installers', 'bin', 'update']
    check('categories list the system groups', sysIds.every((id) => cats.some((c) => c.id === id && c.group === 'system')), cats.map((c) => c.id).join(','))

    await evaluate(`(() => { const w = ${S}.get('workspace'); w.open('shell', ${JSON.stringify(OPEN)}); return w.panes.length })()`)
    await sleep(800)
    await evaluate(`(() => { ${S}.get('ui').setView('disk'); return true })()`)
    await waitFor(q('[data-disk-mode]'), 15000, 'disk tabs')
    await evaluate(`(() => { const c = document.querySelector('[data-disk-mode] [role=radio][data-value="clean"]'); if (c) c.click(); return !!c })()`)
    await waitFor(q('[data-disk-clean]'), 15000, 'clean tab')
    await waitFor(`!${S}.get('diskClean').scanning`, 120000, 'first scan settles')
    await evaluate(`(() => { const s = ${S}.get('diskClean'); s.folders = []; s.addFolder(${JSON.stringify(fake)}); return true })()`)
    await sleep(200)
    await click('[data-clean-scan]')
    await waitFor(`${S}.get('diskClean').scanning`, 5000, 'scan started').catch(() => undefined)
    await waitFor(`${S}.get('diskClean').scanned && !${S}.get('diskClean').scanning`, 120000, 'scan done')
    await sleep(500)

    const rows = await rowsState()
    const sysRows = rows.filter((r) => sysIds.includes(r.group))
    const by = (p) => rows.find((r) => r.path === p)
    check('every system row lives in the fake roots', sysRows.length > 0 && sysRows.every((r) => r.path.startsWith(fake)), sysRows.filter((r) => !r.path.startsWith(fake)).map((r) => r.path).join(' | '))
    const items = await evaluate(`${S}.get('diskClean').items.filter((i) => i.group === 'system').map((i) => ({ path: i.path, category: i.category, kind: i.kind, files: i.files, size: i.size, locked: i.locked || '' }))`)
    const temp = items.find((i) => i.path === TEMP)
    check('Temp row counts only files older than 24 hours, the locked one skipped', temp && temp.files === 2 && temp.size === 60_000, JSON.stringify(temp))
    check('Chrome caches offered (Cache, Code Cache), profile files never', !!by(path.join(CHROME, 'Cache')) && !!by(path.join(CHROME, 'Code Cache')) && !rows.some((r) => r.path.includes('Cookies') || r.path.endsWith('History') || r.path.endsWith('Network')))
    check('Edge is open: its cache is locked', by(path.join(EDGE, 'Cache'))?.locked === 'browser' && by(path.join(EDGE, 'Cache'))?.disabled)
    const admin = [at('Windows', 'Minidump'), at('Windows', 'Temp'), at('Windows', 'SoftwareDistribution', 'Download')].map(by)
    check('admin places show size, locked admin, no tick', admin.every((r) => r && r.locked === 'admin' && r.disabled && r.size !== '0 B'), JSON.stringify(admin.map((r) => r && [r.locked, r.size])))
    const setupRow = by(f.setup)
    check('old installer is listed but not ticked', !!setupRow && !setupRow.checked && !setupRow.disabled)
    const binRow = by(at('Bin'))
    check('Recycle Bin row: size and an Empty button, no tick', !!binRow && binRow.disabled && binRow.emptyBin && binRow.size !== '0 B', JSON.stringify(binRow))
    check('Documents never appear', !rows.some((r) => r.path.includes(`${path.sep}Documents`)))
    check('Temp, Chrome cache and crash dumps are ticked', !!by(TEMP)?.checked && !!by(path.join(CHROME, 'Cache'))?.checked && !!by(path.join(LOCAL, 'CrashDumps'))?.checked)
    await evaluate(`${q('[data-clean-group="temp"]')}?.scrollIntoView({ block: 'start' })`)
    await sleep(300)
    await shotTo('sys-tab')

    const forged = await evaluate(`${D}.diskCleanDelete({ paths: [${JSON.stringify(at('home', 'Documents'))}], permanent: true, open: [] })`)
    check('a path outside the last scan is refused', forged.failed?.[0]?.reason === 'invalid' && fs.existsSync(f.docs), JSON.stringify(forged.failed))
    const adminTry = await evaluate(`${D}.diskCleanDelete({ paths: [${JSON.stringify(at('Windows', 'Minidump'))}], permanent: true, open: [] })`)
    check('desktop refuses an admin place', adminTry.failed?.[0]?.reason === 'locked:admin' && fs.existsSync(f.minidump), JSON.stringify(adminTry.failed))
    const tapForged = await evaluate(`${D}.diskCleanDelete({ paths: [${JSON.stringify(f.setup)}], permanent: false, open: [], oneTap: true })`)
    check('one tap refuses an installer', tapForged.failed?.[0]?.reason === 'invalid' && fs.existsSync(f.setup), JSON.stringify(tapForged.failed))

    await click('[data-clean-onetap-start]')
    await waitFor(`${S}.get('diskClean').tap === 'ready'`, 120000, 'one tap ready')
    await sleep(400)
    const groups = await evaluate(`[...document.querySelectorAll('[data-clean-onetap-group]')].map((g) => g.dataset.cleanOnetapGroup)`)
    check('one tap lists only safe groups', groups.join(',') === ['node_modules', 'temp', 'browser', 'crash'].join(','), groups.join(','))
    const tapPaths = await evaluate(`${S}.get('diskClean').tapItems.map((i) => i.path)`)
    check('one tap skips the open project, installers, bin, admin', tapPaths.includes(path.join(PROJ, 'node_modules')) && !tapPaths.some((p) => p.startsWith(OPEN) || p === f.setup || p === at('Bin') || p.includes(`${path.sep}Windows${path.sep}`)), tapPaths.map((p) => path.relative(fake, p)).join(' | '))
    await evaluate(`${q('[data-clean-onetap]')}?.scrollIntoView({ block: 'center' })`)
    await shotTo('sys-onetap')
    await click('[data-clean-onetap-run]')
    await waitFor(`${S}.get('diskClean').tap === 'idle' && !${S}.get('diskClean').deleting`, 60000, 'one tap done')
    await waitFor(q('[data-clean-report]'), 10000, 'report')
    check('Temp: old files gone, new and locked kept, folder kept', !fs.existsSync(f.oldLog) && !fs.existsSync(f.uni) && fs.existsSync(f.newLog) && fs.existsSync(f.locked) && fs.existsSync(TEMP))
    check('Chrome cache gone, cookies and history kept', !fs.existsSync(path.join(CHROME, 'Cache')) && !fs.existsSync(path.join(CHROME, 'Code Cache')) && fs.existsSync(f.cookies) && fs.existsSync(f.history))
    check('Edge cache (open) and installer and admin places untouched', fs.existsSync(f.edgeCache) && fs.existsSync(f.setup) && fs.existsSync(f.minidump) && fs.existsSync(f.update) && fs.existsSync(f.winTemp))
    check('crash dump gone, open project kept', !fs.existsSync(f.dump) && fs.existsSync(f.openNm))
    check('old project node_modules gone', !fs.existsSync(path.join(PROJ, 'node_modules')))
    const inBin = [...binFrom(PROJ), ...binFrom(path.join(LOCAL, 'CrashDumps'))]
    check('dev junk and crash dump went to the Recycle Bin, Temp and browser cache did not', process.platform !== 'win32' || (inBin.length === 2 && binFrom(TEMP).length === 0 && binFrom(CHROME).length === 0), inBin.join(' | '))
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 5 })`)
    check('history has a delete and a trash row', hist.some((h) => h.action === 'delete' && h.paths.some((p) => p.startsWith(fake))) && hist.some((h) => h.action === 'trash' && h.paths.some((p) => p.startsWith(PROJ))), JSON.stringify(hist.map((h) => [h.action, h.count])))
    await shotTo('sys-onetap-done')
    purge(inBin)

    await evaluate(`${q('[data-clean-group="bin"]')}?.scrollIntoView({ block: 'center' })`)
    await click('[data-clean-empty-bin]')
    await waitFor(`!${S}.get('diskClean').deleting && (${S}.get('diskClean').items.find((i) => i.action === 'bin')?.size === 0)`, 30000, 'bin emptied')
    check('Empty clears the fake bin only', fs.readdirSync(at('Bin')).length === 0)
    await sleep(300)
    await shotTo('sys-bin-done')

    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(600)
    const [s, c] = await sw()
    check('no horizontal scroll at 900px', s <= c, `${s} <= ${c}`)
    await shotTo('sys-900')
    await cmd('Emulation.clearDeviceMetricsOverride')
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    purge(binByOrigin(fake))
    try {
      fs.closeSync(held)
    } catch {}
    const binAfter = realBinCount()
    check('real Recycle Bin count unchanged after cleanup', binAfter === binBefore, `${binBefore} -> ${binAfter}`)
    fs.rmSync(fake, { recursive: true, force: true })
    fs.rmSync(empty, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
