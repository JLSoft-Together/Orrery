const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const http = require('node:http')
const os = require('node:os')
const path = require('node:path')
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4393'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9393'
process.env.CCM_E2E_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v111-pr')
const { launch } = require('./cdp.cjs')
const REPO = path.resolve(__dirname, '../../..')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v111/WP-111f')
const FAKE_PORT = Number(process.env.CCM_E2E_GH_PORT) || 4394
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const T = `${S}.get('terminals')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const REC = path.join(__dirname, 'fixtures', 'stdin-rec.cjs')
const PREP = `localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`
const TOKEN = 'ghp_' + 'e'.repeat(36)
const MERGE_SHA = 'b'.repeat(40)
const LOG = Array.from({ length: 80 }, (_, i) => `line ${i + 1}`).concat(['FAIL src/login.test.ts', 'expected 200, got 500'])

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v111pr-')))
const git = (cwd, ...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', '-c', 'core.autocrlf=false', ...args], { cwd, stdio: 'pipe', encoding: 'utf8' }).trim()

function makeRepo(name, url, branch) {
  const dir = path.join(BASE, name)
  fs.mkdirSync(dir, { recursive: true })
  git(dir, 'init', '-q', '-b', 'main')
  fs.writeFileSync(path.join(dir, 'a.txt'), 'a\n')
  git(dir, 'add', '-A')
  git(dir, 'commit', '-q', '-m', 'init')
  git(dir, 'remote', 'add', 'origin', url)
  git(dir, 'update-ref', 'refs/remotes/origin/main', 'HEAD')
  git(dir, 'checkout', '-q', '-b', branch)
  fs.writeFileSync(path.join(dir, 'a.txt'), 'a\nb\n')
  git(dir, 'add', '-A')
  git(dir, 'commit', '-q', '-m', 'feat: login')
  git(dir, 'update-ref', `refs/remotes/origin/${branch}`, 'HEAD')
  git(dir, 'config', `branch.${branch}.remote`, 'origin')
  git(dir, 'config', `branch.${branch}.merge`, `refs/heads/${branch}`)
  return { dir, sha: git(dir, 'rev-parse', 'HEAD') }
}

const fake = { phase: 'pending', pr: null, merged: false, posts: [], merges: [], deletes: [], logs: 0, headSha: '' }

function prObject() {
  return {
    number: 42,
    title: fake.pr.title,
    html_url: 'https://github.com/o/r/pull/42',
    state: fake.merged ? 'closed' : 'open',
    merged: fake.merged,
    draft: false,
    head: { ref: fake.pr.head, sha: fake.headSha, repo: { full_name: 'o/r' } },
    base: { ref: fake.pr.base, repo: { full_name: 'o/r' } },
    mergeable: true,
    mergeable_state: 'clean',
  }
}

function runs() {
  const iso = (ms) => new Date(Date.now() - ms).toISOString()
  const lint = { id: 2, name: 'lint', status: 'completed', conclusion: 'success', html_url: 'https://github.com/o/r/runs/2', started_at: iso(200000), completed_at: iso(170000), app: { slug: 'github-actions' } }
  const build = { id: 9, name: 'build', html_url: 'https://github.com/o/r/runs/9', started_at: iso(160000), app: { slug: 'github-actions' } }
  if (fake.phase === 'pending') Object.assign(build, { status: 'in_progress', conclusion: null })
  else Object.assign(build, { status: 'completed', conclusion: fake.phase === 'failure' ? 'failure' : 'success', completed_at: iso(10000) })
  return { total_count: 2, check_runs: [build, lint] }
}

function startFake() {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, `http://127.0.0.1:${FAKE_PORT}`)
    let body = ''
    req.on('data', (d) => (body += d))
    req.on('end', () => {
      const json = (status, value, headers = {}) => {
        res.writeHead(status, { 'content-type': 'application/json', ...headers })
        res.end(JSON.stringify(value))
      }
      const p = url.pathname
      if (p === '/user') return json(200, { login: 'e2e-user', name: 'E2E' }, { 'x-oauth-scopes': 'repo' })
      if (p === '/repos/o/r') return json(200, { default_branch: 'main', permissions: { push: true } })
      if (p === '/repos/o/scoped') return json(200, { default_branch: 'main', permissions: { push: true } })
      if (p.startsWith('/repos/o/scoped/')) return json(403, { message: 'Resource not accessible by personal access token' }, { 'x-accepted-github-permissions': 'pull_requests=read' })
      if (p === '/repos/o/r/branches') return json(200, [{ name: 'main' }, { name: 'feat/login' }])
      if (p === '/repos/o/r/pulls' && req.method === 'GET') return json(200, fake.pr && !fake.merged ? [prObject()] : [])
      if (p === '/repos/o/r/pulls' && req.method === 'POST') {
        const b = JSON.parse(body || '{}')
        fake.posts.push(b)
        fake.pr = b
        return json(201, prObject())
      }
      if (p === '/repos/o/r/pulls/42' && req.method === 'GET') return json(200, prObject())
      if (p === '/repos/o/r/pulls/42/merge' && req.method === 'PUT') {
        fake.merges.push(JSON.parse(body || '{}'))
        fake.merged = true
        return json(200, { merged: true, sha: MERGE_SHA, message: 'ok' })
      }
      if (p.startsWith('/repos/o/r/git/refs/heads/') && req.method === 'DELETE') {
        fake.deletes.push(decodeURIComponent(p))
        res.writeHead(204)
        return res.end()
      }
      if (/^\/repos\/o\/r\/commits\/[^/]+\/check-runs$/.test(p)) return json(200, runs())
      if (/^\/repos\/o\/r\/commits\/[^/]+\/status$/.test(p)) return json(200, { state: 'success', statuses: [] })
      if (p === '/repos/o/r/actions/jobs/9/logs') {
        fake.logs += 1
        res.writeHead(200, { 'content-type': 'text/plain' })
        return res.end(LOG.join('\n') + '\n')
      }
      return json(404, { message: 'Not Found' })
    })
  })
  return new Promise((resolve) => server.listen(FAKE_PORT, '127.0.0.1', () => resolve(server)))
}

