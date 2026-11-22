const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4523'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9523'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v110-external-open')
const { launch, SHELL } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

async function run() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-v110-open-'))
  const picked = path.join(root, 'picked')
  const gone = path.join(root, 'gone')
  fs.mkdirSync(picked)
  fs.mkdirSync(gone)
  const app = await launch({ env: { CCM_VERIFY_PROCESSES: '0', CCM_E2E_PICK_FOLDER: picked } })
  const { evaluate, waitFor, shot } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(s, v, extra) }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('connection')`, 30000, 'app ready')
    await evaluate(PREP)
    await waitFor(`${S}.get('connection').state === 'connected'`, 30000, 'ws connected')

    const pick = await evaluate(`${S}.get('connection').pickFolder()`)
    ok('folder.pick is no longer unsupported', pick?.result !== 'unsupported', JSON.stringify(pick))
    ok('folder.pick answers the chosen folder', pick?.result === 'ok' && path.resolve(pick.dir ?? '') === path.resolve(picked), JSON.stringify(pick))

    await evaluate(`${S}.get('connection').addFavorite(${JSON.stringify(gone)}, 'gone'); true`)
    await waitFor(`${S}.get('favorites').list.some((f) => f.label === 'gone')`, 10000, 'favorite added').then(() => ok('favorite added', true), () => ok('favorite added', false))
    fs.rmSync(gone, { recursive: true, force: true })
    await evaluate(`${S}.get('connection').openFavorite(${JSON.stringify(gone)}, 'new'); true`)
    const FAILED = `${S}.get('activity').items.find((e) => e.kind === 'launch.failed' && e.data?.title === 'gone')`
    await waitFor(`!!${FAILED}`, 15000, 'launch result').then(() => ok('favorite.open reaches the launcher', true), () => ok('favorite.open reaches the launcher', false))
    const err = await evaluate(`${FAILED}?.data?.error ?? null`)
    ok('a deleted favorite folder fails as notFound', err === 'notFound', JSON.stringify(err))
    if (failed) await shot('v110-external-open')
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot('v110-external-open-error').catch(() => undefined)
  } finally {
    app.stop()
    fs.rmSync(root, { recursive: true, force: true })
  }
}

;(async () => {
  if (!SHELL) {
    console.log('SKIP v110-external-open: Tauri only')
    process.exit(0)
  }
  try {
    await run()
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
