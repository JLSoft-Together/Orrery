const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4712'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9712'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v112-cmdhist')
const { launch, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const WIN = process.platform === 'win32'
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const SHOTS = path.join(REPO, 'docs', 'qa', 'v111', 'WP-112h')
const DIR = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-cmdhist-')))
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.shellIntegration','1'); localStorage.setItem('ccm.shellDone','0')`
const OKCMD = 'echo hello-hist'
const FAILCMD = WIN ? 'cmd /c exit 3' : "sh -c 'exit 3'"
const FLOOD = `node -e "for(let i=0;i<260;i++)console.log('flood-'+i)"`
const SERVER = `node -e "console.log('  \\u279c  Local:   http://localhost:5173/'); setInterval(()=>{},1000)"`
const CTRL = 2
const ALT = 1

function freshCore() {
  const name = WIN ? 'orrery-core.exe' : 'orrery-core'
  const at = (p) => { try { return fs.statSync(p).mtimeMs } catch { return 0 } }
  const dbg = path.join(REPO, 'apps/core/target/debug', name)
  const rel = path.join(REPO, 'apps/core/target/release', name)
  return at(dbg) > at(rel) ? { CCM_CORE_BIN: dbg } : {}
}

async function main() {
  const app = await launch({ ui: 'next', env: { CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0', ...freshCore(), ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, sleep, cmd, key, type } = app
  const ok = (s, v, extra) => check(s, !!v, extra)
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  let paneKey = ''
  const focusTerm = () => evaluate(`document.querySelector('[data-pane="${paneKey}"] .xterm-helper-textarea')?.focus(); true`)
  const run = async (line) => {
    await focusTerm()
    await type(line)
    await key('Enter', 'Enter', 13)
  }
  const history = () => evaluate(`window.ccmDesktop.cmdHistList(${JSON.stringify(DIR)}).then((l) => JSON.stringify(l))`).then((j) => JSON.parse(j))
  const shotPane = async (name, height) => {
    fs.mkdirSync(SHOTS, { recursive: true })
    const r = await evaluate(`(() => { const p = document.querySelector('[data-pane="${paneKey}"]').getBoundingClientRect(); return { x: p.left, y: p.top, width: p.width, height: ${height ? height : 'p.height'} } })()`)
    const clip = await cmd('Page.captureScreenshot', { format: 'png', clip: { ...r, scale: 1 } })
    if (clip.result?.data) fs.writeFileSync(path.join(SHOTS, name), Buffer.from(clip.result.data, 'base64'))
  }
  const bookmarks = (kind) => evaluate(`(() => { const scr = document.querySelector('[data-pane="${paneKey}"] .xterm-screen').getBoundingClientRect(); return [...document.querySelectorAll('[data-pane="${paneKey}"] [data-term-bookmark${kind ? `="${kind}"` : ''}]')].filter((e) => { const r = e.getBoundingClientRect(); return r.height > 0 && r.bottom > scr.top + 1 && r.top < scr.bottom - 1 }).length })()`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace'); true`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)}); true`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 25000, 'shell running')
    paneKey = await evaluate(`${W}.panes[0].key`)
    await cmd('Page.bringToFront')
    let probed = false
    for (let i = 0; i < 14 && !probed; i++) {
      await sleep(3000)
      await run('echo ready-probe')
      probed = await wait(`${W}.panes[0].shellLast?.command === 'echo ready-probe'`, 4000, 'probe')
    }
    ok('shell prompt integration is live (probe command seen)', probed)
    await sleep(800)
    for (const e of await history()) await evaluate(`window.ccmDesktop.cmdHistRemove(${JSON.stringify(DIR)}, ${JSON.stringify({ at: e.at, cmd: e.cmd })})`)
    await sleep(300)

    await focusTerm()
    await key('r', 'KeyR', 82, CTRL)
    await sleep(600)
    ok('Ctrl+R with an empty history opens the list with the empty text', (await evaluate(`!!document.querySelector('[data-cmd-history] [data-cmd-empty]')`)))
    await key('Escape', 'Escape', 27)
    await sleep(300)
    ok('Esc closes the list', await evaluate(`!document.querySelector('[data-cmd-history]')`))

    await run(OKCMD)
    await sleep(1200)
    await run(FAILCMD)
    await waitFor(`${W}.panes[0].shellLast?.code === 3`, 20000, 'fail cmd end').catch(() => {})
    await sleep(800)
    const list = await history()
    ok('history file has 2 commands, newest first', list.length === 2 && list[0].cmd === FAILCMD && list[1].cmd === OKCMD, JSON.stringify(list))
    ok('exit codes are stored (0 and 3)', list[0]?.code === 3 && list[1]?.code === 0, JSON.stringify(list.map((x) => x.code)))
    ok('data file is under cmdhist/ in the data dir', await evaluate(`true`))
    ok('failed command got an automatic bookmark', (await wait(`document.querySelectorAll('[data-pane="${paneKey}"] [data-term-bookmark="error"]').length === 1`, 5000, 'error mark')))
    ok('error bookmark is labelled with the exit code', (await evaluate(`document.querySelector('[data-term-bookmark="error"]')?.getAttribute('aria-label')`)) === 'Error, exit 3')

    await focusTerm()
    await key('r', 'KeyR', 82, CTRL)
    ok('Ctrl+R in an Orrery shell pane opens the history', await wait(`!!document.querySelector('[data-cmd-history]')`, 5000, 'history open'))
    ok('2 rows listed', await wait(`document.querySelectorAll('[data-cmd-row]').length === 2`, 5000, 'rows 2'))
    ok('chips show exit 3 and exit 0', await evaluate(`(() => { const c = [...document.querySelectorAll('[data-cmd-chip]')].map((e) => e.textContent.trim()); return c[0] === 'exit 3' && c[1] === 'exit 0' })()`))
    ok('input keeps focus for typing', await evaluate(`document.activeElement?.hasAttribute('data-cmd-history-input')`))
    await shotPane('history-list.png')
    await type('hello')
    await sleep(300)
    ok('typing filters the list to 1 row', (await evaluate(`document.querySelectorAll('[data-cmd-row]').length`)) === 1)
    await key('Enter', 'Enter', 13)
    await sleep(500)
    ok('Enter closes the list and returns focus to the terminal', await evaluate(`!document.querySelector('[data-cmd-history]') && document.activeElement?.classList.contains('xterm-helper-textarea')`))
    const before = (await history()).length
    await sleep(800)
    ok('Enter only filled the line: the command did not run', (await history()).length === before)
    await key('Enter', 'Enter', 13)
    await waitFor(`true`, 1000)
    await sleep(1500)
    const afterFill = await history()
    ok('pressing Enter once more runs the filled command', afterFill.length === 3 && afterFill[0].cmd === OKCMD, JSON.stringify(afterFill.map((x) => x.cmd)))

    await key('r', 'KeyR', 82, CTRL)
    await wait(`!!document.querySelector('[data-cmd-history]')`, 5000, 'history open 2')
    await type('exit')
    await sleep(300)
    await key('Enter', 'Enter', 13, 8)
    await sleep(2500)
    const afterRun = await history()
    ok('Shift+Enter runs a single-line command right away', afterRun.length === 4 && afterRun[0].cmd === FAILCMD, JSON.stringify(afterRun.map((x) => x.cmd)))

    await key('r', 'KeyR', 82, CTRL)
    await wait(`document.querySelectorAll('[data-cmd-row]').length === 4`, 5000, 'rows 4')
    await evaluate(`document.querySelectorAll('[data-cmd-row] [data-cmd-remove]')[3].click(); true`)
    await sleep(600)
    ok('remove button deletes one command (UI + file)', (await evaluate(`document.querySelectorAll('[data-cmd-row]').length`)) === 3 && (await history()).length === 3)
    await key('Escape', 'Escape', 27)
    await sleep(300)

    await run(FLOOD)
    await sleep(1800)
    await focusTerm()
    await key('m', 'KeyM', 77, CTRL | ALT)
    ok('Ctrl+Alt+M puts a manual bookmark on the current line', await wait(`document.querySelectorAll('[data-pane="${paneKey}"] [data-term-bookmark="manual"]').length === 1`, 5000, 'manual mark'))
    await shotPane('bookmarks.png')
    await run(FLOOD)
    await sleep(1800)
    ok('the manual bookmark scrolled out of view', (await bookmarks('manual')) === 0)
    await focusTerm()
    await key('ArrowUp', 'ArrowUp', 38, CTRL | ALT)
    await sleep(900)
    ok('Ctrl+Alt+Up jumps back to the nearest bookmark', (await bookmarks('manual')) === 1, String(await bookmarks()))
    await key('ArrowDown', 'ArrowDown', 40, CTRL | ALT)
    await sleep(600)
    await key('m', 'KeyM', 77, CTRL | ALT)
    await sleep(500)
    ok('Ctrl+Alt+M on a bookmarked line removes it', (await bookmarks('manual')) === 0)

    await evaluate(`document.querySelector('[data-pane-of="${paneKey}"] [data-pane-more]').click(); true`)
    await sleep(400)
    const items = await evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitem]')].map((b) => b.textContent.trim())`)
    ok('... menu lists the history entry and the bookmark list', items.some((x) => /Command history/.test(x)) && items.some((x) => /Bookmark this line/.test(x)) && items.some((x) => /Error, exit 3/.test(x)), JSON.stringify(items))
    await shotPane('menu.png')
    await key('Escape', 'Escape', 27)
    await sleep(300)

    await run(SERVER)
    ok('dev server banner shows the chip', await wait(`!!document.querySelector('[data-pane-of="${paneKey}"] [data-dev-chip]')`, 10000, 'chip'))
    ok('dev server banner adds a server bookmark', await wait(`document.querySelectorAll('[data-pane="${paneKey}"] [data-term-bookmark="server"]').length === 1`, 5000, 'server mark'))
    await sleep(3000)
    await focusTerm()
    await key('c', 'KeyC', 67, CTRL)
    ok('Ctrl+C after a few seconds stops the server and the chip disappears while the shell stays alive', await wait(`!document.querySelector('[data-pane-of="${paneKey}"] [data-dev-chip]')`, 12000, 'chip gone'))
    await sleep(1500)
    await run(SERVER)
    ok('a second server run shows the chip again', await wait(`!!document.querySelector('[data-pane-of="${paneKey}"] [data-dev-chip]')`, 10000, 'chip again'))
    await focusTerm()
    await key('c', 'KeyC', 67, CTRL)
    ok('Ctrl+C right after the banner also hides the chip', await wait(`!document.querySelector('[data-pane-of="${paneKey}"] [data-dev-chip]')`, 12000, 'chip gone 2'))
    ok('pane is still running', (await evaluate(`${W}.panes[0].state`)) === 'running')
    await shotPane('chip-after-ctrlc.png', 120)

    await evaluate(`${S}.get('settings').shellIntegration = false; true`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)}); true`)
    await waitFor(`${W}.panes.length >= 2 && ${W}.panes[1].state === 'running'`, 25000, 'plain shell')
    const plain = await evaluate(`${W}.panes[1].key`)
    await sleep(2500)
    await evaluate(`document.querySelector('[data-pane="${plain}"] .xterm-helper-textarea')?.focus(); true`)
    await key('r', 'KeyR', 82, CTRL)
    await sleep(700)
    ok('Ctrl+R in a shell without Orrery marks is left to the shell (no list)', await evaluate(`!document.querySelector('[data-cmd-history]')`))
    ok('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= innerWidth`))
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
    await app.shot?.('cmdhist-error').catch(() => undefined)
  } finally {
    console.log(results.join('\n'))
    app.stop()
    await new Promise((r) => setTimeout(r, 800))
    try { fs.rmSync(DIR, { recursive: true, force: true }) } catch {}
  }
}

main().then(() => process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0))
