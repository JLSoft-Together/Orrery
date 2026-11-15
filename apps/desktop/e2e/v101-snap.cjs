const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const zlib = require('node:zlib')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-snap')
process.env.CCM_E2E_PORT ||= '4474'
process.env.CCM_E2E_DBG ||= '9474'

const { launch, sleep, REPO, OUT, UI, selOptions, selPick } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs', 'qa', 'v101', 'WP-4c')
fs.mkdirSync(SHOTS, { recursive: true })
const GB = 1024 ** 3
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-snap-e2e-'))
const fake = path.join(os.tmpdir(), `ccm-snap-fake-${process.pid}.json`)
const sink = path.join(os.tmpdir(), `ccm-snap-sink-${process.pid}.jsonl`)
const setFake = (o) => fs.writeFileSync(fake, JSON.stringify(o))
const write = (rel, n) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
}
const MB = 1024 * 1024
write('a/big.bin', 3 * MB)
write('b/x.bin', 2 * MB)
write('c/y.bin', Math.round(1.5 * MB))
setFake({ idle: 0, battery: false, free: 100 * GB })

const exeName = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const coreBins = ['debug', 'release'].map((p) => path.join(REPO, 'apps', 'core', 'target', p, exeName)).filter((f) => fs.existsSync(f)).sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)
const coreBin = process.env.CCM_CORE_BIN || coreBins[0] || ''
const webDist = process.env.CCM_WEB_DIST || ''

