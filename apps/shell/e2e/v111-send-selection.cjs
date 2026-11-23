const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4551'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9551'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v111-send-selection')
const { launch } = require('./cdp.cjs')
const REPO = path.resolve(__dirname, '../../..')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v111/WP-112j')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const REC = path.join(__dirname, 'fixtures', 'stdin-rec.cjs')
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const SRC = 'export const a = 1\nexport function retry(t: number) {\n  const apiKey = "sk_live_abcdef123456"\n  return t + 1\n}\nexport const z = 2\n'
const SHIFT = 8
const CTRL = 2

async function run(ui) {
  const repo = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-send-')))
  fs.mkdirSync(path.join(repo, 'src'))
  const file = path.join(repo, 'src', 'alpha.ts')
  fs.writeFileSync(file, SRC)
  const recOut = path.join(repo, 'stdin.txt')
  fs.mkdirSync(SHOTS, { recursive: true })
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) } })
  const { evaluate, waitFor, sleep, cmd, key } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(`${ui}: ${s}`, !!v, extra) }
  const until = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const snap = async (name) => {
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    if (r.result?.data) fs.writeFileSync(path.join(SHOTS, `${name}-${ui}.png`), Buffer.from(r.result.data, 'base64'))
  }
  const sent = (before) => fs.readFileSync(recOut, 'utf8').slice(before).replace(/\x1b\[20[01]~/g, '').replace(/\r\n?/g, '\n')
  const selectLines = async (down) => {
    await evaluate(`document.querySelector('.cm-content')?.focus()`)
    await sleep(100)
    await key('Home', 'Home', 36, CTRL)
    for (let i = 0; i < down; i++) await key('ArrowDown', 'ArrowDown', 40, SHIFT)
    await key('End', 'End', 35, SHIFT)
    await sleep(250)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)

    const pane = await evaluate(`${W}.open('shell', ${JSON.stringify(repo)}, undefined, { tab: true }).key`)
    await waitFor(`${W}.panes.find((p) => p.key === '${pane}')?.state === 'running'`, 25000, 'shell running')
    await sleep(1500)
    await evaluate(`[...document.querySelectorAll('.xterm-helper-textarea')].find((t) => t.offsetParent !== null)?.focus()`)
    await app.type(`node ${REC.replace(/\\/g, '/')} ${recOut.replace(/\\/g, '/')}`)
    await sleep(300)
    await key('Enter', 'Enter', 13)
    for (let i = 0; i < 40 && !fs.existsSync(recOut); i++) await sleep(250)
    ok('recorder started in the agent pane', fs.existsSync(recOut))
    await evaluate(`${T}.upsert({ id: 'e2e-ss', title: 'Agent', status: 'idle', cwd: ${JSON.stringify(repo)} })`)
    await evaluate(`${W}.update('${pane}', { preset: 'claude', terminalId: 'e2e-ss', focusedAt: 1 })`)
    await sleep(500)

    await evaluate(`${W}.openFile(${JSON.stringify(file)})`)
    await waitFor(`!!document.querySelector('[data-file-edit]')`, 10000, 'edit button')
    await evaluate(`document.querySelector('[data-file-edit]').click()`)
    await waitFor(`!!document.querySelector('.cm-editor .cm-content')`, 10000, 'editor')
    await sleep(500)
    ok('no floating button without a selection', await evaluate(`!document.querySelector('[data-send-selection]')`))

    await selectLines(2)
    ok('selecting code shows the Send to agent button', await until(`!!document.querySelector('[data-send-selection]')`, 3000, 'chip'))
    ok('button label is translated', /Send to agent/.test(await evaluate(`document.querySelector('[data-send-selection]')?.textContent ?? ''`)))
    ok('button sits inside the editor (no horizontal page scroll)', await evaluate(`(() => { const b = document.querySelector('[data-send-selection]').getBoundingClientRect(); const e = document.querySelector('[data-file-editor]').getBoundingClientRect(); return b.left >= e.left && b.right <= e.right + 1 && b.bottom <= e.bottom + 30 && document.documentElement.scrollWidth <= window.innerWidth })()`))
    await snap('1-chip')

    let before = fs.readFileSync(recOut, 'utf8').length
    await evaluate(`document.querySelector('[data-send-selection]').click()`)
    for (let i = 0; i < 20 && !/retry/.test(sent(before)); i++) await sleep(250)
    await sleep(800)
    let flat = sent(before)
    ok('click puts @path:from-to and the fenced code in the agent', /@src\/alpha\.ts:1-3\n```(typescript|ts)?\nexport const a = 1\nexport function retry\(t: number\) \{\n  const apiKey = /.test(flat), JSON.stringify(flat.slice(0, 200)))
    ok('secret value is masked', !/sk_live_abcdef123456/.test(flat) && /•••/.test(flat), JSON.stringify(flat.slice(-120)))
    ok('prefill only: no Enter after the block', !/\n\n$/.test(flat) && !/\r$/.test(sent(before)))
    ok('toast tells to press Enter in the agent', await until(`/Enter/.test(document.querySelector('[data-send-selection-sent]')?.textContent ?? '')`, 3000, 'toast'))
    ok('agent pane becomes active', (await evaluate(`${W}.activeKey`)) === pane)
    await snap('2-sent')

    await evaluate(`${W}.panes.filter((p) => p.kind === 'file').forEach((p) => ${W}.activate(p.key))`)
    await sleep(400)
    await selectLines(1)
    before = fs.readFileSync(recOut, 'utf8').length
    await key('L', 'KeyL', 76, CTRL | SHIFT)
    for (let i = 0; i < 20 && !/@src\/alpha\.ts:1-2/.test(sent(before)); i++) await sleep(250)
    ok('Ctrl+Shift+L sends the selection', /@src\/alpha\.ts:1-2\n```/.test(sent(before)), JSON.stringify(sent(before).slice(0, 120)))

    await evaluate(`${W}.panes.filter((p) => p.kind === 'file').forEach((p) => ${W}.activate(p.key))`)
    await sleep(400)
    await selectLines(0)
    before = fs.readFileSync(recOut, 'utf8').length
    await evaluate(`document.querySelector('[data-file-editor]').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 300 }))`)
    ok('right click on a selection offers Send to agent', await until(`!!document.querySelector('[data-context-menu] [data-menu-id="send"]')`, 3000, 'menu'))
    await snap('3-menu')
    await evaluate(`document.querySelector('[data-context-menu] [data-menu-id="send"]').click()`)
    for (let i = 0; i < 20 && !/@src\/alpha\.ts:1\n/.test(sent(before)); i++) await sleep(250)
    ok('menu item sends one line as @path:line', /@src\/alpha\.ts:1\n```/.test(sent(before)), JSON.stringify(sent(before).slice(0, 120)))

    await evaluate(`${W}.panes.filter((p) => p.kind === 'file').forEach((p) => ${W}.activate(p.key))`)
    const second = await evaluate(`${W}.open('shell', ${JSON.stringify(repo)}, undefined, { tab: false }).key`)
    await waitFor(`${W}.panes.find((p) => p.key === '${second}')?.state === 'running'`, 25000, 'second shell')
    await evaluate(`${T}.upsert({ id: 'e2e-ss2', title: 'Second', status: 'idle', cwd: ${JSON.stringify(repo)} }); ${W}.update('${second}', { preset: 'codex', terminalId: 'e2e-ss2', focusedAt: 5 }); ${W}.panes.filter((p) => p.kind === 'file').forEach((p) => ${W}.activate(p.key))`)
    await sleep(500)
    await selectLines(0)
    await key('L', 'KeyL', 76, CTRL | SHIFT)
    ok('two agents in the project: asks which one', await until(`document.querySelectorAll('[data-context-menu] [data-menu-id]').length === 2`, 3000, 'pick menu'))
    await snap('4-pick')
    await evaluate(`document.querySelector('[data-context-menu] [data-menu-id="${pane}"]').click()`)
    await sleep(800)
    ok('picked agent becomes active', (await evaluate(`${W}.activeKey`)) === pane)

    if (failed) await snap('error')
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await snap('error').catch(() => undefined)
  } finally {
    app.stop()
  }
}

;(async () => {
  try {
    await run('classic')
    await run('next')
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
