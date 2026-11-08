// Agent workflow API (2026-11-08): open_session paneId, list_sessions, wait_session, run_log match/since, telegram_send buttons, .orrery/status.json chip.
const fs = require('node:fs')
const http = require('node:http')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const IO = path.join(OUT, 'agent-api')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }
const psq = (s) => `'${s.replace(/'/g, "''")}'`
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function readJson(file, ms = 20000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    try { return JSON.parse(fs.readFileSync(file, 'utf8').replace(/^﻿/, '')) } catch {}
    await sleep(250)
  }
  throw new Error('no file ' + path.basename(file))
}

function request(port, token, body) {
  return new Promise((resolve, reject) => {
    const data = Buffer.from(JSON.stringify(body))
    const headers = { 'Content-Type': 'application/json', 'Content-Length': data.length, Authorization: `Bearer ${token}` }
    const req = http.request({ host: '127.0.0.1', port, method: 'POST', path: '/mcp', headers }, (res) => {
      let text = ''
      res.on('data', (d) => (text += d))
      res.on('end', () => { let json; try { json = JSON.parse(text) } catch {} resolve({ status: res.statusCode, json }) })
    })
    req.on('error', reject)
    req.write(data)
    req.end()
  })
}

const tool = async (port, token, name, args) => (await request(port, token, { jsonrpc: '2.0', id: Date.now(), method: 'tools/call', params: { name, arguments: args } })).json?.result
const sc = (r) => r?.structuredContent

