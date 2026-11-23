const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4571'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9571'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v111-commit-chat')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-commit-chat-')))
const CFG = path.join(BASE, 'claude')
const WORK = path.join(BASE, 'repo')
const SID = 'e2e-commit-chat-0f0ce96d'
const SHOTS = process.env.CCM_E2E_SHOTS || ''
const NOW = Math.floor(Date.now() / 1000) * 1000
const iso = (ms) => new Date(ms).toISOString()
const MIN = 60_000

fs.mkdirSync(path.join(WORK, 'src'), { recursive: true })
const git = (args, at) => execFileSync('git', args, { cwd: WORK, stdio: 'pipe', env: { ...process.env, GIT_AUTHOR_NAME: 'E2E', GIT_AUTHOR_EMAIL: 'e2e@example.com', GIT_COMMITTER_NAME: 'E2E', GIT_COMMITTER_EMAIL: 'e2e@example.com', ...(at ? { GIT_AUTHOR_DATE: iso(at), GIT_COMMITTER_DATE: iso(at) } : {}) } }).toString()
const write = (rel, text) => fs.writeFileSync(path.join(WORK, rel), text)
git(['init', '-q', '-b', 'main'])
write('README.md', '# repo\n')
git(['add', '.'])
git(['commit', '-q', '-m', 'init'], NOW - 120 * MIN)
write('src/login.ts', 'export const retry = 500\n')
write('src/retry.ts', 'export const wait = 2\n')
git(['add', '.'])
git(['commit', '-q', '-m', 'feat: retry login'], NOW - 15 * MIN)
write('src/login.ts', 'export const retry = 600\n')
write('notes.txt', 'a\n')
write('todo.txt', 'b\n')
git(['add', '.'])
git(['commit', '-q', '-m', 'chore: mixed change'], NOW - 12 * MIN)
write('README.md', '# repo by hand\n')
git(['add', '.'])
git(['commit', '-q', '-m', 'docs: edit readme by hand'], NOW - 5 * MIN)

const DIR = path.join(CFG, 'projects', WORK.replace(/[^A-Za-z0-9]/g, '-'))
fs.mkdirSync(DIR, { recursive: true })
const line = (v) => JSON.stringify(v) + '\n'
const prompt = (uuid, at, text) => line({ type: 'user', uuid, timestamp: iso(at), isSidechain: false, cwd: WORK, sessionId: SID, message: { role: 'user', content: text } })
const say = (id, at, text) => line({ type: 'assistant', timestamp: iso(at), cwd: WORK, sessionId: SID, message: { id, model: 'm', content: [{ type: 'text', text }], stop_reason: 'end_turn' } })
const tool = (id, at, name, input) => line({ type: 'assistant', timestamp: iso(at), cwd: WORK, sessionId: SID, message: { id, model: 'm', content: [{ type: 'tool_use', id, name, input }], stop_reason: 'tool_use' } })
const result = (id, at) => line({ type: 'user', timestamp: iso(at), cwd: WORK, sessionId: SID, message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: id, is_error: false, content: 'ok' }] } })
const first = prompt('u0', NOW - 40 * MIN, 'Hello, look around the repo first') + say('a0', NOW - 39 * MIN, 'The repo has a README only.')
const target = prompt('u1', NOW - 20 * MIN, 'Make login retry 500ms with a wait helper') +
  tool('t1', NOW - 19 * MIN, 'Write', { file_path: path.join(WORK, 'src', 'login.ts'), content: 'export const retry = 500\n' }) +
  result('t1', NOW - 19 * MIN) +
  tool('t2', NOW - 18 * MIN, 'Write', { file_path: path.join(WORK, 'src', 'retry.ts'), content: 'export const wait = 2\n' }) +
  result('t2', NOW - 18 * MIN) +
  say('a1', NOW - 17 * MIN, 'Login now retries after **500ms**.')
const later = prompt('u2', NOW - 3 * MIN, 'Now write the docs') + tool('t3', NOW - 2 * MIN, 'Write', { file_path: path.join(WORK, 'docs.md'), content: 'docs\n' }) + result('t3', NOW - 2 * MIN) + say('a2', NOW - MIN, 'Docs written.')
fs.writeFileSync(path.join(DIR, `${SID}.jsonl`), first + target + later)
const TURN = Buffer.byteLength(first)

function copyShot(name) {
  if (!SHOTS) return
  fs.mkdirSync(SHOTS, { recursive: true })
  fs.copyFileSync(path.join(process.env.CCM_E2E_OUT, `${name}.png`), path.join(SHOTS, `${name}.png`))
}

