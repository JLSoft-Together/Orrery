const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-qa')
process.env.CCM_E2E_PORT ||= '4495'
process.env.CCM_E2E_DBG ||= '9495'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'QA')
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-qa-e2e-'))
const write = (rel, n) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
}
write('big/huge.mp4', 3_000_000)
write('docs/readme.txt', 1_000)

;(async () => {
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1' } })
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
  const setText = (sel, value) =>
    evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.value = ${JSON.stringify(value)}; el.dispatchEvent(new Event('change', { bubbles: true })); return true })()`)
  const barState = () =>
    evaluate(`(() => {
      const bar = document.querySelector('[data-disk-mode]')
      const r = bar.getBoundingClientRect()
      const radios = [...bar.querySelectorAll('[role=radio]')]
      const rects = radios.map((x) => x.getBoundingClientRect())
      const inside = rects.every((b) => b.left >= r.left - 1 && b.right <= r.right + 1 && b.width > 0)
      const rows = new Set(rects.map((b) => Math.round(b.top))).size
      return { count: radios.length, values: radios.map((x) => x.dataset.value), inside, rows, barRight: Math.round(r.right), viewport: document.documentElement.clientWidth, scrollW: document.documentElement.scrollWidth }
    })()`)

  try {
    await waitFor(`typeof ${D}?.diskMapSearch === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { ${S}.get('settings').locale = 'vi'; ${S}.get('ui').setView('disk'); return true })()`)
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    await sleep(800)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; ${S}.get('diskMap').roots = [${JSON.stringify(root)}]; return true })()`)
    await sleep(400)
    check('scan button clicked', await click('[data-disk-scan]'))
    await waitFor(`${S}.get('diskMap').indexed && !${S}.get('diskMap').scanning && ${S}.get('diskMap').path !== ''`, 40000, 'scan done')
    await waitFor(q('[data-disk-mode]'), 15000, 'mode bar')

    const wide = await barState()
    check('mode bar has the 9 tabs by value', wide.count === 9 && ['browse', 'search', 'big', 'old', 'clean', 'dupes', 'organize', 'apps', 'ram'].every((v) => wide.values.includes(v)), wide.values.join(','))
    check('mode bar at 1400px: one row, inside its box, no page scroll', wide.inside && wide.rows === 1 && wide.scrollW <= wide.viewport, JSON.stringify(wide))
    await shotTo('mode-bar-1400')

    await cmd('Emulation.setDeviceMetricsOverride', { width: 760, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(900)
    const narrow = await barState()
    check('mode bar at 760px: every tab visible inside its box, no page scroll', narrow.inside && narrow.scrollW <= narrow.viewport && narrow.barRight <= narrow.viewport, JSON.stringify(narrow))
    check('mode bar at 760px wraps onto more than one row', narrow.rows > 1, `rows=${narrow.rows}`)
    await shotTo('mode-bar-760')

    await evaluate(`(() => { const el = document.querySelector('[data-disk-mode] [role=radio][aria-checked=true]'); el.focus(); el.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true })); return true })()`)
    await sleep(300)
    check('arrow key moves to the next tab', (await evaluate(`document.querySelector('[data-disk-mode] [role=radio][aria-checked=true]').dataset.value`)) === 'search')
    check('mode bar keeps radiogroup semantics', await evaluate(`document.querySelector('[data-disk-mode]').getAttribute('role') === 'radiogroup' && !!document.querySelector('[data-disk-mode]').getAttribute('aria-label')`))
    await shotTo('mode-bar-760-search')

    check('date field is a text input with the locale pattern', await evaluate(`(() => { const a = ${q('[data-disk-after]')}; const b = ${q('[data-disk-before]')}; return !!a && !!b && a.type === 'text' && a.placeholder === 'dd/mm/yyyy' && b.placeholder === 'dd/mm/yyyy' })()`))
    await setText('[data-disk-after]', '09/03/2020')
    await sleep(300)
    const after = await evaluate(`${S}.get('diskMap').filters.after`)
    check('typed dd/mm/yyyy becomes the filter', after === new Date(2020, 2, 9).getTime(), String(after))
    check('field keeps showing dd/mm/yyyy', (await evaluate(`${q('[data-disk-after]')}.value`)) === '09/03/2020')
    await setText('[data-disk-before]', '2020-03-10')
    await sleep(300)
    const before = await evaluate(`${S}.get('diskMap').filters.before`)
    check('before covers the whole picked day', before === new Date(2020, 2, 10).getTime() + 86_400_000, String(before))
    check('ISO input is shown back in the locale format', (await evaluate(`${q('[data-disk-before]')}.value`)) === '10/03/2020')
    await setText('[data-disk-after]', '31/02/2020')
    await sleep(200)
    check('an impossible date is rejected and the old value comes back', (await evaluate(`${q('[data-disk-after]')}.value`)) === '09/03/2020' && (await evaluate(`${S}.get('diskMap').filters.after`)) === after)
    await setText('[data-disk-after]', '')
    await sleep(200)
    check('emptying the field clears the filter', (await evaluate(`${S}.get('diskMap').filters.after`)) === null)
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(500)
    await shotTo('search-dates-vi')

    await evaluate(`(() => { document.querySelector('[data-disk-mode] [role=radio][data-value="browse"]').click(); return true })()`)
    await waitFor(q('[data-disk-row]'), 10000, 'browse rows')
    const bigRow = await evaluate(`[...document.querySelectorAll('[data-disk-row]')].map((r) => r.dataset.path).find((p) => /[\\\\/]big$/.test(p)) || ''`)
    check('browse lists the temp folder', !!bigRow, bigRow)
    await evaluate(`(() => { const r = [...document.querySelectorAll('[data-disk-row]')].find((e) => e.dataset.path === ${JSON.stringify(bigRow)}); r.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 400 })); return true })()`)
    await waitFor(q('[data-menu-id="move"]'), 5000, 'move entry')
    const label = await evaluate(`${q('[data-menu-id="move"]')}.textContent.trim()`)
    check('folder row menu offers Move to another drive', label === 'Chuyển sang ổ khác', label)
    await shotTo('browse-menu-move')
    await click('[data-menu-id="move"]')
    await waitFor(q('[data-disk-clean]'), 15000, 'clean tab')
    await waitFor(`!!document.querySelector('[data-move-candidate]')`, 15000, 'move candidate')
    const cand = await evaluate(`[...document.querySelectorAll('[data-move-candidate]')].map((e) => e.dataset.moveCandidate)`)
    check('the clicked folder shows as a move candidate in the Clean up tab', cand.some((p) => path.resolve(p) === path.resolve(bigRow)), cand.join(' | '))
    const picked = await evaluate(`(() => { const s = ${S}.get('diskMove'); return { chosen: !!s.chosen, notice: s.notice?.error ?? null } })()`)
    check('the folder is chosen (or the reason it cannot move is shown)', picked.chosen || !!picked.notice, JSON.stringify(picked))
    check('no move was started', (await evaluate(`${S}.get('diskMove').running`)) === false && fs.existsSync(bigRow) && !fs.lstatSync(bigRow).isSymbolicLink())
    await sleep(500)
    await shotTo('move-from-browse')
  } catch (err) {
    check('script finished', false, String(err && err.message ? err.message : err))
    try {
      await shotTo('qa-failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    fs.rmSync(root, { recursive: true, force: true })
  }
  const failed = results.filter((r) => r.startsWith('FAIL'))
  fs.mkdirSync(OUT, { recursive: true })
  fs.writeFileSync(path.join(OUT, 'result.txt'), results.join('\n'))
  console.log(`\n${results.length - failed.length}/${results.length} passed`)
  process.exit(failed.length ? 1 : 0)
})()
