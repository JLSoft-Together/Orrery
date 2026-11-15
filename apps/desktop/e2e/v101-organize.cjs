const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-organize')
process.env.CCM_E2E_PORT ||= '4481'
process.env.CCM_E2E_DBG ||= '9481'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-3b')
const tag = `orrery-e2e-v101org-${process.pid}-${Date.now()}`
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-organize-e2e-'))
const dl = path.join(root, 'Downloads')
const desk = path.join(root, 'Desktop')
const picked = path.join(root, 'Picked')
const junk = path.join(root, 'junk')
for (const d of [dl, desk, picked, junk]) fs.mkdirSync(d, { recursive: true })
const put = (dir, rel, n = 100) => {
  const f = path.join(dir, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 3))
  return f
}
put(dl, 'a.jpg', 1000)
put(dl, 'Ảnh/a.jpg', 10)
put(dl, 'báo cáo tháng.pdf', 2000)
put(dl, 'c.zip', 3000)
put(dl, 'video.mp4.crdownload', 50)
put(dl, 'desktop.ini', 5)

const emptyDir = path.join(junk, `${tag}-empty`)
fs.mkdirSync(emptyDir)
put(junk, 'full/f.txt', 10)
const lnk = path.join(junk, `${tag}-broken.lnk`)
const liveLnk = path.join(junk, `${tag}-live.lnk`)
const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
if (process.platform === 'win32') {
  const gone = put(junk, 'target-gone.txt', 5)
  const live = put(junk, 'target-live.txt', 5)
  const mk = (file, target) => `$s=(New-Object -ComObject WScript.Shell).CreateShortcut('${file.replace(/'/g, "''")}'); $s.TargetPath='${target.replace(/'/g, "''")}'; $s.Save()`
  ps(`${mk(lnk, gone)}; ${mk(liveLnk, live)}`)
  fs.rmSync(gone)
}
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
  const app = await launch({
    env: {
      CCM_E2E_DISKMAP_CONFIRM: '1',
      CCM_E2E_ORGANIZE_STABLE_MS: '3000',
      CCM_E2E_ORGANIZE_DOWNLOADS: dl,
      CCM_E2E_ORGANIZE_DESKTOP: desk,
      CCM_E2E_ORGANIZE_PICK: picked,
    },
  })
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
  const tabByValue = (value) => evaluate(`(() => { const el = document.querySelector('[data-disk-mode] [role=radio][data-value=' + JSON.stringify(${JSON.stringify(value)}) + ']'); if (!el) return false; el.click(); return true })()`)
  const waitFs = async (pred, label, ms = 15000) => {
    const end = Date.now() + ms
    while (Date.now() < end) {
      if (pred()) return true
      await sleep(200)
    }
    throw new Error(label)
  }
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')

  try {
    await waitFor(`typeof ${D}?.organizePlan === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = 'data'; return true })()`)
    await waitFor(q('[data-disk-open]'), 15000, 'disk open button')
    await click('[data-disk-open]')
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    await sleep(800)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    const st0 = await evaluate(`${D}.organizeState()`)
    check('default source is the fake Downloads, never the real one', st0.settings.sources.length === 1 && path.resolve(st0.settings.sources[0]) === path.resolve(dl), JSON.stringify(st0.settings.sources))
    check('auto is off by default', st0.settings.auto === false)
    check('Organize tab is in the mode bar', await tabByValue('organize'))
    await waitFor(q('[data-organize]'), 8000, 'organize tab')
    await waitFor(q('[data-organize-source="downloads"]'), 8000, 'downloads chip')

    const forged = await evaluate(`${D}.organizeRun({ plan: 'nope', from: 'C:\\\\Windows\\\\win.ini', to: ${JSON.stringify(path.join(dl, 'x'))} })`)
    check('IPC refuses a run without a plan made in main', forged.error === 'expired' && forged.moved === 0, JSON.stringify(forged))
    const forgedPlan = await evaluate(`${D}.organizePlan({ source: ${JSON.stringify(os.homedir())} })`)
    check('IPC refuses to plan a folder that is not in the list', forgedPlan.error === 'unknown' && forgedPlan.items.length === 0)

    await sleep(3500)
    await click('[data-organize-preview]')
    await waitFor(q('[data-organize-item]'), 10000, 'plan rows')
    const rows = await evaluate(`[...document.querySelectorAll('[data-organize-item]')].map((r) => [r.dataset.organizeItem, r.querySelector('span.text-ink-muted')?.textContent?.trim() ?? ''])`)
    const names = rows.map((r) => r[0]).sort()
    check('preview lists only the settled top level files', JSON.stringify(names) === JSON.stringify(['a.jpg', 'báo cáo tháng.pdf', 'c.zip'].sort()), JSON.stringify(rows))
    const aRow = rows.find((r) => r[0] === 'a.jpg')
    check('clash gets a (2) suffix', !!aRow && /a \(2\)\.jpg$/.test(aRow[1]), aRow && aRow[1])
    const skipped = await evaluate(`${q('[data-organize-skipped]')}?.textContent ?? ''`)
    check('download in progress and system file are skipped', skipped.includes('video.mp4.crdownload') && skipped.includes('desktop.ini'), skipped.slice(0, 200))
    await evaluate(`(() => { const r = ${q('[data-organize-item="c.zip"]')}; r.querySelector('input').click(); return true })()`)
    await sleep(200)
    await shotTo('preview')
    await click('[data-organize-run]')
    await waitFor(q('[data-organize-result]'), 10000, 'run result')
    check('ticked files moved into subfolders of the source', fs.existsSync(path.join(dl, 'Ảnh', 'a (2).jpg')) && fs.existsSync(path.join(dl, 'Tài liệu', 'báo cáo tháng.pdf')) && !fs.existsSync(path.join(dl, 'a.jpg')))
    check('unticked file stays', fs.existsSync(path.join(dl, 'c.zip')))
    check('existing file in the target is untouched', fs.statSync(path.join(dl, 'Ảnh', 'a.jpg')).size === 10)
    const logs = fs.readdirSync(path.join(OUT, 'run', 'data', 'organize')).filter((n) => n.endsWith('.jsonl'))
    check('run is logged for undo in CCM_DATA_DIR/organize', logs.length === 1, logs.join(','))
    await waitFor(q('[data-organize-undo]'), 5000, 'history undo button')
    await shotTo('after-run')

    await click('[data-organize-undo]')
    await waitFor(q('[data-organize-undo-result]'), 10000, 'undo result')
    check('undo puts every file back', fs.existsSync(path.join(dl, 'a.jpg')) && fs.existsSync(path.join(dl, 'báo cáo tháng.pdf')) && fs.statSync(path.join(dl, 'a.jpg')).size === 1000)
    check('undo removes the folder it created and keeps the one that held files', !fs.existsSync(path.join(dl, 'Tài liệu')) && fs.existsSync(path.join(dl, 'Ảnh', 'a.jpg')))
    await shotTo('after-undo')

    await click('[data-organize-auto] button[role=switch]')
    await waitFor(`${S}.get('organize').settings?.auto === true`, 5000, 'auto on')
    await waitFor(q('[data-organize-watching]'), 5000, 'watching')
    check('auto switch turns watching on', true)
    check('files already there before auto are left alone', fs.existsSync(path.join(dl, 'a.jpg')))
    const dropped = put(dl, 'dropped.png', 400)
    await sleep(1500)
    check('a new file is not moved before it settles', fs.existsSync(dropped))
    await waitFs(() => fs.existsSync(path.join(dl, 'Ảnh', 'dropped.png')), 'auto move')
    check('a new file is moved after it settles', !fs.existsSync(dropped))
    await waitFor(`${S}.get('organize').runs.some((r) => r.auto)`, 8000, 'auto run in history')
    check('auto run shows in history with undo', true)
    await sleep(300)
    await shotTo('auto')
    await click('[data-organize-auto] button[role=switch]')
    await waitFor(`${S}.get('organize').settings?.auto === false`, 5000, 'auto off')

    await click('[data-organize-add]')
    await waitFor(`${S}.get('organize').sources.length === 2`, 5000, 'picked source')
    check('add folder adds the picked folder', true)
    const bad = await evaluate(`${D}.organizeSettings({ source: ${JSON.stringify(os.homedir())} })`)
    check('settings cannot switch to a folder outside the list', path.resolve(bad.settings.source) !== path.resolve(os.homedir()))

    await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(junk)}]; return true })()`)
    await evaluate(`(() => { ${S}.get('organize').active = false; return true })()`)
    await sleep(300)
    await click('[data-disk-scan]')
    await waitFor(`${S}.get('diskMap').indexed && !${S}.get('diskMap').scanning`, 40000, 'index scan')
    await tabByValue('organize')
    await waitFor(q('[data-organize-junk-scan]'), 8000, 'junk section')
    await click('[data-organize-junk-scan]')
    await waitFor(`${S}.get('organize').junkScanned && !${S}.get('organize').junkScanning`, 20000, 'junk scan')
    const items = await evaluate(`${S}.get('organize').junk.map((i) => [i.kind, i.state, i.path])`)
    const emptyHit = items.find((i) => i[0] === 'empty' && path.resolve(i[2]) === path.resolve(emptyDir))
    check('empty folder from the index is listed', !!emptyHit, JSON.stringify(items))
    check('non empty folder is not listed', !items.some((i) => i[2].endsWith('full')))
    if (process.platform === 'win32') {
      check('broken .lnk is listed as broken', items.some((i) => i[0] === 'shortcut' && i[1] === 'broken' && path.resolve(i[2]) === path.resolve(lnk)))
      check('live .lnk is not listed', !items.some((i) => path.resolve(i[2]) === path.resolve(liveLnk)))
    }
    await evaluate(`(() => { const s = ${S}.get('organize'); s.junkPicked = new Set(s.junk.filter((i) => i.path.includes(${JSON.stringify(tag)})).map((i) => i.path)); return true })()`)
    await sleep(200)
    await evaluate(`${q('[data-organize-junk]')}.scrollIntoView({ block: 'center' })`)
    await shotTo('junk')
    await click('[data-organize-junk-remove]')
    await waitFs(() => !fs.existsSync(emptyDir) && (process.platform !== 'win32' || !fs.existsSync(lnk)), 'junk delete', 20000).catch(() => false)
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 1 })`)
    check('ticked junk goes to the Recycle Bin', !fs.existsSync(emptyDir) && (process.platform !== 'win32' || !fs.existsSync(lnk)), JSON.stringify(hist[0] ?? null))
    purge(binItems(tag))

    await cmd('Emulation.setDeviceMetricsOverride', { width: 760, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(500)
    const [s, c] = await sw()
    check('no horizontal scroll at 760px', s <= c, `${s} <= ${c}`)
    await evaluate(`window.scrollTo(0, 0); ${q('[data-organize]')}.scrollIntoView({ block: 'start' })`)
    await shotTo('organize-760')
    await cmd('Emulation.clearDeviceMetricsOverride')
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(800)
    fs.rmSync(root, { recursive: true, force: true })
    purge(binItems(tag))
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
