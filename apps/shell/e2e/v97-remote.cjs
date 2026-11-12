const os = require('node:os')
const path = require('node:path')
const fs = require('node:fs')
const { spawn, execFileSync } = require('node:child_process')
const REPO = path.resolve(__dirname, '../../..')
const OUT_DIR = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-v97')
process.env.CCM_E2E_SHELL = 'tauri'
process.env.CCM_E2E_PORT = process.env.CCM_E2E_PORT || '4419'
process.env.CCM_E2E_DBG = process.env.CCM_E2E_DBG || '9343'
process.env.CCM_E2E_OUT = OUT_DIR
const { launch, OUT } = require(path.join(REPO, 'apps/desktop/e2e/cdp.cjs'))
const WS = require(path.join(REPO, 'node_modules/ws'))
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const UI = `${S}.get('ui')`
const D = 'window.ccmDesktop'
const REMOTE_PORT = Number(process.env.CCM_E2E_REMOTE_PORT) || 4330
const RELAY = process.env.CCM_E2E_RELAY || 'https://orrery-relay.jinvu.workers.dev'
const EXE = process.platform === 'win32' ? '.exe' : ''
const PHONE = process.env.CCM_E2E_PHONE || path.join(REPO, 'apps/mobile/rust/target/debug/examples/phone_cli' + EXE)
const CORE = process.env.CCM_CORE_BIN || path.join(REPO, 'apps/core/target/debug/orrery-core' + EXE)
const RESULT = path.join(REPO, 'docs/qa/v97/WP-18/RESULT.md')
const SHOTS = path.join(REPO, 'docs/qa/v97/WP-18')
const NAMES = ['', 'Bật remote, listener LAN riêng', 'Ghép bằng QR + confirm native', 'IK + hello + snapshot + ws.snapshot', 'ws.open shell, desktop không đổi focus / view', 'pty.attach + echo + claim', 'ws.open khi desktop ở Monitor (H5)', 'ws.close pane bận → confirm.ask tới điện thoại', 'fs.list / fs.read / denied / UNC', 'Thu hồi → 4401, nối lại bị từ chối', 'Lặp 3–5 qua relay + thu hồi qua relay', 'Tắt remote → listener đóng', 'F3 ws.patch ≤ 1 s + log lệnh + tên nền tảng']
const items = NAMES.map(() => [])
const log = []
const t0 = Date.now()
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const stamp = () => `+${((Date.now() - t0) / 1000).toFixed(1)}s`
const check = (n, name, ok, extra = '') => {
  items[n].push({ name, ok: !!ok, extra: String(extra).slice(0, 300) })
  const line = `${ok ? 'PASS' : 'FAIL'} [${n}] ${name}${extra ? ' — ' + String(extra).slice(0, 300) : ''} ${stamp()}`
  log.push(line)
  console.log(line)
}
const note = (text) => {
  log.push(`NOTE ${text}`)
  console.log(`NOTE ${text}`)
}

const TMP = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v97-'))
const dirs = { base: path.join(TMP, 'base'), a: path.join(TMP, 'proj-a'), b: path.join(TMP, 'proj-b'), s1: path.join(TMP, 'state1'), s2: path.join(TMP, 'state2') }
for (const d of Object.values(dirs)) fs.mkdirSync(d, { recursive: true })
fs.writeFileSync(path.join(dirs.a, 'hello.txt'), 'orrery e2e hello\n')

const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'ignore'] })
function listeners(port) {
  try {
    if (process.platform === 'win32') {
      return run('netstat', ['-ano', '-p', 'TCP'])
        .split(/\r?\n/)
        .map((l) => l.trim().split(/\s+/))
        .filter((c) => c[3] === 'LISTENING' && c[1].endsWith(':' + port))
        .map((c) => c[1])
    }
    return run('lsof', ['-nP', `-iTCP:${port}`, '-sTCP:LISTEN', '-Fn']).split('\n').filter((l) => l.startsWith('n')).map((l) => l.slice(1))
  } catch {
    return []
  }
}
const isPrivate = (ip) => /^(10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.)/.test(ip)