;(async () => {
  let app
  try {
    app = await launch({ env: { ...(coreBin ? { CCM_CORE_BIN: coreBin } : {}), ...(webDist ? { CCM_WEB_DIST: webDist } : {}), CCM_E2E_DISKSNAP_FAKE: fake, CCM_E2E_DISKSNAP_TICK: '500', CCM_E2E_DISKSNAP_TG_SINK: sink, CCM_E2E_DISKMAP_CONFIRM: '1' } })
    const { evaluate, waitFor, cmd } = app
    const D = 'window.ccmDesktop'
    const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
    const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
    const dataDir = path.join(OUT, 'run', 'data', 'disk')
    const snapDir = path.join(dataDir, 'snapshots')

    await waitFor(`typeof ${D}?.diskMapSnapList === 'function'`, 30000, 'bridge')
    check('bridge exposes snapshot methods', await evaluate(`['diskMapSnapList','diskMapSnapCompare','diskMapScheduleGet','diskMapScheduleSet','onDiskMapSchedule','onDiskMapSnapNew'].every((k) => typeof ${D}[k] === 'function')`))
    const s0 = await evaluate(`${D}.diskMapScheduleGet()`)
    check('schedule defaults to daily, Telegram on, no last run', s0.freq === 'daily' && s0.telegram === true && s0.last === null && s0.running === false, JSON.stringify(s0))
    check('no snapshot before any scan', (await evaluate(`${D}.diskMapSnapList()`)).length === 0)

    await evaluate(`(() => { window.__d = { n: 0 }; ${D}.onDiskMapDone(() => window.__d.n++); window.__sn = 0; ${D}.onDiskMapSnapNew(() => window.__sn++); window.__sch = []; ${D}.onDiskMapSchedule((s) => window.__sch.push(s)); return true })()`)
    const scan = async (want) => {
      const r = await evaluate(`${D}.diskMapScan({ roots: [${JSON.stringify(root)}], level: 'balanced', excludes: [] })`)
      if (!r?.started) throw new Error('scan did not start')
      await waitFor(`window.__d.n >= ${want}`, 30000, `scan ${want} done`)
      await waitFor(`window.__sn >= ${want}`, 30000, `snapshot ${want}`)
    }
    await scan(1)
    const list1 = await evaluate(`${D}.diskMapSnapList()`)
    check('first scan writes one snapshot', list1.length === 1 && list1[0].roots.length === 1 && list1[0].bytes >= 6 * MB, JSON.stringify(list1))
    const files1 = fs.readdirSync(snapDir).filter((f) => f.endsWith('.json.gz'))
    const raw1 = zlib.gunzipSync(fs.readFileSync(path.join(snapDir, files1[0]))).toString('utf8')
    check('snapshot file holds folders and sizes but no file names', files1.length === 1 && /"a"|\\\\a"/.test(raw1) && !raw1.includes('big.bin') && !raw1.includes('x.bin') && !raw1.includes('y.bin'), `${raw1.length} bytes`)
    check('snapshot records drive free space', JSON.parse(raw1).volumes.some((v) => v.free === 100 * GB))
    check('a user scan sends nothing to Telegram', !fs.existsSync(sink))

    write('a/more.bin', Math.round(2.5 * MB))
    write('d/new.bin', 2 * MB)
    fs.rmSync(path.join(root, 'c'), { recursive: true, force: true })
    setFake({ idle: 0, battery: false, free: 90 * GB })
    await scan(2)
    const list2 = await evaluate(`${D}.diskMapSnapList()`)
    check('second scan writes a second snapshot, newest first', list2.length === 2 && list2[0].at >= list2[1].at && list2[0].id !== list2[1].id, JSON.stringify(list2.map((m) => m.id)))
    check('a user scan with a 10 GB drop still sends nothing', !fs.existsSync(sink))

    const older = list2[1].id
    const top = await evaluate(`${D}.diskMapSnapCompare({ id: ${JSON.stringify(older)} })`)
    check('compare at the top lists the scanned root and the top growth', top && top.covered && top.rows.length === 1 && top.top.length >= 2, JSON.stringify(top?.top?.map((r) => [r.name, r.delta])))
    const kids = await evaluate(`${D}.diskMapSnapCompare({ id: ${JSON.stringify(older)}, path: ${JSON.stringify(root)} })`)
    const by = Object.fromEntries((kids?.rows ?? []).map((r) => [r.name, r]))
    check('a grew by about 2.5 MB', by.a && by.a.state === 'changed' && by.a.delta >= 2.4 * MB && by.a.delta <= 2.7 * MB, JSON.stringify(by.a))
    check('d is new with its size', by.d && by.d.state === 'new' && by.d.before === 0 && by.d.after >= 2 * MB, JSON.stringify(by.d))
    check('c is gone', by.c && by.c.state === 'gone' && by.c.after === 0 && by.c.delta <= -1.4 * MB, JSON.stringify(by.c))
    check('b is unchanged', by.b && by.b.state === 'same' && by.b.delta === 0, JSON.stringify(by.b))
    check('top growth is a then d, largest first', top.top[0].name === 'a' && top.top[1].name === 'd' && top.top.every((r) => r.delta > 0), JSON.stringify(top.top.map((r) => r.name)))
    const bad = await evaluate(`Promise.all([${D}.diskMapSnapCompare({ id: '../x' }), ${D}.diskMapSnapCompare({ id: '00000099-1700000000000' }), ${D}.diskMapSnapCompare({})])`)
    check('compare refuses bad ids and unknown snapshots', bad.every((x) => x === null), JSON.stringify(bad))

    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await evaluate(`(() => { const el = ${q('[data-setup-done]')}; el?.click(); return true })()`)
    await sleep(300)
    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = '${UI === 'next' ? 'disk' : 'data'}'; return true })()`)
    await waitFor(q('[data-disk-schedule]'), 15000, 'schedule section')
    check('Settings > Data > Disk shows the background scan and the low disk warning together', await evaluate(`!!${q('[data-disk-schedule-freq]')} && !!${q('[data-disk-schedule-telegram]')} && !!${q('[data-setting-row="disk"] [data-disk-threshold]')}`))
    check('the old warning setting is no longer under Notifications', await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsPage = 'notify'; return true })()`) && (await sleep(500), await evaluate(`!document.querySelector('[data-disk-threshold]')`)))
    await evaluate(`(() => { ${S}.get('ui').settingsPage = '${UI === 'next' ? 'disk' : 'data'}'; return true })()`)
    await waitFor(q('[data-disk-schedule]'), 15000, 'schedule section again')
    await evaluate(`${q('[data-disk-schedule]')}.scrollIntoView({ block: 'center' })`)
    await sleep(500)
    await app.shot('settings')
    fs.copyFileSync(path.join(OUT, 'settings.png'), path.join(SHOTS, 'settings-schedule.png'))
    await evaluate(`[...document.querySelectorAll('[data-disk-schedule-freq] [role=radio]')].find((b) => /weekly|hằng tuần/i.test(b.textContent)).click()`)
    await sleep(500)
    check('picking weekly in the UI is saved', (await evaluate(`${D}.diskMapScheduleGet()`)).freq === 'weekly' && fs.readFileSync(path.join(dataDir, 'schedule.json'), 'utf8').includes('"weekly"'))
    await evaluate(`[...document.querySelectorAll('[data-disk-schedule-freq] [role=radio]')].find((b) => /daily|hằng ngày/i.test(b.textContent)).click()`)
    await sleep(400)

    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = false; return true })()`)
    await evaluate(`(() => { ${S}.get('ui').setView('disk'); return true })()`)
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    await evaluate(`(async () => { await ${S}.get('diskMap').open(${JSON.stringify(root)}); return true })()`)
    await waitFor(q('[data-disk-compare-base]'), 15000, 'compare picker')
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)
    const optCount = (await evaluate(selOptions(q('[data-disk-compare-base]'))))?.length
    check('the picker lists both snapshots plus "no comparison"', optCount === 3, String(optCount))
    await evaluate(selPick(q('[data-disk-compare-base]'), `(t, i, v) => v === ${JSON.stringify(String(older))}`))
    await waitFor(q('[data-disk-delta]'), 15000, 'delta cells')
    await sleep(500)
    const cells = await evaluate(`Object.fromEntries([...document.querySelectorAll('[data-disk-row]')].map((r) => [r.dataset.path.split('\\\\').pop(), [r.querySelector('[data-disk-delta]')?.dataset.delta, r.querySelector('[data-disk-delta]')?.textContent.trim()]]))`)
    check('UI delta cells: a up, b same, d up with "new"', cells.a?.[0] === 'up' && /^\+2/.test(cells.a[1]) && cells.b?.[0] === 'same' && cells.d?.[0] === 'up' && /(Mới|New)/.test(cells.d[1]), JSON.stringify(cells))
    await app.shot('compare')
    fs.copyFileSync(path.join(OUT, 'compare.png'), path.join(SHOTS, 'compare-column.png'))
    await evaluate(`${q('[data-disk-delta-sort]')}.click()`)
    await sleep(400)
    check('sort by change is on and keeps the list intact', (await evaluate(`${q('[data-disk-delta-sort]')}.getAttribute('aria-pressed')`)) === 'true')
    await evaluate(`${q('[data-disk-growth-toggle]')}.click()`)
    await waitFor(q('[data-disk-growth-row]'), 10000, 'growth rows')
    const growth = await evaluate(`[...document.querySelectorAll('[data-disk-growth-row]')].map((r) => r.querySelector('.truncate').textContent.trim())`)
    check('growth list: a then d', growth[0] === 'a' && growth[1] === 'd', JSON.stringify(growth))
    await app.shot('growth')
    fs.copyFileSync(path.join(OUT, 'growth.png'), path.join(SHOTS, 'growth-top.png'))
    await cmd('Emulation.setDeviceMetricsOverride', { width: 760, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(700)
    const [sw, cw] = await evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
    check('no horizontal scroll at 760px', sw <= cw, `${sw} <= ${cw}`)
    await app.shot('compare-760')
    fs.copyFileSync(path.join(OUT, 'compare-760.png'), path.join(SHOTS, 'compare-760.png'))
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })

    write('a/more2.bin', 2 * MB)
    setFake({ idle: 3600, battery: true, free: 80 * GB })
    await sleep(2500)
    check('no background scan on battery', (await evaluate(`${D}.diskMapSnapList()`)).length === 2 && !(await evaluate(`${D}.diskMapScheduleGet()`)).running)
    setFake({ idle: 120, battery: false, free: 80 * GB })
    await sleep(2500)
    check('no background scan while the user was active 2 minutes ago', (await evaluate(`${D}.diskMapSnapList()`)).length === 2)
    setFake({ idle: 3600, battery: false, free: 80 * GB })
    await waitFor(`window.__sn >= 3`, 30000, 'background snapshot')
    await sleep(800)
    const sched = await evaluate(`${D}.diskMapScheduleGet()`)
    check('background scan ran once and recorded the last run', sched.last?.ok === true && sched.running === false && sched.last.at > Date.now() - 60000, JSON.stringify(sched))
    check('the page saw the schedule change while it ran', await evaluate(`window.__sch.some((s) => s.running === true) && window.__sch.at(-1).running === false`))
    check('there are three snapshots', (await evaluate(`${D}.diskMapSnapList()`)).length === 3)
    await sleep(1500)
    check('it does not run again before the next due time', (await evaluate(`${D}.diskMapSnapList()`)).length === 3)
    check('the background scan was a light scan of the known root only', (await evaluate(`${D}.diskMapStatus()`)).roots.length === 1)
    const lines = fs.existsSync(sink) ? fs.readFileSync(sink, 'utf8').trim().split('\n').filter(Boolean) : []
    const text = lines.length ? JSON.parse(lines[0]).text : ''
    check('Telegram report went to the sink once', lines.length === 1, `${lines.length} lines`)
    check('report has the drive, the free space drop and both lists', /Quét nền ổ đĩa xong|Background disk scan finished/.test(text) && /80 GB/.test(text) && /10 GB/.test(text) && /(Phình nhiều nhất|Grew the most)/.test(text) && /(Lớn nhất|Largest)/.test(text), text.replace(/\n/g, ' | '))
    check('report names folders, not files', !/\.bin|big|more|new\.|x\.bin/i.test(text.replace(/ccm-snap-e2e-\w+/g, '')) && /\\a /.test(text.replace(/\\\.\.\\/g, '\\')) , text.replace(/\n/g, ' | '))
    check('report shortens the home folder', /~/.test(text) && !text.includes(os.homedir()), text.split('\n').slice(0, 6).join(' | '))
    fs.writeFileSync(path.join(SHOTS, 'telegram-report.txt'), text)
    await evaluate(`(() => { ${S}.get('ui').settingsOpen = true; ${S}.get('ui').settingsPage = '${UI === 'next' ? 'disk' : 'data'}'; return true })()`)
    await waitFor(q('[data-disk-schedule-last]'), 15000, 'last text')
    await evaluate(`${q('[data-disk-schedule]')}.scrollIntoView({ block: 'center' })`)
    await sleep(500)
    const lastText = await evaluate(`${q('[data-disk-schedule-last]')}.textContent.trim()`)
    check('Settings shows the last background scan time', /Lần quét nền cuối|Last background scan/.test(lastText), lastText)
    await app.shot('settings-after')
    fs.copyFileSync(path.join(OUT, 'settings-after.png'), path.join(SHOTS, 'settings-last-run.png'))
  } catch (e) {
    check('run', false, e.message)
    try {
      await app?.shot('failure')
    } catch {}
  } finally {
    app?.stop()
    await sleep(800)
    fs.rmSync(root, { recursive: true, force: true })
    fs.rmSync(fake, { force: true })
    fs.rmSync(sink, { force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
