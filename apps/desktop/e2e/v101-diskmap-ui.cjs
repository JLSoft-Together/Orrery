const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-diskmap-ui')
process.env.CCM_E2E_PORT ||= '4472'
process.env.CCM_E2E_DBG ||= '9472'

const { launch, sleep, OUT, REPO, UI, selPick } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-1c')
const tag = `orrery-e2e-v101c-${process.pid}-${Date.now()}`
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-diskmap-ui-e2e-'))
const MONTH = 30 * 86_400_000
const write = (rel, n, ageMonths = 0) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
  if (ageMonths) {
    const t = new Date(Date.now() - ageMonths * MONTH)
    fs.utimesSync(f, t, t)
  }
}
write('big/huge.mp4', 3_000_000)
write('big/old-video.mkv', 2_000_000, 30)
write('docs/report.pdf', 200_000, 8)
write('docs/readme.txt', 1_000)
write(`trash/${tag}.bin`, 300_000)

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
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1' } })
  const { evaluate, waitFor, shot, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const D = 'window.ccmDesktop'
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  const click = (sel) => evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.click(); return true })()`)
  const setField = (sel, value, ev = 'input') =>
    evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.value = ${JSON.stringify(value)}; el.dispatchEvent(new Event(${JSON.stringify(ev)}, { bubbles: true })); return true })()`)
  const names = () => evaluate(`[...document.querySelectorAll('[data-disk-result]')].map((r) => r.querySelector('span.block')?.textContent?.trim())`)
  const waitResults = async (pred, label) => {
    const end = Date.now() + 8000
    let last = []
    while (Date.now() < end) {
      last = await names()
      if (pred(last)) return last
      await sleep(150)
    }
    throw new Error(`${label}: ${JSON.stringify(last)}`)
  }
  const radio = (sel, i) => evaluate(`(() => { const el = document.querySelectorAll(${JSON.stringify(sel + ' [role=radio]')})[${i - 1}]; if (!el) return false; el.click(); return true })()`)
  const MODE_VALUES = ['browse', 'search', 'big', 'old']
  const mode = (i) => evaluate(`(() => { const el = document.querySelector('[data-disk-mode] [role=radio][data-value="${MODE_VALUES[i - 1]}"]'); if (!el) return false; el.click(); return true })()`)
  const shotTo = async (name) => {
    await shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')

  try {
    await waitFor(`typeof ${D}?.diskMapSearch === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)

    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = '${UI === 'next' ? 'disk' : 'data'}'; return true })()`)
    await waitFor(q('[data-disk-map-settings]'), 15000, 'settings disk section')
    await evaluate(`${q('[data-disk-map-settings]')}.scrollIntoView({ block: 'center' })`)
    await sleep(300)
    await shotTo('settings-data-disk')
    check('Settings > Data shows the Disk section and its open button', await evaluate(`!!${q('[data-disk-open]')} && !${q('[data-disk-open]')}.disabled`))
    await click('[data-disk-open]')
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    check('Disk view opens from Settings', true)
    await sleep(1200)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(root)}]; return true })()`)
    await sleep(500)
    check('scan button is enabled and clicked', await click('[data-disk-scan]'))
    await waitFor(`${S}.get('diskMap').indexed && !${S}.get('diskMap').scanning && ${S}.get('diskMap').path !== ''`, 40000, 'scan done and root opened')
    const st = await evaluate(`${D}.diskMapStatus()`)
    check('scan covered only the temp folder', st.roots.length === 1 && path.resolve(st.roots[0]) === path.resolve(root), JSON.stringify(st.roots))
    await waitFor(q('[data-disk-treemap]'), 10000, 'treemap')
    const rows = await evaluate(`[...document.querySelectorAll('[data-disk-row]')].map((r) => r.dataset.path)`)
    check('treemap and tree have data', rows.length === 3, rows.join(' | '))
    const px = await evaluate(`(() => { const c = ${q('[data-disk-treemap] canvas')} || ${q('[data-disk-treemap]')}; if (!c || !c.getContext) return -1; const w = c.width, h = c.height; const d = c.getContext('2d').getImageData(0, 0, w, h).data; let n = 0; for (let i = 3; i < d.length; i += 4 * 97) if (d[i] > 0) n++; return n })()`)
    check('treemap canvas has drawn pixels', px > 0, String(px))
    await sleep(300)
    await shotTo('browse-treemap')

    const bigRow = rows.find((r) => path.basename(r) === 'big')
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-disk-row]')].find((e) => e.dataset.path === ${JSON.stringify(bigRow)}); r?.querySelector('button')?.click(); return !!r })()`)
    await waitFor(`${S}.get('diskMap').path === ${JSON.stringify(bigRow)}`, 8000, 'enter folder')
    await waitFor(`[...document.querySelectorAll('[data-disk-row]')].some((r) => r.dataset.path.endsWith('huge.mp4'))`, 8000, 'folder rows').catch(() => undefined)
    const inner = await evaluate(`[...document.querySelectorAll('[data-disk-row]')].map((r) => r.dataset.path.split(/[\\\\/]/).pop())`)
    check('click enters a folder and lists its children', inner.includes('huge.mp4') && inner.includes('old-video.mkv'), inner.join(','))

    await mode(2)
    await waitFor(q('[data-disk-search]'), 8000, 'search panel')
    check('search tab shows an idle hint until something is typed', !!(await evaluate(`!!${q('[data-disk-search-idle]')}`)))

    const typeSlow = async (text) => {
      await evaluate(`${q('[data-disk-query]')}.focus()`)
      for (const ch of text) {
        await app.type(ch)
        await sleep(15)
      }
    }
    await typeSlow('*.mkv')
    let got = await waitResults((n) => n.length === 1 && n[0] === 'old-video.mkv', 'glob')
    check('glob *.mkv finds the file', got.length === 1)
    const meta = await evaluate(`[${q('[data-disk-matches]')}?.textContent, ${q('[data-disk-ms]')}?.textContent]`)
    check('total and engine ms are shown', /1/.test(meta[0] || '') && /\d+\s*ms/.test(meta[1] || ''), JSON.stringify(meta))
    await shotTo('search-glob')

    await setField('[data-disk-query]', 'HUGE')
    got = await waitResults((n) => n.length === 1 && n[0] === 'huge.mp4', 'name')
    check('name search is case insensitive', got.length === 1)
    await setField('[data-disk-query]', 'repórt')
    got = await waitResults((n) => n.length === 1 && n[0] === 'report.pdf', 'accent')
    check('name search ignores accents', got.length === 1)
    await setField('[data-disk-query]', 'zzzz-nothing')
    await waitResults((n) => n.length === 0, 'none')
    check('zero matches shows the empty note', !!(await evaluate(`!!${q('[data-disk-results-empty]')}`)))

    await setField('[data-disk-query]', '')
    await evaluate(selPick(q('[data-disk-kind]'), "(t, i, v) => v === 'video'"))
    got = await waitResults((n) => n.length === 2, 'kind video')
    check('kind video finds both videos', got.includes('huge.mp4') && got.includes('old-video.mkv'), got.join(','))
    await setField('[data-disk-min]', '2.5', 'change')
    got = await waitResults((n) => n.length === 1, 'min size')
    check('type video + min 2.5 MB finds only the 3 MB file', got[0] === 'huge.mp4', got.join(','))
    await click('[data-disk-reset]')
    await waitFor(q('[data-disk-search-idle]'), 5000, 'reset')
    await evaluate(selPick(q('[data-disk-only]'), "(t, i, v) => v === 'dir'"))
    await setField('[data-disk-query]', 'big')
    got = await waitResults((n) => n.length === 1 && n[0] === 'big', 'only dir')
    check('only folders returns the folder', got.length === 1)
    await click('[data-disk-reset]')

    await mode(3)
    await setField('[data-disk-min]', '1', 'change')
    got = await waitResults((n) => n.length === 2 && n[0] === 'huge.mp4', 'big files')
    check('Big files sorts by size and applies min size', got.join(',') === 'huge.mp4,old-video.mkv', got.join(','))
    await shotTo('big-files')

    await click('[data-disk-reset]')
    await mode(4)
    const months = await evaluate(`document.querySelectorAll('[data-disk-months] [role=radio]').length`)
    check('Old files offers 4 presets', months === 4, String(months))
    const oldFor = async (idx, pred, label) => {
      await radio('[data-disk-months]', idx)
      return waitResults(pred, label)
    }
    got = await oldFor(4, (n) => n.length === 1 && n[0] === 'old-video.mkv', '24 months')
    check('older than 24 months finds only the 30 month old file', got.length === 1)
    got = await oldFor(3, (n) => n.length === 1 && n[0] === 'old-video.mkv', '12 months')
    check('older than 12 months still excludes the 8 month old file', got.length === 1)
    got = await oldFor(2, (n) => n.length === 2, '6 months')
    check('older than 6 months adds the 8 month old file', got.includes('report.pdf') && got.includes('old-video.mkv'), got.join(','))
    got = await oldFor(1, (n) => n.length === 2, '3 months')
    check('older than 3 months keeps both', got.length === 2)
    await shotTo('old-files')

    await mode(2)
    await setField('[data-disk-query]', tag)
    got = await waitResults((n) => n.length === 1 && n[0] === `${tag}.bin`, 'trash target')
    await click('[data-disk-result] input[type=checkbox]')
    await waitFor(q('[data-disk-selection]'), 5000, 'selection bar')
    await shotTo('search-selected')
    const target = path.join(root, 'trash', `${tag}.bin`)
    await click('[data-disk-trash]')
    await waitFor(q('[data-disk-report]'), 20000, 'delete report')
    await waitResults((n) => n.length === 0, 'results refreshed')
    check('selected result goes to the Recycle Bin and leaves the list', !fs.existsSync(target))
    const inBin = binItems(tag)
    check('file is in the Recycle Bin', process.platform !== 'win32' || inBin.length === 1, inBin.join(' | '))
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 5 })`)
    check('history has the delete', hist.length >= 1 && hist[0].action === 'trash' && hist[0].paths.some((p) => p === target || p.endsWith(`${tag}.bin`)), JSON.stringify(hist[0]))
    await waitFor(`${q('[data-disk-history-empty]')} === null`, 5000, 'history row')
    check('history list shows a row', true)
    purge(inBin)

    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(400)
    for (const [i, label] of [[1, 'browse'], [2, 'search'], [3, 'big'], [4, 'old']]) {
      await mode(i)
      await sleep(400)
      const [s, c] = await sw()
      check(`no horizontal scroll at 900px (${label})`, s <= c, `${s} <= ${c}`)
    }
    await mode(3)
    await sleep(500)
    await shotTo('big-files-900')
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
