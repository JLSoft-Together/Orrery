const fs = require('node:fs')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4433'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9361'
const { launch, OUT } = require('./cdp.cjs')

const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const T = `${S}.get('terminals')`
const ST = `${S}.get('settings')`
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

;(async () => {
  const app = await launch()
  const { evaluate, waitFor, shot, sleep } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.removeItem('ccm.taskbarBadge')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen && ${S}.get('connection').hasData`, 30000, 'reloaded')

    check('bridge has taskbarBadge', await evaluate(`typeof window.ccmDesktop.taskbarBadge === 'function'`))
    check('native badge call resolves', (await evaluate(`window.ccmDesktop.taskbarBadge(2, false).then(() => 'ok', (e) => String(e))`)) === 'ok')
    await evaluate(`window.ccmDesktop.taskbarBadge(0, false)`)
    check('switch defaults on', (await evaluate(`${ST}.taskbarBadge`)) === true)

    await evaluate(`(() => {
      window.__badge = []
      const realFetch = window.fetch.bind(window)
      window.fetch = (url, init) => {
        try {
          const raw = typeof init?.body === 'string' ? init.body : init?.body instanceof ArrayBuffer || ArrayBuffer.isView(init?.body) ? new TextDecoder().decode(init.body) : ''
          if (String(url).includes('ipc') && raw.includes('ccm:badge')) { const j = JSON.parse(raw); window.__badge.push([j.args[0], j.args[1] === true]) }
        } catch {}
        return realFetch(url, init)
      }
      document.hasFocus = () => false
    })()`)
    const wait = (id, since) => `${T}.upsert({ id: '${id}', title: '${id}', cwd: 'F:/x', status: 'waiting', waitingFor: 'permission prompt', statusSince: new Date(Date.now() - ${since}).toISOString() })`
    await evaluate(wait('e2e-w1', 60000))
    await evaluate(wait('e2e-w2', 30000))
    await sleep(600)
    let calls = await evaluate(`window.__badge`)
    check('badge follows the waiting count', calls.at(-1)?.[0] === 2, JSON.stringify(calls))
    check('first items after load do not flash more than once each', calls.every((c) => c[1] === false || c[0] >= 1))
    await evaluate(`window.__badge = []`)
    await evaluate(wait('e2e-w3', 1000))
    await sleep(600)
    calls = await evaluate(`window.__badge`)
    check('a new waiting item in an unfocused window flashes', calls.some((c) => c[0] === 3 && c[1] === true), JSON.stringify(calls))
    await evaluate(`window.__badge = []`)
    await evaluate(`${T}.remove('e2e-w3')`)
    await sleep(600)
    calls = await evaluate(`window.__badge`)
    check('a removed item lowers the count without flashing', calls.some((c) => c[0] === 2 && c[1] === false) && !calls.some((c) => c[1]), JSON.stringify(calls))
    await evaluate(`window.__badge = []`)
    await evaluate(`${ST}.taskbarBadge = false`)
    await sleep(500)
    calls = await evaluate(`window.__badge`)
    check('switch off clears the badge', calls.at(-1)?.[0] === 0 && !calls.some((c) => c[1]), JSON.stringify(calls))
    await evaluate(`${ST}.taskbarBadge = true`)
    await evaluate(`['e2e-w1','e2e-w2'].forEach((id) => ${T}.remove(id))`)

    await evaluate(`(() => {
      window.__notes = []
      window.Notification = class { constructor(title, o) { this.title = title; this.body = o.body; this.tag = o.tag; window.__notes.push({ title, body: o.body, tag: o.tag }) } close() {} static get permission() { return 'granted' } }
      ${ST}.notifyEnabled = true
      ${ST}.notifyDone = true
    })()`)
    const work = (id) => `${T}.upsert({ id: '${id}', title: '${id}', cwd: 'F:/x', status: 'working' })`
    const idle = (id) => `${T}.upsert({ id: '${id}', title: '${id}', cwd: 'F:/x', status: 'idle' })`
    for (const id of ['g1', 'g2', 'g3']) await evaluate(work(id))
    await sleep(400)
    await evaluate(idle('g1'))
    await sleep(300)
    let notes = await evaluate(`window.__notes`)
    check('a single finish keeps its own title', notes.length === 1 && /g1/.test(notes[0].title), JSON.stringify(notes))
    await evaluate(idle('g2'))
    await sleep(300)
    await evaluate(idle('g3'))
    await sleep(300)
    notes = await evaluate(`window.__notes`)
    check('three finishes inside 8 s share one tag', notes.length === 3 && new Set(notes.map((n) => n.tag)).size === 1, JSON.stringify(notes))
    check('merged title counts the sessions', notes.at(-1)?.title === '3 session xong', notes.at(-1)?.title)
    check('merged body lists the names', notes.at(-1)?.body === 'g1, g2, g3', notes.at(-1)?.body)
    await evaluate(`['g1','g2','g3'].forEach((id) => ${T}.remove(id))`)

    await evaluate(`${S}.get('ui').setView('settings')`)
    await sleep(500)
    check('no horizontal overflow', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))
  } catch (e) {
    check('run', false, e.message)
    await shot('99-error').catch(() => {})
  } finally {
    if (results.some((r) => r.startsWith('FAIL'))) await shot('fail').catch(() => {})
    await sleep(500)
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
