const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4581'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9581'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v109-secret-guard')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-secret-')))
const DIR = path.join(BASE, 'repo')
const REMOTE = path.join(BASE, 'remote.git')
const TOKEN = 'ghp_' + 'Q7wE2rT9yU4iO1pA6sD3fG8hJ5kL0zX2'
const PEM = '-----BEGIN RSA PRIVATE KEY-----'
const git = (...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', ...args], { cwd: DIR, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
const count = () => Number(git('rev-list', '--count', 'HEAD').trim())
const staged = () => git('diff', '--cached', '--name-only').split('\n').map((s) => s.trim()).filter(Boolean)

function fixture() {
  fs.mkdirSync(DIR, { recursive: true })
  execFileSync('git', ['init', '-q', '--bare', REMOTE], { stdio: 'ignore' })
  git('init', '-q', '-b', 'main')
  git('config', 'core.autocrlf', 'false')
  fs.writeFileSync(path.join(DIR, 'README.md'), '# secret guard\n')
  git('add', '-A')
  git('commit', '-q', '-m', 'init')
  git('remote', 'add', 'origin', REMOTE)
  git('push', '-q', '-u', 'origin', 'main')
  fs.writeFileSync(path.join(DIR, '.env'), `DEBUG=1\nGITHUB_TOKEN=${TOKEN}\n`)
  fs.writeFileSync(path.join(DIR, 'notes.txt'), 'plain notes\n')
  git('add', '.env', 'notes.txt')
}

const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.explorerOpen','0'); localStorage.removeItem('ccm.secretGuard')`
const typeMessage = (text) => `(() => { const ta = document.querySelector('[data-explorer] textarea'); ta.value = ${JSON.stringify(text)}; ta.dispatchEvent(new Event('input', { bubbles: true })); return true })()`

async function run() {
  const app = await launch({ ui: 'classic', env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(s, v, extra) }
  const until = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(800)
    await key('E', 'KeyE', 69, 2 | 8)
    await waitFor(`!!document.querySelector('[data-explorer] [data-view=changes]')`, 10000, 'explorer')
    await evaluate(`document.querySelector('[data-explorer] [data-view=changes]').click(); true`)
    ok('Changes view lists both staged files', await until(`!!document.querySelector('[data-change=".env"]') && !!document.querySelector('[data-change="notes.txt"]')`, 10000, 'changes'))
    ok('setting is on by default', await evaluate(`${S}.get('settings').secretGuard === true`))

    await evaluate(typeMessage('add env'))
    await waitFor(`!document.querySelector('[data-commit-submit]').disabled`, 5000, 'commit enabled')
    await evaluate(`document.querySelector('[data-commit-submit]').click(); true`)
    ok('commit with a token is held by the warning', await until(`!!document.querySelector('[data-secret-guard]')`, 15000, 'guard'))
    ok('nothing was committed', count() === 1, String(count()))
    const where = await evaluate(`[...document.querySelectorAll('[data-secret-where]')].map((n) => n.textContent.trim())`)
    ok('warning shows file and line', JSON.stringify(where) === JSON.stringify(['.env:2']), JSON.stringify(where))
    const masked = await evaluate(`[...document.querySelectorAll('[data-secret-masked]')].map((n) => n.textContent.trim())`)
    ok('value is masked', JSON.stringify(masked) === JSON.stringify(['ghp_••••']), JSON.stringify(masked))
    ok('raw token never reaches the page', !(await evaluate(`document.documentElement.outerHTML.includes(${JSON.stringify(TOKEN.slice(4))})`)))
    ok('kind label is shown', /GitHub token/.test(await evaluate(`document.querySelector('[data-secret-hit]')?.textContent ?? ''`)))

    await evaluate(`document.querySelector('[data-secret-file][data-path=".env"] [data-secret-unstage]').click(); true`)
    ok('Remove from stage closes the warning once nothing is left', await until(`!document.querySelector('[data-secret-guard]')`, 10000, 'guard closed'))
    ok('.env is no longer staged, notes.txt still is', JSON.stringify(staged()) === JSON.stringify(['notes.txt']), JSON.stringify(staged()))
    ok('notice asks to commit again', /Press Commit again/.test(await evaluate(`document.querySelector('[data-commit-notice]')?.textContent ?? ''`)))

    fs.writeFileSync(path.join(DIR, 'id_rsa'), `${PEM}\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----\n`)
    git('add', 'id_rsa')
    await waitFor(`!!document.querySelector('[data-change="id_rsa"]')`, 10000, 'id_rsa listed').catch(() => undefined)
    await evaluate(typeMessage('add key'))
    await waitFor(`!document.querySelector('[data-commit-submit]').disabled`, 5000, 'commit enabled 2')
    await evaluate(`document.querySelector('[data-commit-submit]').click(); true`)
    ok('private key is held too', await until(`!!document.querySelector('[data-secret-guard]') && /Private key/.test(document.querySelector('[data-secret-guard]').textContent)`, 15000, 'guard 2'))
    ok('still nothing committed', count() === 1, String(count()))
    await evaluate(`document.querySelector('[data-secret-proceed]').click(); true`)
    ok('Commit anyway commits after the explicit click', await until(`!document.querySelector('[data-secret-guard]') && /add key/.test(document.querySelector('[data-commit-notice]')?.textContent ?? '')`, 15000, 'committed'))
    ok('git has the new commit', count() === 2, String(count()))

    const blocked = await evaluate(`window.ccmDesktop.gitPush(${JSON.stringify(DIR)}, 'refs/heads/main', 'origin', 'main', false, { scan: true })`)
    ok('push with the key is held by Rust', blocked?.error === 'secrets' && blocked.hits?.some((h) => h.kind === 'private-key' && h.path === 'id_rsa' && h.commit), JSON.stringify(blocked).slice(0, 300))
    ok('push result carries no raw key text', !JSON.stringify(blocked).includes('MIIEowIBAAKCAQEA'))
    const remoteHead = () => execFileSync('git', ['rev-parse', 'main'], { cwd: REMOTE, encoding: 'utf8' }).trim()
    ok('remote was not updated', remoteHead() !== git('rev-parse', 'HEAD').trim())
    const pushed = await evaluate(`window.ccmDesktop.gitPush(${JSON.stringify(DIR)}, 'refs/heads/main', 'origin', 'main', false, { scan: false })`)
    ok('Push anyway goes through', pushed?.ok === true && remoteHead() === git('rev-parse', 'HEAD').trim(), JSON.stringify(pushed))

    fs.writeFileSync(path.join(DIR, '.env.example'), `GITHUB_TOKEN=${TOKEN}\n`)
    fs.writeFileSync(path.join(DIR, 'aws.md'), 'key AKIAIOSFODNN7EXAMPLE\n')
    git('add', '.env.example', 'aws.md')
    const sample = await evaluate(`window.ccmDesktop.gitCommit(${JSON.stringify(DIR)}, 'samples', false, { scan: true })`)
    ok('sample values and .env.example are not flagged', sample?.ok === true, JSON.stringify(sample).slice(0, 300))

    await evaluate(`const u = ${S}.get('ui'); u.settingsPage = 'workspace'; u.settingsOpen = true; true`)
    ok('Settings has the switch', await until(`!!document.querySelector('[data-setting-row="secretGuard"]')`, 10000, 'setting row'))
    await evaluate(`document.querySelector('[data-setting-row="secretGuard"] [role="switch"]').click(); true`)
    await sleep(300)
    ok('switch off stores 0', (await evaluate(`localStorage.getItem('ccm.secretGuard')`)) === '0')
    await evaluate(`${S}.get('ui').settingsOpen = false; true`)
    if (failed) await shot('secret-guard-fail')
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await shot('secret-guard-error').catch(() => undefined)
  } finally {
    app.stop()
  }
}

;(async () => {
  try {
    fixture()
    await run()
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  try { fs.rmSync(BASE, { recursive: true, force: true }) } catch {}
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