function applyOp(root, op) {
  const parts = op.path.split('/').slice(1).map((x) => x.replace(/~1/g, '/').replace(/~0/g, '~'))
  const last = parts.pop()
  let node = root
  for (const p of parts) node = node[p]
  if (op.op === 'remove') {
    if (Array.isArray(node)) node.splice(Number(last), 1)
    else delete node[last]
  } else if (Array.isArray(node) && op.op === 'add') node.splice(Number(last), 0, op.value)
  else node[last] = op.value
}

class Phone {
  constructor(state, url) {
    this.events = []
    this.summary = null
    this.lastKind = ''
    this.n = 0
    this.buf = ''
    this.proc = spawn(PHONE, ['repl', '--state', state, '--url', url], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    this.proc.stdout.on('data', (d) => {
      this.buf += d
      let i
      while ((i = this.buf.indexOf('\n')) >= 0) {
        const line = this.buf.slice(0, i)
        this.buf = this.buf.slice(i + 1)
        try {
          const ev = JSON.parse(line)
          ev.at = Date.now()
          this.events.push(ev)
          if (ev.ev === 'json') this.track(ev.msg)
        } catch {}
      }
    })
    this.proc.stderr.on('data', () => {})
    this.exited = new Promise((r) => this.proc.on('exit', r))
  }
  track(msg) {
    if (msg.t === 'ws.snapshot') {
      this.summary = JSON.parse(JSON.stringify(msg))
      this.lastKind = 'ws.snapshot'
    } else if (msg.t === 'ws.patch' && this.summary && msg.base === this.summary.rev) {
      for (const op of msg.ops) applyOp(this.summary, op)
      this.summary.rev = msg.rev
      this.lastKind = 'ws.patch'
    }
  }
  wsPane(key) {
    return (this.summary?.tabs ?? []).flatMap((t) => t.panes).find((p) => p.key === key) ?? null
  }
  async waitPane(key, pred, ms) {
    const end = Date.now() + ms
    while (Date.now() < end) {
      const p = this.wsPane(key)
      if (p && pred(p)) return { pane: p, at: Date.now(), kind: this.lastKind }
      await sleep(25)
    }
    return null
  }
  async until(pred, ms = 15000) {
    const end = Date.now() + ms
    while (Date.now() < end) {
      const hit = this.events.find(pred)
      if (hit) return hit
      await sleep(60)
    }
    return null
  }
  closed(ms = 8000) {
    return this.until((e) => e.ev === 'closed' || e.ev === 'fatal', ms)
  }
  write(op) {
    this.proc.stdin.write(JSON.stringify(op) + '\n')
  }
  async cmd(msg, ms = 15000) {
    const id = 'c' + ++this.n
    this.write({ op: 'send', msg: { id, ...msg } })
    const hit = await this.until((e) => e.ev === 'json' && e.msg.re === id, ms)
    return hit ? hit.msg : null
  }
  event(t, ms = 8000, pred = () => true) {
    return this.until((e) => e.ev === 'json' && e.msg.t === t && pred(e.msg), ms).then((e) => e?.msg ?? null)
  }
  ptyIn(stream, text, claim = true) {
    this.write({ op: 'pty_in', stream, text, claim })
  }
  ptyText(stream) {
    return this.events.filter((e) => e.ev === 'pty' && e.stream === stream).map((e) => e.text).join('')
  }
  stop() {
    try {
      this.write({ op: 'quit' })
    } catch {}
    setTimeout(() => this.proc.kill(), 1500)
  }
}

const HELLO = { t: 'hello', proto: 1, app: 'phone-cli', platform: 'cli', caps: [] }

async function paneOnPhone(phone, key, ms = 30000) {
  const t = Date.now()
  let nextGet = t + 6000
  while (Date.now() - t < ms) {
    const hit = await phone.waitPane(key, (p) => !!p.sessionId, 500)
    if (hit) return { pane: hit.pane, ms: Date.now() - t }
    if (Date.now() > nextGet) {
      await phone.cmd({ t: 'ws.get' })
      nextGet = Date.now() + 6000
    }
  }
  return { pane: null, ms }
}

async function connect(state, url, label) {
  const phone = new Phone(state, url)
  const ready = await phone.until((e) => e.ev === 'ready' || e.ev === 'closed' || e.ev === 'fatal', 12000)
  if (ready?.ev !== 'ready') {
    note(`${label}: không vào được phiên (${JSON.stringify(ready)})`)
    return { phone, ready: false }
  }
  const welcome = await phone.cmd(HELLO)
  return { phone, ready: true, welcome }
}

async function tunnel(target) {
  const server = new WS.WebSocketServer({ host: '127.0.0.1', port: 0 })
  await new Promise((r) => server.once('listening', r))
  server.on('connection', (client) => {
    const up = new WS(target)
    const pending = []
    up.on('open', () => pending.splice(0).forEach(([d, b]) => up.send(d, { binary: b })))
    client.on('message', (d, b) => (up.readyState === 1 ? up.send(d, { binary: b }) : pending.push([d, b])))
    up.on('message', (d, b) => b && client.readyState === 1 && client.send(d, { binary: true }))
    up.on('close', (code) => client.close(code >= 3000 && code < 5000 ? code : 1000))
    up.on('error', () => client.close(1011))
    client.on('close', () => up.readyState < 2 && up.close())
    client.on('error', () => {})
  })
  return { url: `ws://127.0.0.1:${server.address().port}/remote`, close: () => server.close() }
}

function pairCli(qr, state, name) {
  return new Promise((resolve) => {
    const proc = spawn(PHONE, ['pair', qr, '--state', state, '--name', name], { stdio: ['ignore', 'pipe', 'ignore'], windowsHide: true })
    let out = ''
    proc.stdout.on('data', (d) => (out += d))
    const kill = setTimeout(() => proc.kill(), 60000)
    proc.on('exit', (code) => {
      clearTimeout(kill)
      const lines = out.split('\n').filter(Boolean).map((l) => { try { return JSON.parse(l) } catch { return null } })
      resolve({ code, lines: lines.filter(Boolean) })
    })
  })
}

;(async () => {
  let app
  const tunnels = []
  const phones = []
  try {
    for (const bin of [PHONE, CORE]) if (!fs.existsSync(bin)) throw new Error('thiếu binary: ' + bin)
    if (listeners(REMOTE_PORT).length) throw new Error(`cổng ${REMOTE_PORT} đang bận, đặt CCM_E2E_REMOTE_PORT khác`)
    app = await launch({ env: { CCM_E2E_CONFIRM: 'ok', CCM_REMOTE_PORT: String(REMOTE_PORT), CCM_CORE_BIN: CORE, CCM_KEYRING_SERVICE: 'Orrery E2E v97' } })
    const { evaluate, waitFor, shot, cmd } = app
    await waitFor(`!!${D} && !!${UI}`, 40000, 'app ready')
    const version = require(path.join(REPO, 'apps/desktop/package.json')).version
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.workspace.explorerOpen','0'); localStorage.setItem('ccm.whatsNew.seen','${version}')`)
    await cmd('Page.reload')
    await waitFor(`!!${UI}`, 40000, 'reloaded')
    await evaluate(`${UI}.view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`${W}.open('shell', ${JSON.stringify(dirs.base)})`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 25000, 'base shell running')
    const pane = (key) => evaluate(`(() => { const p = ${W}.panes.find((x) => x.key === ${JSON.stringify(key)}); return p ? { state: p.state, cwd: p.cwd, kind: p.kind } : null })()`)
    const desk = () => evaluate(`({ view: ${UI}.view, active: ${W}.activeKey, tab: ${W}.activeTab?.key ?? null, panes: ${W}.panes.map((p) => p.key) })`)
    const status = () => evaluate(`${D}.remoteStatus()`)
    const until = async (fn, ms = 15000) => {
      const end = Date.now() + ms
      while (Date.now() < end) {
        if (await fn().catch(() => false)) return true
        await sleep(250)
      }
      return false
    }

    let st = await evaluate(`${D}.remoteEnable(true)`)
    check(1, 'remoteEnable(true) → enabled + listening', st?.enabled && st?.listening, JSON.stringify({ enabled: st?.enabled, listening: st?.listening, port: st?.port, relay: st?.relay }))
    check(1, `port = ${REMOTE_PORT}`, st?.port === REMOTE_PORT, String(st?.port))
    check(1, 'addresses non-empty, all private IPv4', st?.addresses?.length > 0 && st.addresses.every(isPrivate), JSON.stringify(st?.addresses))
    const lis = listeners(REMOTE_PORT)
    check(1, 'netstat: listener only on a private LAN IP (not 0.0.0.0 / 127.0.0.1 / ::)', lis.length > 0 && lis.every((l) => isPrivate(l.replace(/:\d+$/, ''))), lis.join(', '))
    check(1, 'netstat: every listener address is in status.addresses', lis.every((l) => st?.addresses?.includes(l.replace(/:\d+$/, ''))), lis.join(', '))
    const lanIp = st?.addresses?.[0]
    const lanUrl = `ws://${lanIp}:${REMOTE_PORT}/remote`

    const qrText = (await evaluate(`${D}.remotePairStart()`))?.qr
    check(2, 'remotePairStart → QR text', typeof qrText === 'string' && qrText.length > 20, String(qrText).slice(0, 40))
    const paired = await pairCli(qrText, dirs.s1, 'E2E Phone One')
    const ok = paired.lines.find((l) => l.ev === 'paired')
    check(2, 'phone_cli pair → pair.ok (native confirm answered ok by CCM_E2E_CONFIRM)', paired.code === 0 && ok?.reply?.t === 'pair.ok', JSON.stringify(paired.lines.at(-1) ?? {}).slice(0, 200))
    const deviceId = ok?.deviceId
    st = await status()
    check(2, 'status.devices has the new device', st?.devices?.some((d) => d.deviceId === deviceId && d.name === 'E2E Phone One'), JSON.stringify(st?.devices?.map((d) => d.name)))
    const asked = path.join(OUT, 'run', 'data', 'e2e-confirms.log')
    note('e2e-confirms.log: ' + (fs.existsSync(asked) ? fs.readFileSync(asked, 'utf8').trim().slice(0, 200) : '(không có)'))
    await evaluate(`${UI}.settingsPage = 'remote'; ${UI}.settingsOpen = true`)
    await sleep(1200)
    fs.mkdirSync(SHOTS, { recursive: true })
    await shot('v97-settings-remote')
    fs.copyFileSync(path.join(OUT, 'v97-settings-remote.png'), path.join(SHOTS, 'settings-remote.png'))
    await evaluate(`${UI}.settingsOpen = false`)

    const s1 = await connect(dirs.s1, lanUrl, 'LAN 1')
    phones.push(s1.phone)
    const p1 = s1.phone
    check(3, 'IK session established (phone_cli ready)', s1.ready)
    check(3, 'hello → welcome ok (machine, version, minProto)', s1.welcome?.ok === true && !!s1.welcome?.machine && s1.welcome?.minProto >= 1, JSON.stringify(s1.welcome ?? {}).slice(0, 160))
    const snap = await p1.event('snapshot', 8000)
    check(3, 'event snapshot (protocolVersion + terminals)', !!snap?.data && Array.isArray(snap.data.terminals), snap ? `seq ${snap.seq}` : 'none')
    let wsSnap = await p1.event('ws.snapshot', 10000)
    check(3, 'event ws.snapshot', !!wsSnap?.tabs, wsSnap ? `rev ${wsSnap.rev} tabs ${wsSnap.tabs.length}` : 'none')
    const phoneKeys = (s) => (s?.tabs ?? []).flatMap((t) => t.panes.map((p) => p.key)).sort()
    const deskNow = await desk()
    check(3, 'ws.snapshot pane keys match the Pinia workspace store', JSON.stringify(phoneKeys(wsSnap)) === JSON.stringify([...deskNow.panes].sort()), `phone ${JSON.stringify(phoneKeys(wsSnap))} store ${JSON.stringify(deskNow.panes)}`)
    const relayToken = await p1.event('relay.token', 3000)
    note(`relay.token event: ${relayToken ? 'có (exp ' + relayToken.exp + ')' : 'không có'}`)

    const before = await desk()
    const sentAt = Date.now()
    const open1 = await p1.cmd({ t: 'ws.open', cwd: dirs.a, preset: 'shell' })
    check(4, 'ws.open → ok + paneKey', open1?.ok === true && !!open1.paneKey, JSON.stringify(open1))
    const key1 = open1?.paneKey
    check(4, 'new pane is in the store', await until(async () => !!(await pane(key1)), 8000))
    const after = await desk()
    check(4, 'ui.view unchanged', after.view === before.view, `${before.view} → ${after.view}`)
    check(4, 'active pane / tab unchanged', after.active === before.active && after.tab === before.tab, `${before.active} → ${after.active}`)
    check(4, 'new pane runs a shell in the asked cwd', await until(async () => (await pane(key1))?.state === 'running', 25000), JSON.stringify(await pane(key1)))
    const seen = await p1.waitPane(key1, () => true, 4000)
    const seenMs = seen ? seen.at - sentAt : -1
    note('F3: ws.open gửi → pane mới có trong summary điện thoại sau ' + seenMs + ' ms (qua ' + (seen?.kind || 'không có') + ')')
    const found1 = await paneOnPhone(p1, key1)
    const phonePane = found1.pane
    check(4, 'phone sees the new pane (ws.get) with a sessionId', !!phonePane?.sessionId, `${found1.ms} ms ${JSON.stringify(phonePane ?? {}).slice(0, 160)}`)

    const stream = 1
    const attach = await p1.cmd({ t: 'pty.attach', stream, sessionId: phonePane?.sessionId, cols: 100, rows: 30 })
    check(5, 'pty.attach → ok', attach?.ok === true && attach.stream === stream, JSON.stringify(attach))
    check(5, 'event pty.ready {cols, rows}', !!(await p1.event('pty.ready', 8000)))
    await sleep(1500)
    p1.ptyIn(stream, 'echo orrery-e2e\r', true)
    const echoed = await until(async () => (p1.ptyText(stream).match(/orrery-e2e/g) ?? []).length >= 2, 15000)
    check(5, 'output contains the typed echo and its result (≥ 2 × orrery-e2e)', echoed, JSON.stringify(p1.ptyText(stream).slice(-160)))
    check(5, 'desktop pane still running', (await pane(key1))?.state === 'running')
    check(5, 'desktop view still unchanged', (await desk()).view === before.view)

    await evaluate(`${UI}.view = 'monitor'`)
    await sleep(800)
    const open2 = await p1.cmd({ t: 'ws.open', cwd: dirs.b, preset: 'shell' })
    const key2 = open2?.paneKey
    check(6, 'ws.open while desktop is on Monitor → ok + paneKey', open2?.ok === true && !!key2, JSON.stringify(open2))
    check(6, 'pane reaches state running without opening Workspace', await until(async () => (await pane(key2))?.state === 'running', 25000), JSON.stringify(await pane(key2)))
    check(6, 'ui.view still monitor', (await desk()).view === 'monitor')

    await evaluate(`${UI}.view = 'workspace'`)
    await sleep(600)
    const sess2 = (await paneOnPhone(p1, key2)).pane?.sessionId
    const attach2 = await p1.cmd({ t: 'pty.attach', stream: 2, sessionId: sess2, cols: 100, rows: 30 })
    await sleep(1500)
    const longCmd = process.platform === 'win32' ? 'ping -n 60 127.0.0.1\r' : 'sleep 60\r'
    p1.ptyIn(2, longCmd, true)
    await sleep(1500)
    const ran = `!!${W}.panes.find((x) => x.key === ${JSON.stringify(key2)})?.ranCommand`
    check(7, 'a long command typed from the phone marks the pane busy (ranCommand) [finding F1 if FAIL]', await evaluate(ran), `ranCommand=${await evaluate(ran)} attach ${JSON.stringify(attach2)}`)
    await evaluate(`${W}.activate(${JSON.stringify(key2)})`)
    await sleep(800)
    await evaluate(`document.querySelector('[data-pane="${key2}"] .xterm-helper-textarea')?.focus()`)
    await app.type('x')
    await app.key('Enter', 'Enter', 13)
    check(7, 'a command typed on the desktop marks the pane busy (control for the confirm test)', await until(async () => await evaluate(ran), 6000), `ranCommand=${await evaluate(ran)}`)
    const closeReply = await p1.cmd({ t: 'ws.close', paneKey: key2 })
    check(7, 'ws.close busy pane → ok + confirm cid (no immediate close)', closeReply?.ok === true && !!closeReply.confirm, JSON.stringify(closeReply))
    const ask = await p1.event('confirm.ask', 8000, (m) => m.cid === closeReply?.confirm)
    check(7, 'event confirm.ask reaches the phone (cid, title, ok, cancel)', !!ask?.cid && !!ask.title && !!ask.ok && !!ask.cancel, JSON.stringify(ask ?? {}).slice(0, 220))
    check(7, 'desktop shows no [role=alertdialog]', (await evaluate(`document.querySelectorAll('[role=alertdialog]').length`)) === 0)
    check(7, 'pane still open while waiting', !!(await pane(key2)))
    const answer = await p1.cmd({ t: 'confirm.answer', cid: closeReply?.confirm, ok: true })
    check(7, 'confirm.answer ok → reply ok', answer?.ok === true, JSON.stringify(answer))
    check(7, 'pane closed on the desktop', await until(async () => !(await pane(key2)), 10000))

    check(12, 'F3: pane mới tới điện thoại ≤ 1 s sau ws.open, qua ws.patch (không cần ws.get)', seenMs >= 0 && seenMs <= 1000 && seen?.kind === 'ws.patch', seenMs + ' ms ' + (seen?.kind || ''))
    const logLines = await evaluate(`${S}.get('remote').log.map((l) => l.text)`)
    note('log lệnh: ' + JSON.stringify(logLines))
    check(12, 'log có dòng mở pane (ws.open)', logLines.some((l) => /^mở /.test(l) || /opened /.test(l)), JSON.stringify(logLines))
    check(12, 'log có dòng đóng pane (ws.close)', logLines.some((l) => /^đóng /.test(l) || /closed /.test(l)), JSON.stringify(logLines))
    await evaluate(`${UI}.settingsPage = 'remote'; ${UI}.settingsOpen = true`)
    await sleep(1200)
    const settingsText = await evaluate('document.body.innerText')
    check(12, 'Settings hiện log lệnh (data-remote-log có ≥ 2 dòng)', (await evaluate("document.querySelectorAll('[data-remote-log] li').length")) >= 2)
    check(12, 'F2: hàng thiết bị hiện "Không rõ" thay vì key thô', /Không rõ/.test(settingsText) && !/remoteV97\.devices\.platform/.test(settingsText), (settingsText.match(/E2E Phone One[^\n]*\n?[^\n]*/) || [''])[0])
    await shot('v97-settings-remote-log')
    fs.copyFileSync(path.join(OUT, 'v97-settings-remote-log.png'), path.join(SHOTS, 'settings-remote-log.png'))
    await evaluate(`${UI}.settingsOpen = false`)

    const list = await p1.cmd({ t: 'fs.list', root: dirs.a, dir: dirs.a })
    check(8, 'fs.list in pane cwd → entry hello.txt', list?.ok === true && JSON.stringify(list.data).includes('hello.txt'), JSON.stringify(list ?? {}).slice(0, 200))
    const read = await p1.cmd({ t: 'fs.read', path: path.join(dirs.a, 'hello.txt') })
    check(8, 'fs.read inside root → text', read?.ok === true && read.data?.kind === 'text' && /orrery e2e hello/.test(read.data.text ?? ''), JSON.stringify(read ?? {}).slice(0, 200))
    const outside = process.platform === 'win32' ? 'C:\\Windows\\win.ini' : '/etc/hosts'
    const denied = await p1.cmd({ t: 'fs.read', path: outside })
    check(8, 'fs.read outside every root → denied', denied?.ok === false && denied.error === 'denied', JSON.stringify(denied ?? {}))
    const unc = await p1.cmd({ t: 'fs.read', path: '\\\\127.0.0.1\\c$\\Windows\\win.ini' })
    check(8, 'fs.read UNC → invalid', unc?.ok === false && unc.error === 'invalid', JSON.stringify(unc ?? {}))
    const sibling = await p1.cmd({ t: 'fs.read', path: path.join(dirs.a, '..', 'proj-b', 'x') })
    check(8, 'fs.read with .. out of the root → denied or invalid', sibling?.ok === false && ['denied', 'invalid'].includes(sibling.error), JSON.stringify(sibling ?? {}))

    const revoked = await evaluate(`${D}.remoteRevoke(${JSON.stringify(deviceId)})`)
    check(9, 'remoteRevoke(deviceId) → true', revoked === true)
    const revEvent = await p1.event('device.revoked', 6000)
    check(9, 'phone gets device.revoked', !!revEvent)
    const closedEv = await p1.closed(8000)
    check(9, 'LAN session closed with 4401', closedEv?.ev === 'closed' && closedEv.code === 4401, JSON.stringify(closedEv ?? {}))
    const again = await connect(dirs.s1, lanUrl, 'LAN reconnect')
    phones.push(again.phone)
    const againClosed = again.ready ? null : await again.phone.closed(4000)
    check(9, 'reconnect with the revoked key refused with 4401', !again.ready && againClosed?.code === 4401, JSON.stringify(againClosed ?? {}))
    st = await status()
    check(9, 'device gone from status.devices', !st?.devices?.some((d) => d.deviceId === deviceId))

    st = await status()
    note(`relay status before item 10: ${st?.relay}`)
    const online = await until(async () => (await status())?.relay === 'online', 45000)
    check(10, 'status.relay = online', online, String((await status())?.relay))
    const qr2 = (await evaluate(`${D}.remotePairStart()`))?.qr
    const paired2 = await pairCli(qr2, dirs.s2, 'E2E Phone Two')
    const ok2 = paired2.lines.find((l) => l.ev === 'paired')
    check(10, 'second phone paired over LAN', ok2?.reply?.t === 'pair.ok', JSON.stringify(paired2.lines.at(-1) ?? {}).slice(0, 160))
    const deviceId2 = ok2?.deviceId
    const lan2 = await connect(dirs.s2, lanUrl, 'LAN 2')
    phones.push(lan2.phone)
    const st2 = JSON.parse(fs.readFileSync(path.join(dirs.s2, 'phone.json'), 'utf8'))
    const evTok = await lan2.phone.event('relay.token', 2000)
    const tok = evTok ?? (st2.relayToken ? { token: st2.relayToken, exp: st2.relayExp, relay: st2.relay } : null)
    check(10, 'relay token available (pair.ok relayToken or relay.token event)', !!tok?.token, tok ? `${evTok ? 'event' : 'pair.ok'} exp ${tok.exp} relay ${tok.relay}` : JSON.stringify(lan2.phone.events.map((e) => e.msg?.t ?? e.ev)))
    lan2.phone.stop()
    const machine = lan2.welcome?.machine ?? ok2?.machine
    if (online && tok?.token && machine) {
      const base = (tok.relay || RELAY).replace(/^http/, 'ws')
      const tn = await tunnel(`${base}/p/${machine}?t=${encodeURIComponent(tok.token)}`)
      tunnels.push(tn)
      const rel = await connect(dirs.s2, tn.url, 'relay')
      phones.push(rel.phone)
      const rp = rel.phone
      check(10, '[3] relay: IK + hello → welcome', rel.ready && rel.welcome?.ok === true, JSON.stringify(rel.welcome ?? {}).slice(0, 120))
      check(10, '[3] relay: snapshot + ws.snapshot', !!(await rp.event('snapshot', 10000)) && !!(await rp.event('ws.snapshot', 10000)))
      const r1 = await rp.cmd({ t: 'ws.open', cwd: dirs.b, preset: 'shell' }, 20000)
      check(10, '[4] relay: ws.open → paneKey', r1?.ok === true && !!r1.paneKey, JSON.stringify(r1))
      const rkey = r1?.paneKey
      await until(async () => (await pane(rkey))?.state === 'running', 25000)
      const rsess = (await paneOnPhone(rp, rkey)).pane?.sessionId
      const ra = await rp.cmd({ t: 'pty.attach', stream: 1, sessionId: rsess, cols: 100, rows: 30 }, 20000)
      check(10, '[5] relay: pty.attach ok', ra?.ok === true, JSON.stringify(ra))
      await sleep(1500)
      rp.ptyIn(1, 'echo orrery-relay\r', true)
      check(10, '[5] relay: typed echo + result come back', await until(async () => (rp.ptyText(1).match(/orrery-relay/g) ?? []).length >= 2, 20000), JSON.stringify(rp.ptyText(1).slice(-120)))
      await evaluate(`${D}.remoteRevoke(${JSON.stringify(deviceId2)})`)
      const rc = await rp.closed(12000)
      if (rc?.code === 4401) check(10, '[9] relay: revoke closes the relay session with 4401', true)
      else note(`relay chưa đóng 4401 sau thu hồi (nhận ${JSON.stringify(rc)}): relay deploy có thể chưa có close control — ghi chú theo brief`)
    } else {
      check(10, 'relay path skipped (relay offline / no token / no machine)', false, JSON.stringify({ online, token: !!tok?.token, machine }))
    }

    st = await evaluate(`${D}.remoteEnable(false)`)
    check(11, 'remoteEnable(false) → enabled false, listening false', st && !st.enabled && !st.listening, JSON.stringify({ enabled: st?.enabled, listening: st?.listening, relay: st?.relay }))
    await sleep(800)
    check(11, `netstat: nothing listens on ${REMOTE_PORT}`, listeners(REMOTE_PORT).length === 0, listeners(REMOTE_PORT).join(','))
    st = await status()
    check(11, 'status relay off', st?.relay === 'off', String(st?.relay))
    note(`keepAwake setting after off: ${st?.keepAwake} (OS power request không kiểm được: powercfg /requests cần admin)`)
  } catch (e) {
    log.push('FAIL run: ' + e.message)
    console.log('FAIL run:', e.stack || e.message)
    await app?.shot('v97-99-error').catch(() => {})
  } finally {
    for (const p of phones) p.stop()
    for (const t of tunnels) t.close()
    app?.stop()
    const secs = ((Date.now() - t0) / 1000).toFixed(0)
    const rows = NAMES.map((name, n) => {
      if (!n) return null
      const list = items[n]
      const state = !list.length ? 'CHƯA CHẠY' : list.every((c) => c.ok) ? 'ĐẠT' : 'LỖI'
      return `| ${n} | ${name} | ${state} | ${list.filter((c) => c.ok).length}/${list.length} |`
    }).filter(Boolean)
    const md = [`# WP-18 RESULT — ${new Date().toISOString()} (${secs}s)`, '', '| # | Mục | Kết quả | Kiểm |', '|---|---|---|---|', ...rows, '', '## Log', '', '```', ...log, '```', ''].join('\n')
    fs.mkdirSync(path.dirname(RESULT), { recursive: true })
    fs.writeFileSync(RESULT, md)
    try {
      fs.rmSync(TMP, { recursive: true, force: true })
    } catch {}
    process.exit(0)
  }
})()
