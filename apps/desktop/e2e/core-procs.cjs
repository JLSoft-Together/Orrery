const { spawn } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }
const bin = process.env.CCM_CORE_BIN
if (!bin) throw new Error('CCM_CORE_BIN is required')

async function run(mode) {
  const fake = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-core-procs-e2e-'))
  const config = path.join(fake, 'claude-config')
  const cwd = path.join(fake, 'project')
  const sessionId = require('node:crypto').randomUUID()
  fs.mkdirSync(path.join(config, 'sessions'), { recursive: true })
  fs.mkdirSync(path.join(config, 'projects', cwd.replace(/[^a-zA-Z0-9]/g, '-')), { recursive: true })
  fs.mkdirSync(cwd, { recursive: true })
  const child = spawn(process.execPath, ['-e', 'setInterval(()=>{},1000)'], { stdio: 'ignore' })
  const listener = spawn(process.execPath, ['-e', 'const s=require("node:net").createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);setInterval(()=>{},1000)})'], { stdio: ['ignore', 'pipe', 'ignore'] })
  let output = ''
  listener.stdout.on('data', (chunk) => { output += chunk.toString() })
  const started = Date.now()
  fs.writeFileSync(path.join(config, 'sessions', `${child.pid}.json`), JSON.stringify({ pid: child.pid, sessionId, cwd, startedAt: started, version: '2.1.294', kind: 'interactive', entrypoint: 'cli', name: 'core-procs-e2e', nameSource: 'derived', status: 'idle', updatedAt: started, statusUpdatedAt: started }))
  fs.writeFileSync(path.join(config, 'projects', cwd.replace(/[^a-zA-Z0-9]/g, '-'), `${sessionId}.jsonl`), `${JSON.stringify({ type: 'user', timestamp: new Date(started).toISOString(), cwd, sessionId, message: { role: 'user', content: 'hello' } })}\n`)
  let app
  try {
    app = await launch({ env: { CLAUDE_CONFIG_DIR: config, CCM_CORE_PROCS: mode, CCM_CORE_BIN: bin, CCM_VERIFY_PROCESSES: '1', CCM_CODEX: '0', CCM_DEEPSEEK: '0' } })
    await app.waitFor(`!!window.ccmDesktop && !!document.querySelector('#app')?.__vue_app__ && !!${S}.get('resources')`, 30000, 'app ready')
    const port = await new Promise((resolve, reject) => {
      const end = Date.now() + 10000
      const poll = () => {
        const n = Number(output.trim().split(/\s+/).at(-1))
        if (n) return resolve(n)
        if (Date.now() > end) return reject(new Error('child did not open a port'))
        setTimeout(poll, 100)
      }
      poll()
    })
    await app.evaluate(`${S}.get('resources').show('ports')`)
    await app.waitFor(`document.querySelector('[role="dialog"]')`)
    const report = await app.waitFor(`(() => { const r = ${S}.get('resources').report; return !!r && r.outside?.some(x => x.pid === ${listener.pid} && x.ports.some(p => p.port === ${port})) && r.sessions?.some(s => s.pid === ${child.pid} && s.ramBytes > 0) })()`, 70000, 'resources report has the session and the listener')
    check(`${mode}: report has the session with RAM and the outside listener`, Boolean(report), `session=${child.pid} listener=${listener.pid} port=${port}`)
    const session = await app.waitFor(`Object.values(${S}.get('terminals').byId).find(t => t.processId === ${child.pid} && t.status !== 'ended')`)
    check(`${mode}: real node session remains non-stale`, Boolean(session), `pid=${child.pid}`)
  } finally {
    app?.stop()
    child.kill()
    listener.kill()
    fs.rmSync(fake, { recursive: true, force: true })
  }
}

;(async () => {
  try {
    await run('rust')
    await run('node')
  } catch (err) {
    check('E2E run', false, err.message)
  }
  process.exit(results.some((line) => line.startsWith('FAIL')) ? 1 : 0)
})()