async function main() {
  fs.mkdirSync(SHOTS, { recursive: true })
  const main = makeRepo('proj', 'https://github.com/o/r.git', 'feat/login')
  fake.headSha = main.sha
  const scoped = makeRepo('scoped', 'https://github.com/o/scoped.git', 'feat/scoped')
  const other = makeRepo('gitlab', 'https://gitlab.com/o/r.git', 'feat/other')
  const server = await startFake()
  const recOut = path.join(main.dir, 'stdin.txt')
  const app = await launch({
    env: { CCM_GH_API: `http://127.0.0.1:${FAKE_PORT}`, CCM_E2E_CONFIRM: 'ok', CCM_VERIFY_PROCESSES: '0', ...(process.env.CCM_WEB_DIST ? { CCM_WEB_DIST: process.env.CCM_WEB_DIST } : {}) },
  })
  const { evaluate, waitFor, sleep, cmd } = app
  let failed = false
  const ok = (s, v, extra) => { if (!v) failed = true; check(s, !!v, extra) }
  const until = (expr, ms, label) => waitFor(expr, ms, label).then(() => true, () => false)
  const snap = async (name) => {
    const r = await cmd('Page.captureScreenshot', { format: 'png' })
    if (r.result?.data) fs.writeFileSync(path.join(SHOTS, `${name}.png`), Buffer.from(r.result.data, 'base64'))
  }
  const click = (sel) => evaluate(`(() => { const n = [...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.offsetParent); n?.click(); return !!n })()`)
  const text = (sel) => evaluate(`[...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.offsetParent)?.textContent.replace(/\\s+/g, ' ').trim() ?? null`)
  const present = (sel) => `!![...document.querySelectorAll(${JSON.stringify(sel)})].find((x) => x.offsetParent)`
  const confirms = () => { try { return fs.readFileSync(path.join(process.env.CCM_E2E_OUT, 'run', 'data', 'e2e-confirms.log'), 'utf8') } catch { return '' } }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(PREP)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)

    const added = await evaluate(`window.ccmDesktop.ghAccountAdd(${JSON.stringify(TOKEN)})`)
    ok('fake GitHub accepts the token as an account', added?.ok === true, JSON.stringify(added))

    const pane = await evaluate(`${W}.open('shell', ${JSON.stringify(main.dir)}, undefined, { tab: true }).key`)
    await waitFor(`${W}.panes.find((p) => p.key === '${pane}')?.state === 'running'`, 25000, 'shell running')
    await sleep(1500)
    await evaluate(`[...document.querySelectorAll('.xterm-helper-textarea')].find((t) => t.offsetParent !== null)?.focus()`)
    await app.type(`node ${REC.replace(/\\/g, '/')} ${recOut.replace(/\\/g, '/')}`)
    await sleep(300)
    await app.key('Enter', 'Enter', 13)
    for (let i = 0; i < 40 && !fs.existsSync(recOut); i++) await sleep(250)
    ok('recorder started in the agent pane', fs.existsSync(recOut))
    await evaluate(`${T}.upsert({ id: 'e2e-pr', title: 'Agent', status: 'idle', cwd: ${JSON.stringify(main.dir)} })`)
    await evaluate(`${W}.update('${pane}', { preset: 'claude', terminalId: 'e2e-pr', focusedAt: 1 })`)
    await sleep(500)

    await evaluate(`${W}.openLog(${JSON.stringify(main.dir)})`)
    ok('no PR yet: panel offers Create PR', await until(present('[data-pr-state="noPr"] [data-pr-create-open]'), 20000, 'create button'))
    await snap('1-none')

    await click('[data-pr-create-open]')
    ok('Create PR opens the dialog', await until(present('[data-pr-create]'), 10000, 'dialog'))
    await until(`!document.querySelector('[data-pr-create] .animate-spin')`, 8000, 'body done')
    const titleValue = await evaluate(`document.querySelector('[data-pr-title]').value`)
    ok('title is the subject of the last commit', titleValue === 'feat: login', titleValue)
    ok('base defaults to the default branch of the repo', /main/.test((await text('[data-pr-base]')) ?? ''), String(await text('[data-pr-base]')))
    await snap('2-create-dialog')
    await click('[data-pr-submit]')
    const posted = await new Promise((resolve) => {
      const t = setInterval(() => fake.posts.length && (clearInterval(t), resolve(true)), 100)
      setTimeout(() => (clearInterval(t), resolve(false)), 15000)
    })
    ok('PR is created on GitHub with base, head, title', posted && fake.posts[0].base === 'main' && fake.posts[0].head === 'feat/login' && fake.posts[0].title === 'feat: login', JSON.stringify(fake.posts[0]))
    ok('panel shows the new PR with CI running (icon plus text)', await until(`document.querySelector('[data-pr-ci-state="pending"]')?.textContent.includes('CI running')`, 15000, 'pending'), String(await text('[data-pr-row]')))
    await evaluate(`document.querySelector('[data-pr-toggle]').click()`)
    await until(`document.querySelectorAll('[data-pr-run]').length === 2`, 5000, 'runs')
    await snap('3-running')

    fake.phase = 'failure'
    await click('[data-pr-refresh]')
    ok('refresh shows CI failed', await until(`document.querySelector('[data-pr-ci-state="failure"]')?.textContent.includes('CI failed')`, 15000, 'failure'))
    await click('[data-pr-log-toggle]')
    ok('failed job opens the last lines of its log', await until(`/expected 200, got 500/.test(document.querySelector('[data-pr-log-text]')?.textContent ?? '')`, 15000, 'log'))
    ok('log is capped to 60 lines', (await evaluate(`document.querySelector('[data-pr-log-text]').textContent.split('\\n').length`)) <= 61)
    ok('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= window.innerWidth`))
    await snap('4-failed-log')
    const before = fs.readFileSync(recOut, 'utf8').length
    await click('[data-pr-send-error]')
    for (let i = 0; i < 25 && !/expected 200/.test(fs.readFileSync(recOut, 'utf8').slice(before)); i++) await sleep(250)
    const sent = fs.readFileSync(recOut, 'utf8').slice(before).replace(/\x1b\[20[01]~/g, '').replace(/\r\n?/g, '\n')
    ok('Send the error puts the log in the agent prompt', /CI failed build \(feat\/login\)\n```\n[\s\S]*expected 200, got 500\n```\n/.test(sent), JSON.stringify(sent.slice(0, 120)))
    ok('prefill only: the block ends at its fence, no extra Enter', /```\n$/.test(sent) && !/\n\n$/.test(sent), JSON.stringify(sent.slice(-30)))
    ok('toast tells to review it and press Enter', await until(`/Enter/.test(document.querySelector('[data-pr-sent]')?.textContent ?? '')`, 4000, 'sent toast'))

    ok('background watch ends with a CI failed toast', await until(`!!document.querySelector('[data-ci-toast="failure"]')`, 70000, 'ci failed toast'))
    await snap('5-ci-toast')
    ok('the project keeps a red CI chip state', (await evaluate(`${S}.get('ci').chip(${JSON.stringify(main.dir)})`)) === 'failure')

    await evaluate(`${W}.activate('${pane}')`)
    await evaluate(`${W}.openLog(${JSON.stringify(main.dir)})`)
    fake.phase = 'success'
    await sleep(500)
    await click('[data-pr-refresh]')
    ok('after a fix CI shows passed', await until(`document.querySelector('[data-pr-ci-state="success"]')?.textContent.includes('CI passed')`, 15000, 'success'))
    ok('Merge is offered', await until(present('[data-pr-merge-open]'), 5000, 'merge btn'))
    await click('[data-pr-merge-open]')
    ok('Merge opens the dialog with the delete-branch box on', (await until(present('[data-pr-merge]'), 8000, 'merge dialog')) && (await evaluate(`document.querySelector('[data-pr-delete-branch]').checked`)) === true)
    await evaluate(`document.querySelector('[data-pr-method="squash"]').click()`)
    await snap('6-merge-dialog')
    await click('[data-pr-merge-submit]')
    ok('merged toast appears', await until(`!!document.querySelector('[data-pr-merged]')`, 20000, 'merged toast'))
    ok('GitHub got a squash merge for the PR head sha', fake.merges.length === 1 && fake.merges[0].merge_method === 'squash' && fake.merges[0].sha === main.sha, JSON.stringify(fake.merges))
    ok('the remote branch was deleted', fake.deletes.includes('/repos/o/r/git/refs/heads/feat/login'), JSON.stringify(fake.deletes))
    ok('native confirm named the PR, head, base and method', /Merge PR #42/.test(confirms()) && /feat\/login -> main \(squash\)/.test(confirms()), confirms().slice(-300))
    ok('panel shows Merged and no Merge button', (await until(`document.querySelector('[data-pr-state-chip]')?.textContent === 'Merged'`, 8000, 'merged chip')) && !(await evaluate(present('[data-pr-merge-open]'))))
    await snap('7-merged')

    await evaluate(`${W}.open('shell', ${JSON.stringify(scoped.dir)}, undefined, { tab: true })`)
    await sleep(1500)
    await evaluate(`${W}.openLog(${JSON.stringify(scoped.dir)})`)
    ok('token without PR permission shows which permission is missing', await until(`/pull_requests \\(read\\)/.test(document.querySelector('[data-pr-scope]')?.textContent ?? '')`, 20000, 'scope'), String(await text('[data-pr-scope]')))
    await snap('8-scope')

    await evaluate(`${W}.open('shell', ${JSON.stringify(other.dir)}, undefined, { tab: true })`)
    await sleep(1500)
    const otherKey = await evaluate(`${W}.openLog(${JSON.stringify(other.dir)}).key`)
    await sleep(4000)
    ok('a non-GitHub repo shows no PR panel in its Log', await evaluate(`(() => { const lp = document.querySelector(':is([data-pane="${otherKey}"], [data-pane-of="${otherKey}"]) [data-log-pane]'); return !!lp && !lp.querySelector('[data-pr-panel]') })()`))
    if (failed) await snap('error')
  } catch (e) {
    ok('script ran', false, String(e && e.message).slice(0, 300))
    await snap('error').catch(() => undefined)
  } finally {
    app.stop()
    server.close()
  }
}

;(async () => {
  try {
    await main()
  } catch (e) {
    check('script ran', false, String(e && e.message).slice(0, 300))
  }
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
