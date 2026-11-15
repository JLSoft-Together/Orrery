const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-dupes')
process.env.CCM_E2E_PORT ||= '4483'
process.env.CCM_E2E_DBG ||= '9483'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-3a')
const tag = `orrery-e2e-v101dup-${process.pid}-${Date.now()}`
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-dupes-e2e-'))
const DAY = 86_400_000
const MB = 1024 * 1024
const write = (rel, bytes, fill, ageDays) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(bytes, fill))
  const t = new Date(Date.now() - ageDays * DAY)
  fs.utimesSync(f, t, t)
  return f
}
const g1 = [write(`photos/${tag}-holiday.bin`, 1.2 * MB, 11, 400), write(`backup/${tag}-holiday-copy.bin`, 1.2 * MB, 11, 300), write(`downloads/deep/er/${tag}-holiday (1).bin`, 1.2 * MB, 11, 20)]
const g2 = [write(`work/${tag}-report.bin`, 2 * MB, 22, 100), write(`work/old/${tag}-report-v1.bin`, 2 * MB, 22, 100)]
const g3 = [write(`music/${tag}-song.bin`, 1.5 * MB, 33, 60), write(`usb/${tag}-song.bin`, 1.5 * MB, 33, 50)]
const lone = write(`work/${tag}-same-size-other.bin`, 2 * MB, 44, 10)
const tiny = [write(`tiny/${tag}-a.txt`, 2000, 55, 5), write(`tiny/${tag}-b.txt`, 2000, 55, 5)]

const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
const binItems = (name) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $_.Name -like '${name}*' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const purge = (paths) => {
  for (const p of paths) {
    const info = path.join(path.dirname(p), path.basename(p).replace(/^\$R/, '$I'))
    for (const f of [p, info]) fs.rmSync(f, { recursive: true, force: true })
  }
}

