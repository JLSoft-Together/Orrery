const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4557'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9557'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-chat-view')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-chat-')))
const CFG = path.join(BASE, 'claude')
const WORK = path.join(BASE, 'work')
const SID = 'e2e-chat-0f0ce96d'
const SECRET = `ghp_${'A1b2C3d4E5'.repeat(4)}`
fs.mkdirSync(path.join(WORK, 'src'), { recursive: true })
fs.writeFileSync(path.join(WORK, 'src', 'api.ts'), 'export const retry = 500\n')
const DIR = path.join(CFG, 'projects', WORK.replace(/[^A-Za-z0-9]/g, '-'))
fs.mkdirSync(DIR, { recursive: true })
const FILE = path.join(DIR, `${SID}.jsonl`)
const TS = '2026-11-20T01:00:00.000Z'
const line = (v) => JSON.stringify(v) + '\n'
const prompt = (uuid, text) => line({ type: 'user', uuid, timestamp: TS, isSidechain: false, cwd: WORK, sessionId: SID, message: { role: 'user', content: text } })
const say = (id, text) => line({ type: 'assistant', timestamp: TS, cwd: WORK, sessionId: SID, message: { id, model: 'm', content: [{ type: 'text', text }], stop_reason: 'end_turn' } })
const tool = (id, tid, name, input) => line({ type: 'assistant', timestamp: TS, cwd: WORK, sessionId: SID, message: { id, model: 'm', content: [{ type: 'tool_use', id: tid, name, input }], stop_reason: 'tool_use' } })
const result = (tid) => line({ type: 'user', timestamp: TS, cwd: WORK, sessionId: SID, message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: tid, is_error: false, content: 'ok' }] } })
fs.writeFileSync(
  FILE,
  prompt('u1', `Fix retry in the api, token ${SECRET}`) +
    tool('a1', 't1', 'Read', { file_path: path.join(WORK, 'src', 'api.ts') }) +
    result('t1') +
    tool('a2', 't2', 'Edit', { file_path: path.join(WORK, 'src', 'api.ts'), old_string: 'retry = 100', new_string: 'retry = 500' }) +
    result('t2') +
    tool('a3', 't3', 'Bash', { command: 'npm test' }) +
    result('t3') +
    say('a4', 'Retry is now **500ms**.\n\n```ts\nexport const retry = 500\n```'),
)

async function run(ui) {
  const app = await launch({ ui, env: { CLAUDE_CONFIG_DIR: CFG, CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'; true`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(WORK)}, undefined, { tab: true }); true`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 25000, 'shell pane')
    await sleep(500)
    const key = await evaluate(`(() => {
      ${T}.upsert({ id: 'e2e-chat-a', title: 'Chat', status: 'idle', cwd: ${JSON.stringify(WORK)}, claudeSessionId: ${JSON.stringify(SID)} })
      const p = ${W}.panes.find((x) => (!x.kind || x.kind === 'term') && x.state === 'running')
      ${W}.update(p.key, { preset: 'claude', terminalId: 'e2e-chat-a' })
      return p.key
    })()`)
    const PANE = `document.querySelector('[data-pane="${key}"]')`
    const TOGGLE = `document.querySelector('[data-pane-of="${key}"] [data-pane-chat-toggle]')`
    ok('toggle shows on a Claude pane header', await wait(`!!${TOGGLE}`, 8000, 'toggle'))
    await evaluate(`${TOGGLE}.click(); true`)
    ok('chat view opens over the terminal', await wait(`!!${PANE}?.querySelector('[data-pane-chat] [data-chat-view]')`, 8000, 'chat view'))
    ok('toggle is pressed', (await evaluate(`${TOGGLE}.getAttribute('aria-pressed')`)) === 'true')
    ok('owner question shows', await wait(`(${PANE}.querySelector('[data-chat-me]')?.textContent ?? '').includes('Fix retry in the api')`, 15000, 'question'))
    const meText = await evaluate(`${PANE}.querySelector('[data-chat-me]')?.textContent ?? ''`)
    ok('secret is masked', !meText.includes(SECRET), meText.slice(0, 120))
    ok('agent answer shows as markdown', (await evaluate(`${PANE}.querySelector('[data-chat-ai] strong')?.textContent`)) === '500ms')
    ok('code block has a Copy button', await evaluate(`!!${PANE}.querySelector('[data-chat-code] [data-chat-copy]')`))
    const toolLine = await evaluate(`${PANE}.querySelector('[data-chat-tools-toggle]')?.textContent.replace(/\\s+/g, ' ').trim() ?? ''`)
    ok('tool calls fold into one line', /Read 1 file/.test(toolLine) && /Edited 1 file/.test(toolLine) && /Ran 1 command/.test(toolLine), toolLine)
    ok('tool list starts closed', await evaluate(`!${PANE}.querySelector('[data-chat-tools-list]')`))
    await evaluate(`${PANE}.querySelector('[data-chat-tools-toggle]').click(); true`)
    ok('tool line opens its list', await wait(`(${PANE}.querySelector('[data-chat-tools-list]')?.textContent ?? '').includes('Edit')`, 4000, 'tool list'))
    ok('no input box in chat', await evaluate(`!${PANE}.querySelector('[data-pane-chat] textarea, [data-pane-chat] input')`))
    ok('read only footer', await evaluate(`!!${PANE}.querySelector('[data-chat-readonly]')`))

    fs.appendFileSync(FILE, prompt('u2', 'Now add a test please') + say('a5', 'Test added in api.test.ts.'))
    ok('new turn streams in while open', await wait(`[...${PANE}.querySelectorAll('[data-chat-me]')].some((e) => e.textContent.includes('Now add a test please')) && [...${PANE}.querySelectorAll('[data-chat-ai]')].some((e) => e.textContent.includes('Test added'))`, 15000, 'live'))

    await evaluate(`${TOGGLE}.click(); true`)
    ok('back to terminal hides chat', await wait(`!${PANE}.querySelector('[data-pane-chat]')`, 4000, 'chat closed'))
    ok('terminal is still mounted', await evaluate(`!!${PANE}.querySelector('.xterm')`))
    ok('toggle is released', (await evaluate(`${TOGGLE}.getAttribute('aria-pressed')`)) === 'false')
    if (failed) await shot(`chat-view-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`chat-view-${ui}-error`).catch(() => undefined)
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
  fs.rmSync(BASE, { recursive: true, force: true })
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
