const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4512'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9512'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112b')
const { launch, OUT, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const QA = path.join(REPO, 'docs/qa/v111/WP-112b')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.setupDone'); localStorage.removeItem('ccm.setupChip.hidden'); localStorage.removeItem('ccm.tourWorkspaceDone')`

async function main() {
  const app = await launch({ ui: 'classic', env: { CCM_VERIFY_PROCESSES: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const keep = []
  const snap = async (name) => { await shot(name); keep.push(name) }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await waitFor(`!!document.querySelector('[data-ws-footer]')`, 20000, 'footer')
    await sleep(1500)

    check('no setup modal on first run', await evaluate(`!document.querySelector('[data-quick-setup]') && !${S}.get('ui').setupOpen`))
    check('empty state offers the project picker', await evaluate(`!!document.querySelector('[data-launch-agent="claude"]')`))
    check('Claude is the default agent', await evaluate(`${S}.get('workspace').preset === 'claude'`))
    check('setup chip is on the footer', await evaluate(`!!document.querySelector('[data-ws-footer] [data-ws-footer-setup-open]')`))
    check('CPU and RAM stay on the footer', await evaluate(`!!document.querySelector('[data-ws-footer] [data-footer-sys]')`))
    check('tour waits for the first pane', await evaluate(`!${S}.get('ui').tourOpen`))
    await snap('01-first-screen')

    await evaluate(`${S}.get('workspace').launch('shell', ${JSON.stringify(REPO)})`)
    await waitFor(`${S}.get('workspace').panes.length > 0`, 20000, 'pane')
    const tour = await waitFor(`${S}.get('ui').tourOpen === 'workspace' || !!document.querySelector('[data-mist-tips]') || document.body.textContent.includes('Open a tab')`, 8000, 'tour').then(() => true, () => false)
    check('tour starts after the first pane exists', tour)
    await evaluate(`localStorage.setItem('ccm.tourWorkspaceDone','1'); ${S}.get('ui').tourOpen = false`)
    await key('Escape', 'Escape', 27)
    await sleep(1200)
    check('first pane opens', await evaluate(`${S}.get('workspace').panes.length > 0`))
    check('quiet numbers go into the more popover', await evaluate(`!!document.querySelector('[data-ws-footer] [data-ws-footer-more]')`))
    check('quiet chips are not inline', await evaluate(`!document.querySelector('[data-ws-footer] [data-ws-footer-focus]') && !document.querySelector('[data-ws-footer] [data-ws-footer-tokens]')`))
    await snap('02-footer-normal')
    await evaluate(`document.querySelector('[data-ws-footer-more]').click()`)
    await sleep(300)
    check('popover lists the focus row', await evaluate(`!!document.querySelector('[data-ws-footer-popover] [data-ws-more-row="focus"]')`))
    await snap('03-popover-open')
    await key('Escape', 'Escape', 27)
    await sleep(300)
    check('Escape closes the popover', await evaluate(`!document.querySelector('[data-ws-footer-popover]')`))

    await evaluate(`document.querySelector('[data-ws-footer-more]').click()`)
    await sleep(200)
    await evaluate(`document.querySelector('[data-ws-more-row="focus"]').click()`)
    await sleep(400)
    check('focus mode on lifts its chip out', await evaluate(`!!document.querySelector('[data-ws-footer] [data-ws-footer-focus]')`))
    await snap('04-footer-with-chip')
    await evaluate(`document.querySelector('[data-ws-footer] [data-ws-footer-focus]').click()`)
    await sleep(300)

    await evaluate(`document.querySelector('[data-ws-footer-setup-open]').click()`)
    await waitFor(`!!document.querySelector('[data-setup-row]')`, 10000, 'checklist rows')
    const rows = await evaluate(`[...document.querySelectorAll('[data-setup-row]')].map((r) => r.dataset.setupRow).join(',')`)
    check('Settings opens on the checklist', await evaluate(`${S}.get('ui').settingsOpen === true`), rows)
    check('checklist lists the old setup items', ['notify', 'sound'].every((id) => rows.includes(id)), rows)
    check('checklist shows how many are left', await evaluate(`/\\d/.test(document.querySelector('[data-setup-summary]')?.textContent ?? '')`))
    await sleep(500)
    await snap('05-settings-checklist')
    await evaluate(`${S}.get('ui').settingsOpen = false`)
    await sleep(300)

    await evaluate(`document.querySelector('[data-ws-footer-setup-hide]').click()`)
    await sleep(300)
    check('hide removes the chip and remembers it', await evaluate(`!document.querySelector('[data-ws-footer-setup]') && localStorage.getItem('ccm.setupChip.hidden') === '1'`))
    await cmd('Page.reload')
    await waitFor(`!!document.querySelector('[data-ws-footer]')`, 20000, 'footer again')
    await sleep(800)
    check('chip stays hidden after a reload', await evaluate(`!document.querySelector('[data-ws-footer-setup]')`))
  } catch (err) {
    check('script error', false, String(err?.message ?? err))
    await app.shot('error').catch(() => {})
  } finally {
    app.stop()
  }
  fs.mkdirSync(QA, { recursive: true })
  for (const n of keep) fs.copyFileSync(path.join(OUT, n + '.png'), path.join(QA, n + '.png'))
  fs.writeFileSync(path.join(OUT, 'v112b.txt'), results.join('\n') + '\n')
  if (results.some((r) => r.startsWith('FAIL'))) process.exitCode = 1
}
main()
