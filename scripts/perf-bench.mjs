import { spawn, spawnSync, execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync, createWriteStream } from 'node:fs'
import http from 'node:http'
import net from 'node:net'
import os from 'node:os'
import path from 'node:path'
import readline from 'node:readline'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const WS = createRequire(path.join(ROOT, 'package.json'))('ws')
const WIN = process.platform === 'win32'
const MAC = process.platform === 'darwin'

const argv = process.argv.slice(2)
const opt = (name, def) => {
  const i = argv.findIndex((a) => a === `--${name}` || a.startsWith(`--${name}=`))
  if (i < 0) return def
  if (argv[i].includes('=')) return argv[i].split('=').slice(1).join('=')
  const next = argv[i + 1]
  return next && !next.startsWith('--') ? next : 'true'
}

const SCENARIOS = {
  'idle-monitor': { view: 'monitor' },
  'idle-workspace': { view: 'workspace', panes: 4 },
  'demo-6x3': { view: 'monitor', demo: [6, 3] },
  'demo-30x6': { view: 'monitor', demo: [30, 6] },
}
const wanted = opt('scenario', 'all') === 'all' ? Object.keys(SCENARIOS) : opt('scenario').split(',')
for (const s of wanted) if (!SCENARIOS[s]) throw new Error(`unknown scenario ${s}`)
const SECONDS = Number(opt('seconds', 120))
const SETTLE = Number(opt('settle', 20))
const TRACE = opt('trace', 'false') !== 'false'
const TRACE_SECONDS = Number(opt('trace-seconds', 25))
const PROFILE = opt('profile', 'false') !== 'false'
const PROFILE_SECONDS = Number(opt('profile-seconds', 25))
const PATCH_SECONDS = Number(opt('patch-seconds', 10))
const PROFILE_DIR = opt('profile-dir', '')
const COLLECTOR_PROF = opt('collector-prof', 'false') !== 'false'
const COLLECTOR_PROF_SECONDS = Number(opt('collector-prof-seconds', 30))
const FIXTURE_SEED = opt('seed', null)
const TICK_MS = 2000

const defaultExe = WIN ? 'C:\\Program Files\\Orrery\\Orrery.exe' : MAC ? '/Applications/Orrery.app/Contents/MacOS/Orrery' : ''
const STAGE = opt('stage', 'false') !== 'false'
const exeArg = STAGE ? '' : opt('exe', existsSync(defaultExe) ? defaultExe : '')
const exe = exeArg || path.join(ROOT, 'node_modules', 'electron', 'dist', WIN ? 'electron.exe' : MAC ? 'Electron.app/Contents/MacOS/Electron' : 'electron')
const exeArgs = exeArg ? [] : [path.join(ROOT, 'apps', 'desktop', 'stage')]

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const mb = (n) => Math.round((n / 1048576) * 10) / 10
const round = (n, d = 1) => Math.round(n * 10 ** d) / 10 ** d
const avg = (xs) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0)
const p95 = (xs) => {
  if (!xs.length) return 0
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.ceil(s.length * 0.95) - 1)]
}

const getJson = (url) =>
  new Promise((res, rej) =>
    http
      .get(url, (r) => {
        let b = ''
        r.on('data', (d) => (b += d))
        r.on('end', () => {
          try {
            res(JSON.parse(b))
          } catch (e) {
            rej(e)
          }
        })
      })
      .on('error', rej),
  )

const freePort = () =>
  new Promise((res, rej) => {
    const srv = net.createServer()
    srv.once('error', rej)
    srv.listen(0, '127.0.0.1', () => {
      const { port } = srv.address()
      srv.close(() => res(port))
    })
  })

const lowPriority = (pid) => {
  try {
    os.setPriority(pid, os.constants.priority.PRIORITY_BELOW_NORMAL)
  } catch {}
}

const killTree = (pid) => {
  if (!pid) return
  try {
    if (WIN) execFileSync('taskkill', ['/T', '/F', '/PID', String(pid)], { stdio: 'ignore' })
    else process.kill(-pid, 'SIGKILL')
  } catch {}
}

