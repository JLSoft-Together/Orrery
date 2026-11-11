const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const A = `${S}.get('agents')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v89p-')))
const BIN = path.join(BASE, 'bin')
const PROJ = path.join(BASE, 'proj')
const CFG = path.join(BASE, 'claude')
const ARGS = path.join(BASE, 'claude-args.txt')
const SID = '3f2b8c1e-5d4a-4e6b-9c7d-1a2b3c4d5e6f'
fs.mkdirSync(BIN, { recursive: true })
fs.mkdirSync(PROJ, { recursive: true })
fs.mkdirSync(CFG, { recursive: true })
fs.writeFileSync(path.join(BIN, 'claude.cmd'), `@echo off\r\necho %* >> "${ARGS}"\r\nping -n 60 127.0.0.1 >nul\r\n`)
fs.writeFileSync(path.join(CFG, '.claude.json'), JSON.stringify({ mcpServers: { 'demo-a': { command: 'node' } }, projects: {} }, null, 2))

async function main() {
  const app = await launch({ env: { CLAUDE_CONFIG_DIR: CFG, CCM_VERIFY_PROCESSES: '0', PATH: `${BIN};${process.env.PATH}` } })
  const { evaluate, waitFor, shot, sleep, key } = app
  const item = (id) => `document.querySelector('[data-context-menu] [data-menu-id="${id}"]')`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.theme','dark'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('settings').contextWarnTokens = 500000`)
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')

    const pane = await evaluate(`${W}.open('shell', ${JSON.stringify(PROJ)}, undefined, { tab: true }).key`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(1000)
    const term = (patch) => evaluate(`${T}.upsert({ id: 'e2e-v89', title: 'e2e-v89', status: 'idle', cwd: ${JSON.stringify(PROJ)}, compactions: 0, ...${JSON.stringify(patch)} })`)
    const agent = (tokens) => evaluate(`${A}.upsert({ id: 'e2e-v89-main', terminalId: 'e2e-v89', role: 'main', status: 'idle', contextTokens: ${tokens} })`)
    await term({})
    await agent(120000)
    await evaluate(`${W}.update('${pane}', { preset: 'claude', terminalId: 'e2e-v89', agentSessionId: '${SID}' })`)
    await sleep(800)

    const chip = `document.querySelector(':is([data-pane="${pane}"], [data-pane-of="${pane}"]) [data-context-tokens], :is([data-pane="${pane}"], [data-pane-of="${pane}"]) [title*="120"]')`
    const chipText = await evaluate(`[...document.querySelectorAll('span[data-pane-chip]')].map(s => s.textContent.trim()).filter(x => /k$/i.test(x)).join(',')`)
    check('pane chip shows 120k', /120\s?k/i.test(chipText), chipText)
    check('no warning under threshold', await evaluate(`!document.querySelector('[data-context-warn]') && !document.querySelector('[data-context-warn-toast]')`))

    await agent(612000)
    await waitFor(`!!document.querySelector('[data-context-warn-toast="e2e-v89"]')`, 5000, 'toast').catch(() => {})
    check('toast at 612k', await evaluate(`!!document.querySelector('[data-context-warn-toast="e2e-v89"]')`))
    check('chip turns warning', await evaluate(`!!document.querySelector('[data-pane-chip][data-context-warn]')`))
    check('toast has Compact action', await evaluate(`[...(document.querySelector('[data-context-warn-toast="e2e-v89"]')?.querySelectorAll('button') ?? [])].some(b => /Compact/.test(b.textContent))`))
    await shot('v89p-01-warn')
    await agent(615000)
    await sleep(400)
    check('toast not repeated', await evaluate(`document.querySelectorAll('[data-context-warn-toast]').length === 1`))
    await evaluate(`[...document.querySelectorAll('[data-context-warn-toast] button')].find(b => /đóng|close|dismiss/i.test(b.getAttribute('aria-label') ?? ''))?.click()`)
    await agent(90000)
    await sleep(300)
    check('chip back to normal after compact', await evaluate(`!document.querySelector('[data-pane-chip][data-context-warn]')`))
    await agent(520000)
    await waitFor(`!!document.querySelector('[data-context-warn-toast="e2e-v89"]')`, 5000, 'toast again').catch(() => {})
    check('re-armed toast after drop', await evaluate(`!!document.querySelector('[data-context-warn-toast="e2e-v89"]')`))
    await agent(120000)

    await evaluate(`document.querySelector('[data-pane-more]').click()`)
    await waitFor(`!!document.querySelector('[data-context-menu]')`, 3000, 'pane menu')
    check('pane ⋯ has MCP & plugin entry', await evaluate(`!!${item('tools')}`))
    await evaluate(`${item('tools')}.click()`)
    await waitFor(`!!document.querySelector('[data-session-tools] [data-tools-row="demo-a"]')`, 15000, 'sheet from pane')
    check('sheet opened for pane dir', await evaluate(`${S}.get('ui').sessionTools?.paneKey === '${pane}'`))
    await evaluate(`document.querySelector('[data-tools-row="demo-a"] [role=switch]').click()`)
    await waitFor(`!!document.querySelector('[data-tools-restart]')`, 5000, 'restart button')
    await term({ status: 'working' })
    await sleep(400)
    check('restart disabled while working', await evaluate(`document.querySelector('[data-tools-restart]').disabled === true`))
    await term({ status: 'idle' })
    await sleep(400)
    check('restart enabled when idle', await evaluate(`document.querySelector('[data-tools-restart]').disabled === false`))
    await shot('v89p-02-sheet-pane')
    await evaluate(`document.querySelector('[data-tools-restart]').click()`)
    let args = ''
    for (let i = 0; i < 30 && !args; i++) { await sleep(500); try { args = fs.readFileSync(ARGS, 'utf8') } catch {} }
    check('restart runs claude --resume <session>', args.includes(`--resume ${SID}`), args.trim())
    const cfg = JSON.parse(fs.readFileSync(path.join(CFG, '.claude.json'), 'utf8'))
    const pk = Object.keys(cfg.projects).find((k) => k.toLowerCase() === PROJ.replace(/\\/g, '/').toLowerCase())
    check('toggle from pane wrote this project', !!pk && cfg.projects[pk].disabledMcpServers?.includes('demo-a'), pk ?? '')

    await evaluate(`${S}.get('ui').sessionTools = null`)
    await evaluate(`${W}.update('${pane}', { terminalId: 'e2e-v89' })`)
    await evaluate(`${S}.get('ui').view = 'monitor'`)
    await waitFor(`!!document.querySelector('[data-board-card="e2e-v89"]')`, 10000, 'card').catch(() => {})
    if (await evaluate(`!!document.querySelector('[data-board-card="e2e-v89"]')`)) {
      check('card chip shows tokens', await evaluate(`/120\\s?k/i.test(document.querySelector('[data-board-card="e2e-v89"]').textContent)`))
      await evaluate(`document.querySelector('[data-board-card="e2e-v89"]').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 300 }))`)
      await waitFor(`!!document.querySelector('[data-context-menu]')`, 3000, 'card menu')
      check('card menu has MCP & plugin entry', await evaluate(`!!${item('card:tools')}`))
      await evaluate(`${item('card:tools')}.click()`)
      await waitFor(`!!document.querySelector('[data-session-tools]')`, 10000, 'sheet from card')
      check('card sheet keeps pane key', await evaluate(`${S}.get('ui').sessionTools?.paneKey === '${pane}'`))
      await shot('v89p-03-card')
    } else { await shot('v89p-03-nocard'); check('card visible in monitor', false) }
  } catch (e) {
    check('run', false, String(e?.message ?? e))
    await shot('v89p-fail').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'v89p-results.txt'), results.join('\n'))
    try { fs.rmSync(BASE, { recursive: true, force: true }) } catch {}
    const fails = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fails}/${results.length} pass`)
    process.exit(fails ? 1 : 0)
  }
}
main()
