const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4541'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9541'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-diff-comments')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const REC = path.join(__dirname, 'fixtures', 'stdin-rec.cjs')
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); Object.keys(localStorage).filter((k) => k.startsWith('ccm.diffComments.')).forEach((k) => localStorage.removeItem(k))`
const V2 = 'export const a = 2\nexport const t = 500\nretry(t)\n'

function makeRepo() {
  const repo = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-cm-')))
  fs.mkdirSync(path.join(repo, 'src'))
  const file = path.join(repo, 'src', 'alpha.ts')
  fs.writeFileSync(file, 'export const a = 1\n')
  const git = (...args) => execFileSync('git', args, { cwd: repo, stdio: 'pipe' })
  git('init', '-q')
  git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'add', '.')
  git('-c', 'user.email=e2e@x', '-c', 'user.name=e2e', 'commit', '-qm', 'init')
  fs.writeFileSync(file, V2)
  return { repo, file }
}

async function run(ui) {
  const { repo, file } = makeRepo()
  const recOut = path.join(repo, 'stdin.txt')
  const app = await launch({ ui, env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(`${ui}: ${s}`, !!v, extra) }
  const until = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
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
    ok('recorder started in the pane', fs.existsSync(recOut))
    await evaluate(`${T}.upsert({ id: 'e2e-cm', title: 'Reviewer', status: 'idle', cwd: ${JSON.stringify(repo)} })`)
    await evaluate(`${W}.update('${pane}', { preset: 'claude', terminalId: 'e2e-cm' })`)
    await sleep(500)

    await evaluate(`${W}.openChanges(${JSON.stringify(repo)})`)
    const row = (n) => `document.querySelector('.diff .r[data-new="${n}"]')`
    ok('diff shows the changed file', await until(`!!${row(3)}`, 15000, 'diff rows'))
    ok('each row has a hidden + button', await evaluate(`!!${row(1)}?.querySelector('[data-cm-add="new:1"]') && getComputedStyle(${row(1)}.querySelector('[data-cm-add]')).opacity === '0'`))

    const write = async (text) => {
      await waitFor(`!!document.querySelector('.diff [data-cm-input]')`, 5000, 'composer')
      await evaluate(`(() => { const t = document.querySelector('.diff [data-cm-input]'); t.value = ${JSON.stringify(text)}; t.dispatchEvent(new Event('input', { bubbles: true })); })()`)
      await evaluate(`document.querySelector('.diff [data-cm-save]').click()`)
      await sleep(300)
    }

    await evaluate(`${row(1)}.querySelector('[data-cm-add]').click()`)
    ok('+ opens a composer under the line', await until(`!!${row(1)}?.nextElementSibling?.matches('[data-cm-draft]')`, 5000, 'composer'))
    ok('composer textarea has focus', await evaluate(`document.activeElement?.matches('[data-cm-input]') === true`))
    await write('Read a from config')
    ok('comment shows right under its line', await evaluate(`${row(1)}?.nextElementSibling?.matches('.cmt[data-cm]') && /Read a from config/.test(${row(1)}.nextElementSibling.textContent)`))

    await evaluate(`${row(2)}.querySelector('[data-cm-add]').click()`)
    await sleep(200)
    await evaluate(`${row(3)}.querySelector('[data-cm-add]').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, shiftKey: true }))`)
    ok('Shift+click extends the composer to a line range', await until(`/alpha\\.ts:2-3/.test(document.querySelector('.diff [data-cm-draft]')?.textContent ?? '')`, 5000, 'range'))
    await write('Wrap retry in try / catch')
    ok('range comment sits under its last line', await evaluate(`/Wrap retry/.test(${row(3)}?.nextElementSibling?.textContent ?? '')`))
    ok('both lines of the range are marked', await evaluate(`${row(2)}.classList.contains('cm-in') && ${row(3)}.classList.contains('cm-in')`))
    ok('bar counts 2 comments', /2 comments/.test(await evaluate(`document.querySelector('[data-diff-comments-count]')?.textContent ?? ''`)))

    const firstId = await evaluate(`${row(1)}.nextElementSibling.dataset.cm`)
    await evaluate(`document.querySelector('[data-cm-edit="${firstId}"]').click()`)
    await write('Read a from the config file')
    ok('edit replaces the text', /Read a from the config file/.test(await evaluate(`${row(1)}?.nextElementSibling?.textContent ?? ''`)))
    await evaluate(`${row(1)}.querySelector('[data-cm-add]').click()`)
    await write('temporary')
    const tempId = await evaluate(`[...document.querySelectorAll('.diff .cmt[data-cm]')].find((n) => /temporary/.test(n.textContent))?.dataset.cm`)
    await evaluate(`document.querySelector('[data-cm-del="${tempId}"]').click()`)
    await sleep(300)
    ok('delete removes a comment', (await evaluate(`document.querySelectorAll('.diff .cmt[data-cm]').length`)) === 2)
    ok('comments kept in localStorage per repo', /Wrap retry/.test(await evaluate(`Object.keys(localStorage).filter((k) => k.startsWith('ccm.diffComments.')).map((k) => localStorage.getItem(k)).join('')`)))

    fs.writeFileSync(file, V2.replace('a = 2', 'a = 3'))
    ok('changed line marks its comment as outdated', await until(`!!document.querySelector('.diff .cm-stale')`, 12000, 'stale'))
    fs.writeFileSync(file, V2)
    ok('line back to what it was clears outdated', await until(`!document.querySelector('.diff .cm-stale') && !!${row(1)}`, 12000, 'fresh'))

    await evaluate(`${W}.update('${pane}', { focusedAt: 1 })`)
    await evaluate(`document.querySelector('[data-diff-comments-pick]').click()`)
    ok('agent menu lists the Claude pane of this project', await until(`!!document.querySelector('[data-diff-comments-target="${pane}"]')`, 3000, 'menu'))
    const before = fs.readFileSync(recOut, 'utf8').length
    await evaluate(`document.querySelector('[data-diff-comments-target="${pane}"]').click()`)
    let sent = ''
    for (let i = 0; i < 20 && !/Wrap retry/.test(sent); i++) {
      await sleep(250)
      sent = fs.readFileSync(recOut, 'utf8').slice(before)
    }
    await sleep(1200)
    sent = fs.readFileSync(recOut, 'utf8').slice(before).replace(/\x1b\[20[01]~/g, '')
    const flat = sent.replace(/\r\n?/g, '\n')
    ok('prompt has a header', /Review comments on my diff/.test(flat), JSON.stringify(flat.slice(0, 80)))
    ok('prompt lists path:line with code and comment', /1\. src\/alpha\.ts:1\n\s+```\n\s+export const a = 2\n\s+```\n\s+Read a from the config file/.test(flat), JSON.stringify(flat.slice(0, 300)))
    ok('prompt lists the range item', /2\. src\/alpha\.ts:2-3\n[\s\S]*export const t = 500\n\s+retry\(t\)[\s\S]*Wrap retry in try \/ catch/.test(flat))
    ok('prefill only: nothing typed after the last comment (no Enter)', /Wrap retry in try \/ catch$/.test(sent), JSON.stringify(sent.slice(-40)))
    ok('sent comments are cleared', await until(`!document.querySelector('[data-diff-comments]') && !document.querySelector('.diff .cmt')`, 3000, 'cleared'))
    ok('toast tells to press Enter', await until(`/Press Enter/.test(document.querySelector('[data-diff-comments-sent]')?.textContent ?? '')`, 3000, 'toast'))
    ok('agent pane becomes active after fill', (await evaluate(`${W}.activeKey`)) === pane)

    await evaluate(`${W}.panes.find((p) => p.kind === 'changes') && ${W}.activate(${W}.panes.find((p) => p.kind === 'changes').key)`)
    await evaluate(`${W}.update('${pane}', { preset: 'shell', agent: undefined, terminalId: undefined })`)
    await sleep(300)
    await evaluate(`${row(1)}.querySelector('[data-cm-add]').click()`)
    await write('no agent here')
    ok('send is disabled without a Claude / Codex pane', await evaluate(`document.querySelector('[data-diff-comments-send]')?.disabled === true`), await evaluate(`JSON.stringify({ btn: !!document.querySelector('[data-diff-comments-send]'), p: ${W}.panes.filter((p) => !p.kind || p.kind === 'term').map((p) => [p.preset, p.agent, p.state]) })`))
    await evaluate(`document.querySelector('[data-diff-comments-clear]').click()`)
    await sleep(200)
    ok('Clear all empties the bar', await evaluate(`!document.querySelector('[data-diff-comments]')`))
    if (failed) await shot(`v109-diff-comments-${ui}`)
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot(`v109-diff-comments-${ui}-error`).catch(() => undefined)
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
