const fs = require('node:fs')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')

const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const I = `${S}.get('inbox')`
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v104/WP-104e')
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}
const CTRL = 2, ALT = 1, SHIFT = 8

;(async () => {
  const app = await launch()
  const { evaluate, waitFor, shot, key, sleep } = app
  fs.mkdirSync(SHOTS, { recursive: true })
  const keep = async (name) => {
    await shot(name)
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const groups = () => evaluate(`[...document.querySelectorAll('[data-inbox] [data-inbox-group]')].map((g) => g.dataset.inboxGroup)`)
  const rowCount = () => evaluate(`document.querySelectorAll('[data-inbox] [data-inbox-row]').length`)
  const chipN = () => evaluate(`Number(document.querySelector('[data-inbox-chip]')?.innerText.match(/Inbox (\\d+)/)?.[1] ?? 0)`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.theme','orrery-light'); localStorage.setItem('ccm.themeMigration','4.0.2-galaxy'); localStorage.removeItem('ccm.inbox')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    for (let i = 0; i < 4; i++) await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length === 4 && ${W}.panes.every((p) => p.state === 'running')`, 30000, 'four shells')
    const keys = await evaluate(`${W}.panes.map((p) => p.key)`)
    await evaluate(`(() => {
      const ago = (min) => new Date(Date.now() - min * 60000).toISOString()
      ${T}.upsert({ id: 'e2e-perm', title: 'api: migrate schema', cwd: ${JSON.stringify(REPO)}, status: 'waiting', waitingFor: 'permission prompt', statusSince: ago(3) })
      ${T}.upsert({ id: 'e2e-ask', title: 'web: pick a layout', cwd: ${JSON.stringify(REPO)}, status: 'waiting', waitingFor: 'input needed', statusSince: ago(6) })
      ${T}.upsert({ id: 'e2e-done', title: 'docs: release notes', cwd: ${JSON.stringify(REPO)}, status: 'idle', statusSince: ago(2) })
      const k = ${JSON.stringify(keys)}
      ${W}.update(k[0], { preset: 'claude', terminalId: 'e2e-perm' })
      ${W}.update(k[1], { preset: 'claude', terminalId: 'e2e-ask' })
      ${W}.update(k[2], { preset: 'claude', terminalId: 'e2e-done', attention: 'done' })
      ${W}.update(k[3], { state: 'error' })
      ${W}.activate(k[3])
    })()`)
    await waitFor(`!!document.querySelector('[data-inbox-chip]')`, 8000, 'chip')
    const total = await evaluate(`${I}.count`)
    check('chip shows the Inbox count', (await chipN()) === total && total >= 4, `${await chipN()} / ${total}`)
    check('chip uses the waiting tone with questions open', (await evaluate(`document.querySelector('[data-inbox-chip]').dataset.tone`)) === 'waiting')
    check('no old attention or menu queue chip', !(await evaluate(`!!document.querySelector('[data-attention-chip], [data-menu-queue-chip]')`)))

    await key('A', 'KeyA', 65, CTRL | SHIFT)
    await waitFor(`!!document.querySelector('[data-inbox] [data-inbox-row]')`, 5000, 'panel')
    const order = await groups()
    check('groups follow the shared order', JSON.stringify(order) === JSON.stringify(['permission', 'question', 'error', 'done']), JSON.stringify(order))
    check('chip and panel count the same rows', (await rowCount()) === total && (await chipN()) === total, `${await rowCount()} ${await chipN()} ${total}`)
    check('done row offers View changes', await evaluate(`!!document.querySelector('[data-inbox-group=done] [data-inbox-changes]')`))
    check('no horizontal overflow', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))
    await keep('desktop-inbox-light')
    await evaluate(`${S}.get('settings').theme = 'galaxy'`)
    await sleep(600)
    await keep('desktop-inbox-dark')

    await evaluate(`document.querySelector('[data-inbox] [data-inbox-row="0"]').focus()`)
    await key('s', 'KeyS', 83)
    await sleep(400)
    check('S snoozes the first row and the count drops', (await evaluate(`${I}.count`)) === total - 1 && (await rowCount()) === total - 1, String(await evaluate(`${I}.count`)))
    check('S shows an Undo toast', await evaluate(`[...document.querySelectorAll('[data-toast]')].some((n) => /15/.test(n.textContent))`))
    await key('Escape', 'Escape', 27)
    await sleep(300)
    check('Escape closes the panel', !(await evaluate(`!!document.querySelector('[data-inbox]')`)))
    await evaluate(`${S}.get('inbox').toggle()`)
    await waitFor(`!!document.querySelector('[data-inbox] [data-inbox-row]')`, 5000, 'panel from Ctrl+K')
    check('panel opened without a trigger stays inside the window', await evaluate(`(() => { const r = document.querySelector('[data-inbox]').getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight && r.right <= innerWidth })()`))
    await keep('desktop-inbox-rail')
    await key('Escape', 'Escape', 27)
    await sleep(300)

    const first = await evaluate(`${I}.next()?.paneKey`)
    await key('q', 'KeyQ', 81, CTRL | ALT)
    await sleep(400)
    check('Ctrl+Alt+Q opens the next Inbox item without the panel', (await evaluate(`${W}.activeKey`)) === first && !(await evaluate(`!!document.querySelector('[data-inbox]')`)), `${await evaluate(`${W}.activeKey`)} vs ${first}`)
    const second = await evaluate(`${I}.next()?.paneKey`)
    await key('q', 'KeyQ', 81, CTRL | ALT)
    await sleep(400)
    check('Ctrl+Alt+Q again steps to the item after it', second !== first && (await evaluate(`${W}.activeKey`)) === second, `${await evaluate(`${W}.activeKey`)} vs ${second}`)

    await evaluate(`${W}.remove(${JSON.stringify(keys[3])})`)
    await sleep(300)
    check('closing a pane drops its row', !(await evaluate(`${I}.items.some((i) => i.paneKey === ${JSON.stringify(keys[3])})`)))
  } catch (e) {
    check('run', false, e.message)
    await shot('99-error').catch(() => {})
  } finally {
    await evaluate(`(() => { const w = ${W}; [...w.panes].forEach((p) => w.remove(p.key)); ['e2e-perm','e2e-ask','e2e-done'].forEach((id) => ${T}.remove(id)) })()`).catch(() => {})
    await sleep(1000)
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
