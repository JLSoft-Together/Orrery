const fs = require('node:fs')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }

async function main() {
  const app = await launch({ env: { CCM_VERIFY_PROCESSES: '0', CCM_SHELL_LOG: '1' } })
  const { evaluate, waitFor, shot, sleep } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.removeItem('ccm.bugReport.seen'); localStorage.removeItem('ccm.bugReport.nudge'); localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.theme','dark')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 20000, 'reloaded')
    check('bridge has bugReportPreview + bugReportSave', await evaluate(`typeof window.ccmDesktop.bugReportPreview === 'function' && typeof window.ccmDesktop.bugReportSave === 'function'`))

    const preview = await evaluate(`window.ccmDesktop.bugReportPreview()`)
    const names = (preview?.items ?? []).map((i) => i.name)
    check('preview ok with limit 50 MB', preview?.ok === true && preview.limit === 52428800, JSON.stringify({ ok: preview?.ok, limit: preview?.limit }))
    check('preview lists a shell or pty log', names.some((n) => /logs[\\/](shell|pty)\.log$/.test(n)), names.join(', '))
    check('preview items have kind and bytes', (preview?.items ?? []).every((i) => ['log', 'crash', 'dump', 'report'].includes(i.kind) && Number.isFinite(i.bytes)))

    await sleep(6500)
    const seen = await evaluate(`localStorage.getItem('ccm.bugReport.seen')`)
    check('first run stores the seen set', (() => { try { return Array.isArray(JSON.parse(seen)) } catch { return false } })(), String(seen))
    check('first run shows no crash toast', await evaluate(`!document.querySelector('[data-bug-report-toast]')`))

    await evaluate(`${S}.get('ui').settingsPage = 'about'; ${S}.get('ui').settingsOpen = true`)
    await waitFor(`!!document.querySelector('[data-bug-report]')`, 10000, 'bug report section')
    await waitFor(`document.querySelectorAll('[data-bug-report-item]').length > 0`, 10000, 'items')
    const shown = await evaluate(`[...document.querySelectorAll('[data-bug-report-item]')].map(e => e.dataset.bugReportItem)`)
    check('section lists the preview items', names.length > 0 && names.every((n) => shown.includes(n)), shown.join(', '))
    check('save button enabled, dumps switch off', await evaluate(`(() => { const b = document.querySelector('[data-bug-report-save]'); const s = document.querySelector('[data-bug-report-dumps] [role=switch]'); return !!b && !b.disabled && !!s && s.getAttribute('aria-checked') === 'false' })()`))
    check('total row shown', await evaluate(`!!document.querySelector('[data-bug-report-total]')`))
    check('no horizontal scroll', await evaluate(`document.documentElement.scrollWidth <= window.innerWidth`))
    check('section has no inner horizontal overflow', await evaluate(`(() => { const s = document.querySelector('[data-bug-report]'); return s.scrollWidth <= s.clientWidth + 1 })()`))
    await evaluate(`document.querySelector('[data-bug-report]').scrollIntoView({ block: 'center' })`)
    await sleep(300)
    await shot('v90-01-section')

    await evaluate(`document.querySelector('[data-bug-report-dumps] [role=switch]').click()`)
    await waitFor(`document.querySelector('[data-bug-report-dumps] [role=switch]').getAttribute('aria-checked') === 'true'`, 3000, 'dumps on')
    await shot('v90-02-dumps-on')
    await evaluate(`document.querySelector('[data-bug-report-dumps] [role=switch]').click()`)

    await evaluate(`${S}.get('ui').settingsOpen = false`)
    await sleep(500)
    await evaluate(`window.dispatchEvent(new ErrorEvent('error', { message: 'ResizeObserver loop completed with undelivered notifications.' }))`)
    await sleep(400)
    check('ResizeObserver noise gives no toast', await evaluate(`!document.querySelector('[data-bug-report-toast]')`))
    await evaluate(`window.dispatchEvent(new ErrorEvent('error', { message: 'e2e boom', error: new Error('e2e boom') }))`)
    await waitFor(`!!document.querySelector('[data-bug-report-toast] [data-bug-report-open]')`, 5000, 'ui error toast')
    check('UI error shows one toast with the Send logs action', await evaluate(`document.querySelectorAll('[data-bug-report-toast]').length === 1`))
    await shot('v90-03-toast')
    await evaluate(`document.querySelector('[data-bug-report-open]').click()`)
    await waitFor(`!!document.querySelector('[data-bug-report]')`, 10000, 'section after click')
    await sleep(800)
    check('action opens Settings at the Bug report section', await evaluate(`(() => { const r = document.querySelector('[data-bug-report]').getBoundingClientRect(); return ${S}.get('ui').settingsOpen === true && r.top < window.innerHeight && r.bottom > 0 })()`))
    check('toast closed after the action', await evaluate(`!document.querySelector('[data-bug-report-toast]')`))
    await shot('v90-04-from-toast')

    await evaluate(`${S}.get('ui').settingsOpen = false`)
    await sleep(400)
    await evaluate(`window.dispatchEvent(new ErrorEvent('error', { message: 'e2e second', error: new Error('e2e second') }))`)
    await sleep(600)
    check('max one UI error toast per session', await evaluate(`!document.querySelector('[data-bug-report-toast]')`))
  } catch (e) {
    check('run', false, String(e?.message ?? e))
    await shot('v90-fail').catch(() => {})
  } finally {
    await app.stop()
    fs.writeFileSync(require('node:path').join(OUT, 'v90-results.txt'), results.join('\n'))
    const fails = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fails}/${results.length} pass`)
    process.exit(fails ? 1 : 0)
  }
}
main()