async function profileCollector(claudeRoot, tmp) {
  const inspectorPort = await freePort()
  const port = await freePort()
  const data = path.join(tmp, 'collector-prof-data')
  mkdirSync(data, { recursive: true })
  const child = spawn(process.execPath, [`--inspect=127.0.0.1:${inspectorPort}`, '--import', 'tsx', path.join(ROOT, 'apps', 'collector', 'src', 'index.ts'), '--dev'], {
    env: { ...process.env, CCM_PORT: String(port), CCM_DATA_DIR: data, CCM_TOAST: '0', CCM_VERIFY_PROCESSES: '0', CLAUDE_CONFIG_DIR: claudeRoot, CODEX_HOME: path.join(tmp, 'codex'), CODEWHALE_HOME: path.join(tmp, 'codewhale') },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let log = ''
  let wsUrl = null
  const appendLog = (b) => {
    log = (log + String(b)).slice(-4000)
    wsUrl ??= log.match(/ws:\/\/127\.0\.0\.1:\d+\/[^\s]+/)?.[0] ?? null
  }
  child.stdout.on('data', appendLog)
  child.stderr.on('data', appendLog)
  let cdp = null
  try {
    for (let i = 0; i < 120; i++) {
      if (wsUrl) break
      if (child.exitCode !== null) break
      await sleep(250)
    }
    if (!wsUrl) throw new Error(`profile collector failed to start (exit ${child.exitCode}): ${log.trim()}`)
    cdp = await connect(wsUrl)
    await sleep(1000)
    await cdp.cmd('Profiler.enable')
    await cdp.cmd('Profiler.setSamplingInterval', { interval: 500 })
    await cdp.cmd('Profiler.start')
    await sleep(COLLECTOR_PROF_SECONDS * 1000)
    const profile = await cdp.cmd('Profiler.stop')
    await cdp.cmd('Profiler.disable')
    const prof = profile.result?.profile
    if (!prof) throw new Error('profile collector returned no CPU profile')
    const analyzed = analyzeProfile(prof, COLLECTOR_PROF_SECONDS)
    return { mode: 'standalone Node collector, source via tsx', seconds: COLLECTOR_PROF_SECONDS, sampleCount: analyzed.sampleCount, sampledMs: analyzed.sampledMs, topSelf: analyzed.topSelf.slice(0, 20) }
  } finally {
    cdp?.close()
    if (child.exitCode === null) {
      child.kill()
      await new Promise((resolve) => child.once('exit', resolve))
    }
  }
}

async function connect(wsUrl) {
  const ws = new WS(wsUrl, { perMessageDeflate: false, maxPayload: 1 << 30 })
  await new Promise((r, j) => {
    ws.once('open', r)
    ws.once('error', j)
  })
  let id = 0
  const pending = new Map()
  const listeners = new Map()
  ws.on('message', (raw) => {
    const m = JSON.parse(raw)
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m)
      pending.delete(m.id)
    } else if (m.method) for (const f of listeners.get(m.method) ?? []) f(m.params)
  })
  const cmd = (method, params = {}) =>
    new Promise((r) => {
      const i = ++id
      pending.set(i, r)
      ws.send(JSON.stringify({ id: i, method, params }))
    })
  const on = (method, f) => listeners.set(method, [...(listeners.get(method) ?? []), f])
  const evaluate = async (expr) => {
    const r = await cmd('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true })
    if (r.result?.exceptionDetails) throw new Error('eval: ' + JSON.stringify(r.result.exceptionDetails).slice(0, 300))
    return r.result?.result?.value
  }
  return { cmd, on, evaluate, close: () => ws.close() }
}

const PS_SAMPLER = (root) => `
$ErrorActionPreference = 'SilentlyContinue'
$root = ${root}
$n = 0
$set = $null
while ($true) {
  if ($n % 5 -eq 0) {
    $all = @(Get-CimInstance Win32_Process | ForEach-Object { [pscustomobject]@{ pid = [int]$_.ProcessId; ppid = [int]$_.ParentProcessId; cmd = $_.CommandLine } })
    $set = New-Object 'System.Collections.Generic.HashSet[int]'
    [void]$set.Add($root)
    do { $grew = $false; foreach ($p in $all) { if ($set.Contains($p.ppid) -and $set.Add($p.pid)) { $grew = $true } } } while ($grew)
    $meta = [ordered]@{ k = 'meta'; procs = @($all | Where-Object { $set.Contains($_.pid) }) }
    [Console]::Out.WriteLine(($meta | ConvertTo-Json -Compress -Depth 4))
  }
  $ids = [int[]]@($set)
  $procs = @(Get-Process -Id $ids)
  $rows = @($procs | ForEach-Object { ,@($_.Id, [math]::Round($_.TotalProcessorTime.TotalSeconds, 3), $_.WorkingSet64, $_.PrivateMemorySize64) })
  $o = [ordered]@{ k = 's'; t = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); p = $rows }
  [Console]::Out.WriteLine(($o | ConvertTo-Json -Compress -Depth 4))
  $n++
  Start-Sleep -Milliseconds ${TICK_MS}
}
`

const parseCpuTime = (s) => s.split(':').reduce((a, v) => a * 60 + Number(v), 0)

function startSampler(rootPid, onMeta, onTick) {
  if (WIN) {
    const child = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', PS_SAMPLER(rootPid)], { stdio: ['ignore', 'pipe', 'ignore'] })
    lowPriority(child.pid)
    readline.createInterface({ input: child.stdout }).on('line', (line) => {
      let m
      try {
        m = JSON.parse(line)
      } catch {
        return
      }
      if (m.k === 'meta') onMeta(Array.isArray(m.procs) ? m.procs : [m.procs])
      else if (m.k === 's') onTick(m.t, (m.p ?? []).filter((r) => Array.isArray(r)))
    })
    return () => killTree(child.pid)
  }
  let stopped = false
  let n = 0
  ;(async () => {
    while (!stopped) {
      const out = spawnSync('ps', ['-axo', 'pid=,ppid=,rss=,cputime=,command='], { encoding: 'utf8', maxBuffer: 1 << 26 }).stdout ?? ''
      const rows = out
        .split('\n')
        .map((l) => /^\s*(\d+)\s+(\d+)\s+(\d+)\s+(\S+)\s+(.*)$/.exec(l))
        .filter(Boolean)
        .map((m) => ({ pid: Number(m[1]), ppid: Number(m[2]), rss: Number(m[3]) * 1024, cpu: parseCpuTime(m[4]), cmd: m[5] }))
      const set = new Set([rootPid])
      let grew = true
      while (grew) {
        grew = false
        for (const r of rows) if (set.has(r.ppid) && !set.has(r.pid)) (set.add(r.pid), (grew = true))
      }
      const mine = rows.filter((r) => set.has(r.pid))
      if (n++ % 5 === 0) onMeta(mine.map((r) => ({ pid: r.pid, ppid: r.ppid, cmd: r.cmd })))
      onTick(Date.now(), mine.map((r) => [r.pid, r.cpu, r.rss, r.rss]))
      await sleep(TICK_MS)
    }
  })()
  return () => {
    stopped = true
  }
}

function collectorPid(port) {
  try {
    if (WIN) {
      const out = execFileSync('netstat', ['-ano', '-p', 'tcp'], { encoding: 'utf8' })
      const m = new RegExp(`127\\.0\\.0\\.1:${port}\\s+\\S+\\s+LISTENING\\s+(\\d+)`).exec(out)
      return m ? Number(m[1]) : null
    }
    const out = execFileSync('lsof', ['-nP', `-iTCP:${port}`, '-sTCP:LISTEN', '-t'], { encoding: 'utf8' })
    return Number(out.split('\n')[0]) || null
  } catch {
    return null
  }
}

function roleOf(cmd, pid, collector) {
  if (!cmd) return 'other'
  if (/--type=gpu-process/.test(cmd)) return 'gpu'
  if (/--type=renderer/.test(cmd)) return 'renderer'
  if (/--type=crashpad/.test(cmd)) return 'crashpad'
  if (/--type=utility/.test(cmd)) {
    const sub = /--utility-sub-type=(\S+)/.exec(cmd)?.[1] ?? ''
    if (/NetworkService/.test(sub)) return 'network'
    if (/AudioService/.test(sub)) return 'audio'
    if (/NodeService/.test(sub)) return pid === collector ? 'collector' : 'pty-host'
    return 'utility-other'
  }
  if (/--type=/.test(cmd)) return 'other-chromium'
  if (/Orrery|electron/i.test(cmd) && !/--type=/.test(cmd)) return 'main'
  return 'pty-child'
}

function summarize(ticks, cmds, collector, fromT) {
  const roles = new Map()
  const pids = new Map()
  const roleOfPid = (pid) => roleOf(cmds.get(pid)?.cmd, pid, collector)
  for (let i = 1; i < ticks.length; i++) {
    const a = ticks[i - 1]
    const b = ticks[i]
    if (b.t < fromT) continue
    const dt = (b.t - a.t) / 1000
    if (dt <= 0) continue
    const prev = new Map(a.p.map((r) => [r[0], r]))
    const tickRoles = new Map()
    const add = (role, cpu, ws, pm) => {
      const x = tickRoles.get(role) ?? { cpu: 0, ws: 0, pm: 0, n: 0, cpuSec: 0 }
      x.cpu += cpu
      x.ws += ws
      x.pm += pm
      x.n++
      tickRoles.set(role, x)
    }
    for (const r of b.p) {
      const [pid, cpuSec, ws, pm] = r
      const role = roleOfPid(pid)
      const before = prev.get(pid)
      const d = before ? Math.max(0, cpuSec - before[1]) : 0
      const cpu = before ? (d / dt) * 100 : 0
      add(role, cpu, ws, pm)
      const px = pids.get(pid) ?? { role, cpu: [], ws: [], pm: [], cpuSec: 0, cmd: (cmds.get(pid)?.cmd ?? '').slice(0, 160) }
      if (before) px.cpu.push(cpu)
      px.ws.push(ws)
      px.pm.push(pm)
      px.cpuSec += d
      pids.set(pid, px)
    }
    const total = { cpu: 0, ws: 0, pm: 0, n: 0 }
    for (const [role, x] of tickRoles) {
      const acc = roles.get(role) ?? { cpu: [], ws: [], pm: [], n: [], cpuSec: 0 }
      acc.cpu.push(x.cpu)
      acc.ws.push(x.ws)
      acc.pm.push(x.pm)
      acc.n.push(x.n)
      acc.cpuSec += (x.cpu / 100) * dt
      roles.set(role, acc)
      total.cpu += x.cpu
      total.ws += x.ws
      total.pm += x.pm
      total.n += x.n
    }
    const acc = roles.get('TOTAL') ?? { cpu: [], ws: [], pm: [], n: [], cpuSec: 0 }
    acc.cpu.push(total.cpu)
    acc.ws.push(total.ws)
    acc.pm.push(total.pm)
    acc.n.push(total.n)
    acc.cpuSec += (total.cpu / 100) * dt
    roles.set('TOTAL', acc)
  }
  const out = {}
  const order = ['TOTAL', 'main', 'renderer', 'gpu', 'collector', 'pty-host', 'network', 'audio', 'crashpad', 'pty-child']
  const rank = (k) => (order.includes(k) ? order.indexOf(k) : 99)
  const keys = [...roles.keys()].sort((a, b) => rank(a) - rank(b))
  for (const k of keys) {
    const x = roles.get(k)
    out[k] = {
      procs: Math.round(avg(x.n) * 10) / 10,
      privateMB: mb(avg(x.pm)),
      privateMBMax: mb(Math.max(...x.pm)),
      rssMB: mb(avg(x.ws)),
      cpuAvgPct: round(avg(x.cpu)),
      cpuP95Pct: round(p95(x.cpu)),
      cpuSec: round(x.cpuSec),
    }
  }
  const perPid = [...pids.entries()].map(([pid, x]) => ({ pid, role: x.role, privateMB: mb(avg(x.pm)), rssMB: mb(avg(x.ws)), cpuAvgPct: round(avg(x.cpu)), cpuP95Pct: round(p95(x.cpu)), cpuSec: round(x.cpuSec), cmd: x.cmd }))
  perPid.sort((a, b) => b.cpuSec - a.cpuSec)
  return { roles: out, pids: perPid }
}

const TRACE_CATEGORIES = [
  'toplevel',
  'disabled-by-default-devtools.timeline',
  'disabled-by-default-devtools.timeline.stack',
  'devtools.timeline',
  'gpu',
  'cc',
  'viz',
  'blink',
  'v8',
  'benchmark',
]

const PAGE_PROBE = `(() => {
  if (window.__bench) return true
  const calls = new Map()
  const note = (kind) => {
    const lines = (new Error().stack || '').split('\\n').slice(3, 5).map((l) => l.trim()).join(' <- ')
    const k = kind + ' ' + lines
    calls.set(k, (calls.get(k) || 0) + 1)
  }
  const wrap = (name) => {
    const orig = window[name]
    window[name] = function (fn, ...rest) {
      note(name)
      return orig.call(this, fn, ...rest)
    }
  }
  wrap('requestAnimationFrame')
  wrap('setTimeout')
  wrap('setInterval')
  window.__bench = calls
  return true
})()`

const PAGE_STATE = `(() => {
  const desc = (el) => el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') + (typeof el.className === 'string' && el.className ? '.' + el.className.trim().split(/\\s+/).slice(0, 3).join('.') : '')
  const running = document.getAnimations().filter((a) => a.playState === 'running')
  const anim = {}
  for (const a of running) {
    const t = a.effect && a.effect.target
    const k = (a.animationName || a.transitionProperty || a.constructor.name) + ' on ' + (t ? desc(t) : '?')
    anim[k] = (anim[k] || 0) + 1
  }
  const backdrop = []
  const infinite = []
  let all = 0
  for (const el of document.querySelectorAll('*')) {
    all++
    const cs = getComputedStyle(el)
    const bf = cs.backdropFilter || cs.webkitBackdropFilter
    if (bf && bf !== 'none') {
      const r = el.getBoundingClientRect()
      if (r.width > 0 && r.height > 0) backdrop.push(desc(el) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height) + ' ' + bf)
    }
    if (cs.animationName && cs.animationName !== 'none' && cs.animationIterationCount === 'infinite') infinite.push(desc(el) + ' ' + cs.animationName + ' ' + cs.animationDuration)
  }
  const canvases = [...document.querySelectorAll('canvas')].map((c) => desc(c) + ' ' + c.width + 'x' + c.height + (c.offsetParent === null ? ' hidden' : ''))
  return { dom: all, runningAnimations: anim, backdropFilter: backdrop.slice(0, 40), infiniteCss: infinite.slice(0, 40), canvases: canvases.slice(0, 20), hidden: document.hidden, visibility: document.visibilityState }
})()`

const PAGE_CALLS = `(() => [...window.__bench.entries()].sort((a, b) => b[1] - a[1]).slice(0, 25))()`

const PAGE_REGIONS = `(() => {
  const regions = { header: 'header', rail: 'aside[data-tour=sessions]', map: '[data-tour=map]', activity: 'section[aria-labelledby=activity-title]' }
  const out = { total: document.querySelectorAll('*').length }
  let named = 0
  for (const [k, sel] of Object.entries(regions)) {
    let n = 0
    for (const r of document.querySelectorAll(sel)) n += 1 + r.querySelectorAll('*').length
    out[k] = n
    named += n
  }
  out.other = out.total - named
  const comps = new Set()
  for (const el of document.querySelectorAll('*')) {
    let c = el.__vueParentComponent
    while (c && !comps.has(c)) {
      comps.add(c)
      c = c.parent
    }
  }
  out.vueComponents = comps.size
  const byName = {}
  for (const c of comps) {
    const n = (c.type && (c.type.__name || c.type.name)) || '(anon)'
    byName[n] = (byName[n] || 0) + 1
  }
  out.topComponents = Object.entries(byName).sort((a, b) => b[1] - a[1]).slice(0, 15)
  return out
})()`

const patchStart = (S) => `(() => {
  const counts = {}
  const offs = []
  for (const [id, st] of ${S}.entries()) {
    counts[id] = { patches: 0, actions: 0 }
    offs.push(st.$subscribe(() => counts[id].patches++, { flush: 'sync', detached: true }))
    offs.push(st.$onAction(() => counts[id].actions++, true))
  }
  window.__patchStop = () => {
    for (const f of offs) f()
    return counts
  }
  return true
})()`

function analyzeProfile(prof, seconds) {
  const nodes = new Map(prof.nodes.map((n) => [n.id, n]))
  const parent = new Map()
  for (const n of prof.nodes) for (const c of n.children ?? []) parent.set(c, n.id)
  const keyOf = (n) => {
    const f = n.callFrame
    const file = String(f.url ?? '').split('/').slice(-2).join('/')
    return { key: `${f.functionName || '(anon)'} @ ${file}:${f.lineNumber + 1}:${f.columnNumber + 1}`, file: file || '(native)', name: f.functionName || '(anon)' }
  }
  const self = new Map()
  const total = new Map()
  const byFile = new Map()
  let sum = 0
  const dts = prof.timeDeltas
  for (let i = 0; i < prof.samples.length; i++) {
    const dt = (dts[i + 1] ?? dts[i] ?? 0) / 1000
    sum += dt
    const leaf = nodes.get(prof.samples[i])
    const lk = keyOf(leaf)
    self.set(lk.key, (self.get(lk.key) ?? 0) + dt)
    byFile.set(lk.file, (byFile.get(lk.file) ?? 0) + dt)
    const seen = new Set()
    for (let id = prof.samples[i]; id !== undefined; id = parent.get(id)) {
      const k = keyOf(nodes.get(id)).key
      if (seen.has(k)) continue
      seen.add(k)
      total.set(k, (total.get(k) ?? 0) + dt)
    }
  }
  const special = ['(idle)', '(program)', '(garbage collector)', '(root)']
  const specialMs = {}
  for (const [k, v] of self) {
    const name = k.split(' @ ')[0]
    if (special.includes(name)) specialMs[name] = round(v)
  }
  const isSpecial = (k) => special.includes(k.split(' @ ')[0])
  const pct = (v) => round((v / sum) * 100, 2)
  const top = (m) =>
    [...m.entries()]
      .filter(([k]) => !isSpecial(k))
      .sort((a, b) => b[1] - a[1])
      .slice(0, 30)
      .map(([k, v]) => ({ fn: k, ms: round(v), pctOfWindow: pct(v), msPerSec: round(v / seconds) }))
  const busy = sum - (specialMs['(idle)'] ?? 0)
  return {
    seconds,
    sampleCount: prof.samples.length,
    sampledMs: round(sum),
    busyPct: pct(busy),
    special: specialMs,
    topSelf: top(self),
    topTotal: top(total),
    byFile: [...byFile.entries()]
      .filter(([k]) => k !== '(native)')
      .sort((a, b) => b[1] - a[1])
      .slice(0, 20)
      .map(([k, v]) => ({ file: k, ms: round(v), pctOfWindow: pct(v) })),
  }
}

async function recordProfile(cdp, seconds, tmp, name) {
  await cdp.cmd('Profiler.enable')
  await cdp.cmd('Profiler.setSamplingInterval', { interval: 500 })
  await cdp.cmd('Profiler.start')
  await sleep(seconds * 1000)
  const r = await cdp.cmd('Profiler.stop')
  await cdp.cmd('Profiler.disable')
  const prof = r.result?.profile
  if (!prof) throw new Error('no profile: ' + JSON.stringify(r.error ?? r).slice(0, 200))
  const dir = PROFILE_DIR ? path.resolve(PROFILE_DIR) : tmp
  mkdirSync(dir, { recursive: true })
  const file = path.join(dir, `${name}.cpuprofile`)
  writeFileSync(file, JSON.stringify(prof))
  return { prof, file }
}

function analyzeTrace(file, seconds) {
  const data = JSON.parse(readFileSync(file, 'utf8'))
  const events = Array.isArray(data) ? data : data.traceEvents
  const procName = new Map()
  const threadName = new Map()
  for (const e of events) {
    if (e.ph !== 'M') continue
    if (e.name === 'process_name') procName.set(e.pid, e.args?.name)
    else if (e.name === 'thread_name') threadName.set(`${e.pid}:${e.tid}`, e.args?.name)
  }
  const kindOf = (pid) => {
    const n = procName.get(pid) ?? ''
    if (/GPU/i.test(n)) return 'gpu'
    if (/Renderer/i.test(n)) return 'renderer'
    if (/Browser/i.test(n)) return 'browser'
    return n || 'other'
  }
  const perThread = new Map()
  const names = new Map()
  const frames = new Map()
  const fnCalls = new Map()
  const timers = new Map()
  const timerInstall = new Map()
  const rafStacks = new Map()
  const FRAME_NAMES = new Set(['Display::DrawAndSwap', 'DrawFrame', 'ProxyMain::BeginMainFrame', 'Commit', 'RasterTask', 'FireAnimationFrame', 'UpdateLayoutTree', 'Paint', 'Layerize', 'PrePaint', 'TimerFire', 'FunctionCall', 'EventDispatch', 'MinorGC', 'MajorGC', 'RunTask'])
  const bump = (m, k, dur) => {
    const x = m.get(k) ?? { count: 0, ms: 0 }
    x.count++
    x.ms += dur / 1000
    m.set(k, x)
  }
  const stackTop = (st) => {
    const f = Array.isArray(st) ? st[0] : st?.callFrames?.[0]
    return f ? `${f.functionName || '(anon)'} @ ${String(f.url ?? '').split('/').slice(-2).join('/')}:${f.lineNumber}:${f.columnNumber}` : null
  }
  for (const e of events) {
    if (e.ph === 'M') continue
    const pid = e.pid
    const kind = kindOf(pid)
    const th = `${pid}:${kind}:${threadName.get(`${pid}:${e.tid}`) ?? e.tid}`
    if (e.ph === 'X') {
      if (e.name === 'ThreadControllerImpl::RunTask') {
        const x = perThread.get(th) ?? { runTaskMs: 0, tasks: 0 }
        x.runTaskMs += (e.dur ?? 0) / 1000
        x.tasks++
        perThread.set(th, x)
      }
      if (kind === 'renderer' || kind === 'gpu') bump(names, `${kind}|${threadName.get(`${pid}:${e.tid}`) ?? e.tid}|${e.name}`, e.dur ?? 0)
      if (FRAME_NAMES.has(e.name)) {
        const k = `${pid}:${kind}:${e.name}`
        frames.set(k, (frames.get(k) ?? 0) + 1)
      }
      if (e.name === 'FunctionCall' && e.args?.data?.url) {
        const d = e.args.data
        bump(fnCalls, `${d.functionName || '(anon)'} @ ${String(d.url).split('/').slice(-2).join('/')}:${d.lineNumber}:${d.columnNumber}`, e.dur ?? 0)
      }
      if (e.name === 'TimerFire') bump(timers, `timer ${e.args?.data?.timerId}`, e.dur ?? 0)
    }
    if (e.name === 'TimerInstall' && e.args?.data) {
      const d = e.args.data
      timerInstall.set(`timer ${d.timerId}`, { timeout: d.timeout, singleShot: d.singleShot, from: stackTop(d.stackTrace) })
    }
    if ((e.name === 'RequestAnimationFrame' || e.name === 'TimerInstall') && e.args?.data?.stackTrace) {
      const k = `${e.name} ${stackTop(e.args.data.stackTrace)}`
      rafStacks.set(k, (rafStacks.get(k) ?? 0) + 1)
    }
  }
  const top = (m, n, key = 'ms') => [...m.entries()].sort((a, b) => b[1][key] - a[1][key]).slice(0, n)
  const threads = [...perThread.entries()]
    .map(([k, v]) => ({ thread: k, busyMsPerSec: round(v.runTaskMs / seconds, 1), tasksPerSec: round(v.tasks / seconds, 1) }))
    .sort((a, b) => b.busyMsPerSec - a.busyMsPerSec)
    .slice(0, 14)
  const perPerSec = {}
  for (const [k, v] of frames) perPerSec[k] = round(v / seconds, 2)
  const topEvents = {}
  const byGroup = new Map()
  for (const [k, v] of names) {
    const [kind, thread] = k.split('|')
    const g = `${kind}|${thread}`
    if (!byGroup.has(g)) byGroup.set(g, [])
    byGroup.get(g).push([k.split('|')[2], v])
  }
  const groups = [...byGroup.entries()]
    .map(([g, list]) => ({ g, total: list.reduce((a, [, v]) => a + v.ms, 0), list }))
    .sort((a, b) => b.total - a.total)
    .slice(0, 8)
  for (const { g, list } of groups) topEvents[g] = list.sort((a, b) => b[1].ms - a[1].ms).slice(0, 8).map(([n, v]) => ({ name: n, perSec: round(v.count / seconds, 2), msPerSec: round(v.ms / seconds, 2) }))
  return {
    processes: Object.fromEntries([...procName.entries()].map(([pid, n]) => [pid, n])),
    threads,
    perSecond: perPerSec,
    topEventsByThread: topEvents,
    topFunctionCalls: top(fnCalls, 15).map(([k, v]) => ({ fn: k, perSec: round(v.count / seconds, 2), msPerSec: round(v.ms / seconds, 2) })),
    topTimers: top(timers, 10).map(([k, v]) => ({ timer: k, install: timerInstall.get(k) ?? null, perSec: round(v.count / seconds, 2), msPerSec: round(v.ms / seconds, 2) })),
    topInstallStacks: [...rafStacks.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12).map(([k, v]) => ({ site: k, perSec: round(v / seconds, 2) })),
  }
}

async function recordTrace(port, seconds, tmp) {
  const ver = await getJson(`http://127.0.0.1:${port}/json/version`)
  const br = await connect(ver.webSocketDebuggerUrl)
  const done = new Promise((r) => br.on('Tracing.tracingComplete', r))
  await br.cmd('Tracing.start', { traceConfig: { recordMode: 'recordUntilFull', includedCategories: TRACE_CATEGORIES }, transferMode: 'ReturnAsStream', streamFormat: 'json' })
  await sleep(seconds * 1000)
  await br.cmd('Tracing.end')
  const { stream } = await done
  const file = path.join(tmp, 'trace.json')
  const out = createWriteStream(file)
  for (;;) {
    const r = await br.cmd('IO.read', { handle: stream, size: 1 << 20 })
    const res = r.result ?? {}
    out.write(res.base64Encoded ? Buffer.from(res.data ?? '', 'base64') : (res.data ?? ''))
    if (res.eof || r.error) break
  }
  await new Promise((r) => out.end(r))
  await br.cmd('IO.close', { handle: stream })
  br.close()
  return file
}

function appVersion() {
  try {
    if (WIN && exeArg) {
      const v = execFileSync('powershell.exe', ['-NoProfile', '-Command', `(Get-Item -LiteralPath '${exeArg}').VersionInfo.ProductVersion`], { encoding: 'utf8' }).trim()
      if (v) return v
    }
  } catch {}
  return JSON.parse(readFileSync(path.join(ROOT, 'apps', 'desktop', 'package.json'), 'utf8')).version
}

async function runScenario(name, version) {
  const sc = SCENARIOS[name]
  const tmp = mkdtempSync(path.join(os.tmpdir(), 'perf-bench-'))
  const dirs = { ud: path.join(tmp, 'ud'), data: path.join(tmp, 'data'), claude: path.join(tmp, 'claude'), codex: path.join(tmp, 'codex'), codewhale: path.join(tmp, 'codewhale') }
  for (const d of Object.values(dirs)) mkdirSync(d, { recursive: true })
  mkdirSync(path.join(dirs.claude, 'sessions'), { recursive: true })
  writeFileSync(path.join(dirs.data, 'preview-full-access.json'), '{"full":false}')
  const dbg = await freePort()
  const port = await freePort()
  let fixture = null
  let app = null
  let stopSampler = () => {}
  let cdp = null
  try {
    if (sc.demo) {
      const [sessions, subagents] = sc.demo
      fixture = spawn(process.execPath, [path.join(ROOT, 'scripts', 'demo-fixture.mjs'), `--root=${dirs.claude}`, `--sessions=${sessions}`, `--subagents=${subagents}`, `--duration=${SETTLE + SECONDS + 400}`, ...(FIXTURE_SEED === null ? [] : [`--seed=${FIXTURE_SEED}`])], { stdio: ['ignore', 'pipe', 'ignore'] })
      lowPriority(fixture.pid)
      await new Promise((res, rej) => {
        const t = setTimeout(() => rej(new Error('fixture did not start')), 30000)
        readline.createInterface({ input: fixture.stdout }).on('line', (l) => {
          if (l.includes('fixture root')) (clearTimeout(t), res())
        })
      })
      await sleep(1500)
    }
    app = spawn(exe, [`--user-data-dir=${dirs.ud}`, `--remote-debugging-port=${dbg}`, ...exeArgs], {
      env: { ...process.env, CCM_PORT: String(port), CCM_DATA_DIR: dirs.data, CCM_TOAST: '0', CCM_VERIFY_PROCESSES: '0', CLAUDE_CONFIG_DIR: dirs.claude, CODEX_HOME: dirs.codex, CODEWHALE_HOME: dirs.codewhale },
      stdio: 'ignore',
      detached: !WIN,
    })
    lowPriority(app.pid)
    let target
    for (let i = 0; i < 120 && !target; i++) {
      await sleep(500)
      try {
        const list = await getJson(`http://127.0.0.1:${dbg}/json`)
        target = list.find((t) => t.type === 'page' && t.url.startsWith(`http://127.0.0.1:${port}/`))
      } catch {}
    }
    if (!target) throw new Error('no app page')
    cdp = await connect(target.webSocketDebuggerUrl)
    const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
    const W = `${S}.get('workspace')`
    const waitFor = async (expr, ms, label) => {
      const end = Date.now() + ms
      while (Date.now() < end) {
        try {
          if (await cdp.evaluate(expr)) return
        } catch {}
        await sleep(250)
      }
      throw new Error('timeout: ' + label)
    }
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 40000, 'app ready')
    await cdp.evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await cdp.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 40000, 'reloaded')
    await cdp.cmd('Page.bringToFront')
    await cdp.evaluate(`${S}.get('ui').setView('${sc.view}')`)
    if (sc.panes) {
      for (let i = 0; i < sc.panes; i++) await cdp.evaluate(`${W}.open('shell', ${JSON.stringify(os.tmpdir())}, undefined, ${i === 0 ? '{ tab: true }' : 'undefined'})`)
      await waitFor(`${W}.panes.length >= ${sc.panes} && ${W}.panes.every((p) => p.state === 'running')`, 40000, 'panes running')
    }
    if (sc.demo) await sleep(5000)

    const cmds = new Map()
    const ticks = []
    stopSampler = startSampler(
      app.pid,
      (list) => {
        for (const p of list) {
          if (!cmds.has(p.pid)) lowPriority(p.pid)
          cmds.set(p.pid, { ppid: p.ppid, cmd: p.cmd })
        }
      },
      (t, p) => ticks.push({ t, p }),
    )
    const settleEnd = Date.now() + SETTLE * 1000
    await sleep(SETTLE * 1000)
    const fromT = settleEnd
    await sleep(SECONDS * 1000)
    const collector = collectorPid(port)
    const result = { sampled: { seconds: SECONDS, settle: SETTLE, tickMs: TICK_MS, ticks: ticks.filter((x) => x.t >= fromT).length }, ...summarize(ticks, cmds, collector, fromT) }
    if (TRACE) {
      stopSampler()
      stopSampler = () => {}
      await cdp.evaluate(PAGE_PROBE)
      result.pageState = await cdp.evaluate(PAGE_STATE)
      const file = await recordTrace(dbg, TRACE_SECONDS, tmp)
      result.trace = { seconds: TRACE_SECONDS, ...analyzeTrace(file, TRACE_SECONDS) }
      result.trace.pageScheduling = await cdp.evaluate(PAGE_CALLS)
    }
    if (PROFILE && sc.demo) {
      stopSampler()
      stopSampler = () => {}
      const { prof, file } = await recordProfile(cdp, PROFILE_SECONDS, tmp, name)
      await cdp.evaluate(patchStart(S))
      await sleep(PATCH_SECONDS * 1000)
      const patches = await cdp.evaluate('window.__patchStop()')
      for (const v of Object.values(patches)) (v.patchesPerSec = round(v.patches / PATCH_SECONDS, 2), v.actionsPerSec = round(v.actions / PATCH_SECONDS, 2))
      result.profile = { ...analyzeProfile(prof, PROFILE_SECONDS), file: PROFILE_DIR ? file : '(temp, deleted)', storePatches: patches, dom: await cdp.evaluate(PAGE_REGIONS) }
      console.log('profile', name, 'busy', result.profile.busyPct + '%', 'saved', file)
    }
    if (COLLECTOR_PROF) result.collectorProfile = await profileCollector(dirs.claude, tmp)
    return result
  } finally {
    stopSampler()
    cdp?.close()
    killTree(app?.pid)
    killTree(fixture?.pid)
    await sleep(2500)
    for (let i = 0; i < 20; i++) {
      try {
        rmSync(tmp, { recursive: true, force: true })
        break
      } catch {
        await sleep(500)
      }
    }
  }
}

