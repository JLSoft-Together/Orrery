const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-diskmap-fast')
process.env.CCM_E2E_PORT ||= '4473'
process.env.CCM_E2E_DBG ||= '9473'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-1d')
const FIXTURE = path.join(REPO, 'apps', 'core', 'crates', 'core-disk', 'tests', 'fixtures', 'mft-sample.bin')
const exe = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const bins = ['debug', 'release'].map((p) => path.join(REPO, 'apps', 'core', 'target', p, exe)).filter((f) => fs.existsSync(f))
bins.sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)

;(async () => {
  if (process.platform !== 'win32') {
    console.log('SKIP Windows only')
    return
  }
  const app = await launch({ env: { CCM_MFT_FIXTURE: FIXTURE, ...(bins[0] ? { CCM_CORE_BIN: bins[0] } : {}) } })
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

  try {
    await waitFor(`typeof ${D}?.diskMapFastScan === 'function'`, 30000, 'bridge')
    check('preload exposes diskMapFastScan', true)
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    const bad = await evaluate(`${D}.diskMapFastScan({ drive: 'C:\\\\Windows' })`)
    check('bridge refuses a path that is not a drive letter', bad.started === false && bad.error === 'unsupported', JSON.stringify(bad))

    await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = 'data'; return true })()`)
    await waitFor(q('[data-disk-open]'), 15000, 'settings disk section')
    await click('[data-disk-open]')
    await waitFor(q('[data-disk]'), 15000, 'disk view')
    await sleep(1200)
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)

    const vols = await evaluate(`${S}.get('diskMap').volumes.map((v) => ({ mount: v.mount, fs: v.fs, network: v.network, kind: v.kind }))`)
    const ntfs = vols.find((v) => v.fs === 'NTFS' && !v.network && /^[A-Z]:\\$/.test(v.mount))
    const other = vols.find((v) => v.fs !== 'NTFS' || v.network)
    check('an NTFS volume is listed', !!ntfs, JSON.stringify(vols))
    if (other) {
      await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(other.mount)}]; return true })()`)
      await sleep(300)
      check('no fast scan button for a non NTFS or network drive', !(await evaluate(`!!${q('[data-disk-fastscan]')}`)), other.mount + ' ' + other.fs)
    }
    await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(ntfs.mount)}]; return true })()`)
    await waitFor(q('[data-disk-fastscan]'), 5000, 'fast scan button')
    const hint = await evaluate(`${q('[data-disk-fasthint]')}?.textContent ?? ''`)
    check('fast scan button and admin note show for one NTFS drive', /admin/i.test(hint), hint)
    await evaluate(`${q('[data-disk-scanbar]')}.scrollIntoView({ block: 'center' })`)
    await sleep(300)
    await shotTo('fastscan-button')
    await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(ntfs.mount)}, ${JSON.stringify(other?.mount ?? 'X:\\')}]; return true })()`)
    await sleep(300)
    check('button hides when two drives are ticked', !(await evaluate(`!!${q('[data-disk-fastscan]')}`)))
    await evaluate(`(() => { const s = ${S}.get('diskMap'); s.roots = [${JSON.stringify(ntfs.mount)}]; return true })()`)
    await waitFor(q('[data-disk-fastscan]'), 5000, 'fast scan button again')

    check('fast scan button clicked', await click('[data-disk-fastscan]'))
    await waitFor(`${S}.get('diskMap').indexed && !${S}.get('diskMap').scanning && !${S}.get('diskMap').fastRunning`, 30000, 'fast scan done')
    const st = await evaluate(`${D}.diskMapStatus()`)
    check('index root is the drive', st.roots.length === 1 && st.roots[0] === ntfs.mount, JSON.stringify(st.roots))
    check('entries come from the MFT fixture', (st.entries ?? 0) > 5 && (st.entries ?? 0) < 50, String(st.entries))
    const drive = ntfs.mount.slice(0, 2)
    const kids = await evaluate(`${D}.diskMapChildren({ path: ${JSON.stringify(drive + '\\Users\\jin')}, sort: 'size', limit: 10 })`)
    check('fixture folders are browsable', kids?.items?.map((i) => i.name).join(',') === 'a.bin,Ảnh dài tên.jpg,link1.dat', JSON.stringify(kids?.items?.map((i) => i.name)))
    check('no fast scan error note', !(await evaluate(`!!${q('[data-disk-fasterror]')}`)))
    await waitFor(q('[data-disk-row]'), 8000, 'tree rows')
    await sleep(500)
    await shotTo('fastscan-done')
    const [s1, c1] = await sw()
    check('no horizontal scroll at 1400px', s1 <= c1, `${s1}/${c1}`)
    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(500)
    const [s2, c2] = await sw()
    check('no horizontal scroll at 900px', s2 <= c2, `${s2}/${c2}`)
  } catch (err) {
    check('run', false, err instanceof Error ? err.message : String(err))
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    const failed = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
