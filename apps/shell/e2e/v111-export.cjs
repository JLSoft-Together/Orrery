const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4567'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9567'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v111-export')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-export-')))
const CFG = path.join(BASE, 'claude')
const WORK = path.join(BASE, 'work')
const SID = 'e2e-export-0f0ce96d'
const SECRET = `ghp_${'A1b2C3d4E5'.repeat(4)}`
const SHOTS = process.env.CCM_E2E_SHOTS || ''
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
      ${T}.upsert({ id: 'e2e-export-a', title: 'Chat', status: 'idle', cwd: ${JSON.stringify(WORK)}, claudeSessionId: ${JSON.stringify(SID)} })
      const p = ${W}.panes.find((x) => (!x.kind || x.kind === 'term') && x.state === 'running')
      ${W}.update(p.key, { preset: 'claude', terminalId: 'e2e-export-a' })
      return p.key
    })()`)
    const PANE = `document.querySelector('[data-pane="${key}"]')`
    const TOGGLE = `document.querySelector('[data-pane-of="${key}"] [data-pane-chat-toggle]')`
    ok('toggle shows on a Claude pane header', await wait(`!!${TOGGLE}`, 8000, 'toggle'))
    await evaluate(`${TOGGLE}.click(); true`)
    ok('chat view opens', await wait(`!!${PANE}?.querySelector('[data-pane-chat] [data-chat-view]')`, 8000, 'chat view'))
    ok('chat has loaded', await wait(`!!${PANE}.querySelector('[data-chat-me]')`, 15000, 'question'))
    ok('Export button is in the header', await wait(`!!${PANE}.querySelector('[data-chat-header] [data-chat-export-button]')`, 4000, 'export button'))
    await evaluate(`${PANE}.querySelector('[data-chat-export-button]').click(); true`)
    ok('menu opens with Copy and Save', await wait(`!!${PANE}.querySelector('[data-chat-export-menu] [data-chat-export-copy]') && !!${PANE}.querySelector('[data-chat-export-save]')`, 4000, 'menu'))
    ok('Telegram is hidden when not paired', await evaluate(`!${PANE}.querySelector('[data-chat-export-telegram]')`))
    const labels = await evaluate(`[...${PANE}.querySelectorAll('[role=menuitem]')].map((e) => e.textContent.trim())`)
    ok('menu labels', labels.join('|') === 'Copy Markdown|Save as .md file', labels.join('|'))
    await sleep(300)
    if (SHOTS) await shot(`export-menu-${ui}`)
    const got = await evaluate(`window.ccmDesktop.chatView({ op: 'export', key: ${JSON.stringify(key)}, agent: 'claude', session: ${JSON.stringify(SID)}, cwd: ${JSON.stringify(WORK)}, form: 'full', dest: 'text' })`)
    ok('export op answers', got && got.ok === true && typeof got.text === 'string', JSON.stringify(got).slice(0, 120))
    const file = path.join(BASE, `out-${ui}.md`)
    fs.writeFileSync(file, got.text)
    const md = fs.readFileSync(file, 'utf8')
    ok('md has header and both turns', md.startsWith('# Session Claude:') && md.includes('### Bạn') && md.includes('### Claude') && md.includes('Retry is now **500ms**'))
    ok('md folds the tools into one line', /> Tool: .*Read 1 file.*Edit 1 file.*Bash 1 lần/.test(md), md.split(/\r?\n/).find((l) => l.startsWith('> Tool')))
    ok('md lists the changed file', md.includes('## File đã sửa') && /- `src\/api\.ts` \+1 -1/.test(md))
    ok('md masks the secret and home path', !md.includes(SECRET) && !md.includes(os.homedir()))
    const short = await evaluate(`window.ccmDesktop.chatView({ op: 'export', key: ${JSON.stringify(key)}, agent: 'claude', session: ${JSON.stringify(SID)}, cwd: ${JSON.stringify(WORK)}, form: 'short', dest: 'text' })`)
    ok('short form has prompts and files only', short.ok && short.text.startsWith('## Yêu cầu') && short.text.includes('## File đã sửa') && !short.text.includes('Retry is now'))
    await evaluate(`${PANE}.querySelector('[data-chat-export-copy]').click(); true`)
    ok('Copy shows the result', await wait(`${PANE}.querySelector('[data-chat-export-result]')?.getAttribute('data-outcome') === 'copied' || ${PANE}.querySelector('[data-chat-export-result]')?.getAttribute('data-outcome') === 'failed'`, 6000, 'result'))
    const outcome = await evaluate(`${PANE}.querySelector('[data-chat-export-result]')?.getAttribute('data-outcome')`)
    ok('Copy succeeded', outcome === 'copied', String(outcome))
    ok('menu closed after the pick', await evaluate(`!${PANE}.querySelector('[data-chat-export-menu]')`))
    if (failed) await shot(`export-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`export-${ui}-error`).catch(() => undefined)
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