function table(name, r) {
  console.log(`\n== ${name}  (${r.sampled.seconds}s after ${r.sampled.settle}s settle, ${r.sampled.ticks} ticks; CPU % of one core)`)
  console.log(['role'.padEnd(11), 'procs'.padStart(5), 'private MB'.padStart(11), 'max MB'.padStart(8), 'rss MB'.padStart(8), 'cpu avg'.padStart(8), 'cpu p95'.padStart(8), 'cpu s'.padStart(7)].join(' '))
  for (const [k, v] of Object.entries(r.roles)) console.log([k.padEnd(11), String(v.procs).padStart(5), String(v.privateMB).padStart(11), String(v.privateMBMax).padStart(8), String(v.rssMB).padStart(8), String(v.cpuAvgPct).padStart(8), String(v.cpuP95Pct).padStart(8), String(v.cpuSec).padStart(7)].join(' '))
}

const version = appVersion()
const results = {}
const failures = {}
for (const name of wanted) {
  try {
    results[name] = await runScenario(name, version)
    table(name, results[name])
  } catch (e) {
    failures[name] = e.message
    console.log(`\n== ${name}: FAILED ${e.message}`)
  }
}

const label = opt('label', version)
const out = opt('out', path.join(ROOT, 'docs', 'perf', `${label}-${process.platform}.json`))
mkdirSync(path.dirname(path.resolve(out)), { recursive: true })
writeFileSync(
  out,
  JSON.stringify({ version, label, platform: process.platform, date: new Date().toISOString().slice(0, 10), exe: exeArg || 'electron dev (apps/desktop/stage)', cores: os.cpus().length, totalMemMB: Math.round(os.totalmem() / 1048576), cpuUnit: 'percent of one core', seconds: SECONDS, settle: SETTLE, scenarios: results, failures }, null, 2),
)
console.log(`\nwrote ${path.relative(ROOT, path.resolve(out))}`)