;(async () => {
  fs.rmSync(IO, { recursive: true, force: true })
  fs.mkdirSync(IO, { recursive: true })
  const app = await launch({ exe: process.argv[2] })
  const { evaluate, waitFor, shot, type, key } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await sleep(800)

    const shellKey = await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}).key`)
    await waitFor(`${W}.panes.find(p => p.key === ${JSON.stringify(shellKey)})?.state === 'running'`, 20000, 'shell running')
    await sleep(2500)
    await evaluate(`document.querySelector('[data-pane="${shellKey}"] .xterm-helper-textarea').focus()`)
    const envFile = path.join(IO, 'env.json')
    await type(`@{api=$env:ORRERY_API; token=$env:ORRERY_TOKEN} | ConvertTo-Json | Set-Content -Encoding utf8 ${psq(envFile)}`)
    await key('Enter', 'Enter', 13)
    const env = await readJson(envFile)
    const port = Number(/^http:\/\/127\.0\.0\.1:(\d+)$/.exec(env.api ?? '')?.[1])
    const token = env.token
    check('pane env has api + token', port > 0 && /^[0-9a-f]{64}$/.test(token ?? ''))

    const tools = (await request(port, token, { jsonrpc: '2.0', id: 1, method: 'tools/list' })).json?.result?.tools ?? []
    const def = (n) => tools.find((t) => t.name === n)
    check('tools/list has wait_session', !!def('wait_session'))
    check('open_session schema has submit', !!def('open_session')?.inputSchema?.properties?.submit)
    check('run_log schema has match / ignoreCase / since', ['match', 'ignoreCase', 'since'].every((k) => def('run_log')?.inputSchema?.properties?.[k]))
    check('telegram_send schema has buttons', !!def('telegram_send')?.inputSchema?.properties?.buttons)

    const opened = await tool(port, token, 'open_session', { agent: 'shell', cwd: REPO })
    const paneId = sc(opened)?.paneId
    check('open_session returns paneId', sc(opened)?.ok === true && typeof paneId === 'string' && paneId.length > 0, JSON.stringify(sc(opened)))
    const list = sc(await tool(port, token, 'list_sessions', {}))
    const entry = list?.sessions?.find((s) => s.sessionId === paneId)
    check('list_sessions lists the opened pane under paneId', !!entry && entry.openedByYou === true, JSON.stringify(list?.sessions))

    const t0 = Date.now()
    const timedOut = sc(await tool(port, token, 'wait_session', { session: paneId, until: 'idle', timeoutMs: 3000 }))
    check('wait_session times out on a shell (never idle)', timedOut?.ok === true && timedOut.reached === false && Date.now() - t0 >= 2500, JSON.stringify(timedOut))
    check('wait_session refuses an unknown pane', sc(await tool(port, token, 'wait_session', { session: 'nope' }))?.error === 'target')
    check('wait_session refuses a bad timeout', sc(await tool(port, token, 'wait_session', { session: paneId, timeoutMs: 999999 }))?.error === 'timeoutMs')
    const waiting = tool(port, token, 'wait_session', { session: paneId, until: 'exited', timeoutMs: 30000 })
    await sleep(1000)
    const closed = sc(await tool(port, token, 'close_session', { session: paneId }))
    const exited = sc(await waiting)
    check('wait_session sees the pane exit', closed?.ok === true && exited?.reached === true && exited?.status === 'exited', JSON.stringify({ closed, exited }))

    const badRe = await tool(port, token, 'run_log', { match: '(' })
    check('run_log rejects an invalid regex', sc(badRe)?.error === 'match', JSON.stringify(sc(badRe)))
    check('run_log rejects a malformed cursor', sc(await tool(port, token, 'run_log', { since: 'x' }))?.error === 'since')

    const btn = (n) => Array.from({ length: n }, (_, i) => ({ label: `B${i}`, value: `v${i}` }))
    check('telegram_send refuses 7 buttons', sc(await tool(port, token, 'telegram_send', { text: 'x', buttons: btn(7) }))?.error === 'buttons')
    check('telegram_send refuses buttons + file', sc(await tool(port, token, 'telegram_send', { text: 'x', file: 'README.md', buttons: btn(1) }))?.error === 'buttons-file')
    const unpaired = sc(await tool(port, token, 'telegram_send', { text: 'x', buttons: btn(2) }))
    check('telegram_send with buttons needs a paired owner', unpaired?.ok === false && ['not-paired', 'telegram-off'].includes(unpaired?.error), JSON.stringify(unpaired))

    const work = path.join(IO, 'work')
    fs.mkdirSync(work, { recursive: true })
    const agentKey = await evaluate(`${W}.open('claude', ${JSON.stringify(work)}).key`)
    await waitFor(`${W}.panes.find(p => p.key === ${JSON.stringify(agentKey)})?.state === 'running'`, 30000, 'claude running')
    fs.mkdirSync(path.join(work, '.orrery'), { recursive: true })
    fs.writeFileSync(path.join(work, '.orrery', 'status.json'), JSON.stringify({ phase: 'E2E phase', progress: 0.4, note: 'note‮evil' }))
    const chip = `[...document.querySelectorAll('[data-pane-chip]')].some(e => e.textContent.includes('E2E phase'))`
    const shown = await waitFor(`${S}.get('agentStatus').statusOf(${JSON.stringify(agentKey)})`, 40000, 'status chip').catch(() => null)
    check('status.json reaches the pane', shown?.phase === 'E2E phase' && shown?.progress === 0.4 && !/‮/.test(shown?.note ?? ''), JSON.stringify(shown))
    check('chip rendered in the pane header', !!(await evaluate(`!!${chip}`)))
    await shot('agent-api-01-chip')
    fs.writeFileSync(path.join(work, '.orrery', 'status.json'), JSON.stringify({ phase: 'Second', progress: 1 }))
    const updated = await waitFor(`${S}.get('agentStatus').statusOf(${JSON.stringify(agentKey)})?.phase === 'Second'`, 10000, 'update').catch(() => false)
    check('chip follows file changes', !!updated)
    fs.rmSync(path.join(work, '.orrery', 'status.json'))
    const gone = await waitFor(`!${S}.get('agentStatus').statusOf(${JSON.stringify(agentKey)})`, 10000, 'cleared').catch(() => false)
    check('deleting the file clears the chip', !!gone)
  } catch (e) {
    check('run', false, e.message)
    await shot('agent-api-99-error').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'agent-api-results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