async function run(ui) {
  const app = await launch({ ui, env: { CLAUDE_CONFIG_DIR: CFG, CODEX_HOME: path.join(BASE, 'codex'), CCM_VERIFY_PROCESSES: '0', CCM_CODEX: '0', CCM_DEEPSEEK: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const name = (s) => `${ui}: ${s}`
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(name(s), v, extra) }
  const wait = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const row = (subject) => `[...document.querySelectorAll('[data-commit]')].find((r) => r.querySelector('[data-log-subject]')?.textContent === ${JSON.stringify(subject)})`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'; true`)
    await evaluate(`${W}.open('shell', ${JSON.stringify(WORK)}, undefined, { tab: true }); true`)
    await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 25000, 'shell pane')
    await sleep(800)
    await evaluate(`${W}.openLog(${JSON.stringify(WORK)}); true`)
    ok('git log lists the commits', await wait(`!!${row('feat: retry login')}`, 20000, 'log rows'))
    ok('chip shows on the agent commit', await wait(`!!${row('feat: retry login')}?.querySelector('[data-commit-chip]')`, 15000, 'chip'))
    const strong = await evaluate(`(() => { const c = ${row('feat: retry login')}.querySelector('[data-commit-chip]'); return c && { kind: c.getAttribute('data-commit-chip'), text: c.textContent.trim(), turn: c.getAttribute('data-turn'), session: c.getAttribute('data-session') } })()`)
    ok('strong chip names the session', strong && strong.kind === 'strong' && /^Claude repo$/.test(strong.text) && strong.session === SID, JSON.stringify(strong))
    ok('chip points at the turn that wrote the files', strong && Number(strong.turn) === TURN, `${strong && strong.turn} vs ${TURN}`)
    const weak = await evaluate(`(() => { const c = ${row('chore: mixed change')}?.querySelector('[data-commit-chip]'); return c && { kind: c.getAttribute('data-commit-chip'), text: c.textContent.trim() } })()`)
    ok('part match is a faint chip with ?', weak && weak.kind === 'weak' && weak.text.endsWith('?'), JSON.stringify(weak))
    ok('commit by hand has no chip', await evaluate(`!${row('docs: edit readme by hand')}?.querySelector('[data-commit-chip]')`))
    ok('first commit has no chip', await evaluate(`!${row('init')}?.querySelector('[data-commit-chip]')`))
    await sleep(300)
    if (SHOTS) {
      await shot(`git-log-chips-${ui}`)
      copyShot(`git-log-chips-${ui}`)
    }
    await evaluate(`${row('feat: retry login')}.querySelector('[data-commit-chip]').click(); true`)
    ok('chat opens in a dialog', await wait(`!!document.querySelector('[data-chat-dialog] [data-chat-view]')`, 8000, 'dialog'))
    ok('chat lands on the turn', await wait(`document.querySelector('[data-chat-dialog] [data-chat-focus]')?.getAttribute('data-id') === 'u:${TURN}'`, 15000, 'focus'))
    const focus = await evaluate(`document.querySelector('[data-chat-dialog] [data-chat-focus]')?.textContent ?? ''`)
    ok('focused turn is the prompt that edited the files', focus.includes('Make login retry 500ms'), focus.slice(0, 80))
    ok('later turn is not loaded yet, view is pinned', await evaluate(`!!document.querySelector('[data-chat-dialog] [data-chat-pinned]') && !document.querySelector('[data-chat-dialog]').textContent.includes('Now write the docs')`))
    const visible = await evaluate(`(() => { const s = document.querySelector('[data-chat-dialog] [data-chat-scroll]').getBoundingClientRect(); const f = document.querySelector('[data-chat-dialog] [data-chat-focus]').getBoundingClientRect(); return f.top >= s.top - 1 && f.top < s.bottom })()`)
    ok('focused turn is scrolled into view', visible)
    await sleep(300)
    if (SHOTS) {
      await shot(`chat-turn-${ui}`)
      copyShot(`chat-turn-${ui}`)
    }
    await evaluate(`document.querySelector('[data-chat-dialog] [data-chat-live]').click(); true`)
    ok('Latest loads the newest turn', await wait(`document.querySelector('[data-chat-dialog]')?.textContent.includes('Now write the docs') && !document.querySelector('[data-chat-dialog] [data-chat-pinned]')`, 10000, 'live'))
    await evaluate(`document.querySelector('[data-chat-dialog-close]').click(); true`)
    ok('dialog closes', await wait(`!document.querySelector('[data-chat-dialog]')`, 4000, 'closed'))
    if (failed) await shot(`commit-chat-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`commit-chat-${ui}-error`).catch(() => undefined)
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