;(async () => {
  const coreBin = process.env.CCM_E2E_CORE_BIN
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1', ...(coreBin ? { CCM_CORE_BIN: coreBin } : {}) } })
  const { evaluate, waitFor, shot, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const D = 'window.ccmDesktop'
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  const click = (sel) => evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.click(); return true })()`)
  const shotTo = async (name) => {
    await shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
  const groupsText = () => evaluate(`[...document.querySelectorAll('[data-dupes-group]')].map((r) => r.textContent.replace(/\\s+/g, ' ').trim())`)
  const openAll = () =>
    evaluate(`(async () => { for (let i = 0; i < 6; i++) { const closed = [...document.querySelectorAll('[data-dupes-group] button[aria-expanded=false]')]; if (!closed.length) break; closed.forEach((b) => b.click()); await new Promise((r) => setTimeout(r, 120)) } return true })()`)
  const fileRows = () =>
    evaluate(`[...document.querySelectorAll('[data-dupes-file]')].map((r) => ({ path: r.dataset.path, keep: r.querySelector('[data-dupes-keep]').checked, tick: r.querySelector('[data-dupes-tick]').checked }))`)

  try {
    await waitFor(`typeof ${D}?.diskMapDupes === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = 'data'; return true })()`)
    await waitFor(q('[data-disk-open]'), 15000, 'settings disk section')
    await click('[data-disk-open]')
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    await sleep(1200)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    await evaluate(`(() => { ${S}.get('diskMap').roots = [${JSON.stringify(root)}]; return true })()`)
    await sleep(400)
    await click('[data-disk-scan]')
    await waitFor(`${S}.get('diskMap').indexed && !${S}.get('diskMap').scanning`, 40000, 'scan done')
    const st = await evaluate(`${D}.diskMapStatus()`)
    check('scan covered only the temp folder', st.roots.length === 1 && path.resolve(st.roots[0]) === path.resolve(root), JSON.stringify(st.roots))

    const tabs = await evaluate(`[...document.querySelectorAll('[data-disk-mode] [role=radio]')].map((e) => e.textContent.trim())`)
    check('Disk modes include the duplicates tab', tabs.some((t) => /trùng|Duplicates/i.test(t)), tabs.join(','))
    await evaluate(`(() => { const el = [...document.querySelectorAll('[data-disk-mode] [role=radio]')].find((e) => /trùng|Duplicates/i.test(e.textContent)); el?.click(); return !!el })()`)
    await waitFor(q('[data-dupes]'), 8000, 'dupes panel')
    check('start button is enabled once the index exists', await evaluate(`!${q('[data-dupes-start]')}.disabled`))
    await shotTo('01-before-search')

    await click('[data-dupes-start]')
    await waitFor(`${S}.get('diskDupes').running || ${S}.get('diskDupes').hasResult`, 8000, 'search started')
    await waitFor(q('[data-dupes-groups]'), 40000, 'result summary')
    const store = () => evaluate(`(() => { const s = ${S}.get('diskDupes'); return { groups: s.groups.length, wasted: s.wasted, files: s.plan.files, bytes: s.plan.bytes, error: s.error } })()`)
    let s = await store()
    check('3 duplicate groups found (tiny files and the same-size decoy are not groups)', s.groups === 3 && !s.error, JSON.stringify(s))
    const expectWaste = 1.2 * MB * 2 + 2 * MB + 1.5 * MB
    check('wasted bytes add up', Math.abs(s.wasted - expectWaste) < 4, `${s.wasted} vs ${expectWaste}`)
    check('plan frees the same bytes by default', Math.abs(s.bytes - expectWaste) < 4 && s.files === 4, JSON.stringify(s))
    const heads = await groupsText()
    check('groups sorted by wasted size', heads.length === 3 && /holiday/.test(heads[0]) && /report/.test(heads[1]) && /song/.test(heads[2]), heads.join(' | '))
    await openAll()
    await sleep(300)
    let rows = await fileRows()
    check('every copy is listed after expanding', rows.length === 7, String(rows.length))
    const keepers = rows.filter((r) => r.keep).map((r) => r.path)
    const want = [g1[0], g2[0], g3[0]].map((p) => path.resolve(p))
    check('the oldest copy is kept in each group by default', keepers.length === 3 && want.every((w) => keepers.some((k) => path.resolve(k) === w)), keepers.join(' | '))
    check('exactly the other copies are ticked', rows.filter((r) => r.tick).length === 4 && rows.every((r) => r.keep !== r.tick))
    await shotTo('02-result-expanded')

    const keeperRow = rows.find((r) => path.resolve(r.path) === path.resolve(g1[0]))
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-dupes-file]')].find((e) => e.dataset.path === ${JSON.stringify(keeperRow.path)}); r.querySelector('[data-dupes-tick]').click(); return true })()`)
    await sleep(200)
    rows = await fileRows()
    const still = rows.find((r) => r.path === keeperRow.path)
    check('the last unticked copy cannot be ticked', still && !still.tick && still.keep)
    const other = g1[2]
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-dupes-file]')].find((e) => e.dataset.path === ${JSON.stringify(other)}); r.querySelector('[data-dupes-keep]').click(); return !!r })()`)
    await sleep(200)
    rows = await fileRows()
    const g1rows = rows.filter((r) => g1.some((p) => path.resolve(p) === path.resolve(r.path)))
    check('choosing "keep this copy" moves the keeper', g1rows.filter((r) => r.keep).length === 1 && g1rows.find((r) => r.keep).path === other, JSON.stringify(g1rows))
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-dupes-file]')].find((e) => e.dataset.path === ${JSON.stringify(g1[0])}); r.querySelector('[data-dupes-keep]').click(); return !!r })()`)
    await sleep(200)
    s = await store()
    check('plan is back to the default total', s.files === 4 && Math.abs(s.bytes - expectWaste) < 4, JSON.stringify(s))
    await shotTo('03-keep-changed')

    await click('[data-dupes-trash]')
    await waitFor(q('[data-dupes-report]'), 30000, 'delete report')
    await sleep(500)
    const gone = [g1[1], g1[2], g2[1], g3[1]]
    const kept = [g1[0], g2[0], g3[0], lone, ...tiny]
    const goneWhich = gone.map((p) => !fs.existsSync(p))
    check('ticked copies left the folder', goneWhich.every(Boolean), goneWhich.join(','))
    const keptWhich = kept.map((p) => fs.existsSync(p))
    check('kept copies and unrelated files are untouched', keptWhich.every(Boolean), keptWhich.join(','))
    const inBin = binItems(tag)
    check('removed copies are in the Recycle Bin', process.platform !== 'win32' || inBin.length === 4, inBin.join(' | '))
    s = await store()
    check('list is empty after cleaning', s.groups === 0, JSON.stringify(s))
    const report = await evaluate(`${q('[data-dupes-report]')}?.textContent.replace(/\\s+/g, ' ').trim()`)
    check('report shows the freed size', /MB/.test(report || ''), report)
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 3 })`)
    check('history records the delete as trash', hist.length >= 1 && hist[0].action === 'trash' && hist[0].count === 4, JSON.stringify(hist[0]))
    await shotTo('04-after-clean')
    purge(inBin)

    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(400)
    const [sc, cl] = await sw()
    check('no horizontal scroll at 900px', sc <= cl, `${sc} <= ${cl}`)
    await shotTo('05-narrow-900')
    await cmd('Emulation.clearDeviceMetricsOverride')
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    fs.rmSync(root, { recursive: true, force: true })
    purge(binItems(tag))
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
