const { execFileSync, spawnSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const argv = process.argv.slice(2)
const opt = (name, def) => {
  const hit = argv.find((a) => a.startsWith(`--${name}=`))
  return hit ? hit.slice(name.length + 3) : def
}
const MODE = opt('mode', '')
const ROUNDS = Number(opt('rounds', 2))
const SAMPLES = Number(opt('samples', 300))
const FLOOD_MB = Number(opt('flood-mb', 128))
const ROOT = path.resolve(__dirname, '../../..')
const BASE_OUT = process.env.CCM_E2E_OUT || path.join(os.tmpdir(), 'ccm-e2e-bench')
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const round = (n, d = 1) => Math.round(n * 10 ** d) / 10 ** d
const pct = (xs, p) => {
  const s = [...xs].sort((a, b) => a - b)
  return s.length ? s[Math.min(s.length - 1, Math.ceil(s.length * p) - 1)] : 0
}
const psj = (script) => JSON.parse(execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }))

function orchestrate() {
  const runs = { node: [], rust: [] }
  for (let r = 0; r < ROUNDS; r++) {
    for (const mode of ['node', 'rust']) {
      const out = path.join(BASE_OUT, `${mode}-${r}`)
      fs.rmSync(out, { recursive: true, force: true })
      fs.mkdirSync(out, { recursive: true })
      console.log(`# round ${r + 1}/${ROUNDS} ${mode}`)
      const res = spawnSync(process.execPath, [__filename, `--mode=${mode}`, `--samples=${SAMPLES}`, `--flood-mb=${FLOOD_MB}`], {
        env: { ...process.env, CCM_CORE_PTY: mode, CCM_E2E_OUT: out },
        stdio: ['ignore', 'inherit', 'inherit'],
        encoding: 'utf8',
      })
      const file = path.join(out, 'bench.json')
      if (res.status !== 0 || !fs.existsSync(file)) throw new Error(`${mode} round ${r} failed (status ${res.status})`)
      runs[mode].push(JSON.parse(fs.readFileSync(file, 'utf8')))
    }
  }
  const med = (mode, pick) => {
    const xs = runs[mode].map(pick).sort((a, b) => a - b)
    return xs[Math.floor(xs.length / 2)]
  }
  const summary = {}
  for (const mode of ['node', 'rust']) {
    summary[mode] = {
      echoP50Ms: med(mode, (x) => x.echo.p50),
      echoP90Ms: med(mode, (x) => x.echo.p90),
      echoP99Ms: med(mode, (x) => x.echo.p99),
      rss1PaneMB: med(mode, (x) => x.rss1.workingSetMB),
      private1PaneMB: med(mode, (x) => x.rss1.privateMB),
      rss8PanesMB: med(mode, (x) => x.rss8.workingSetMB),
      private8PanesMB: med(mode, (x) => x.rss8.privateMB),
      floodCpuSeconds: med(mode, (x) => x.flood.cpuSeconds),
      floodCpuPctOfOneCore: med(mode, (x) => x.flood.cpuPct),
      floodPeakWorkingSetMB: med(mode, (x) => x.flood.peakWorkingSetMB),
      floodMBPerSecond: med(mode, (x) => x.flood.mbPerSecond),
    }
  }
  const coreExe = path.join(ROOT, 'apps', 'core', 'target', 'release', 'orrery-core.exe')
  const doc = {
    label: 'v87-step2-core-pty',
    platform: process.platform,
    node: process.version,
    cpus: os.cpus().length,
    cpuModel: os.cpus()[0]?.model,
    totalMemMB: Math.round(os.totalmem() / 1048576),
    generated: new Date().toISOString(),
    coreBinaryBytes: fs.existsSync(coreExe) ? fs.statSync(coreExe).size : null,
    params: { rounds: ROUNDS, echoSamples: SAMPLES, floodMB: FLOOD_MB },
    summary,
    runs,
  }
  const target = opt('out', path.join(ROOT, 'docs', 'perf', 'v87-step2-core-pty-win32.json'))
  fs.mkdirSync(path.dirname(target), { recursive: true })
  fs.writeFileSync(target, JSON.stringify(doc, null, 2) + '\n')
  console.log(JSON.stringify(summary, null, 2))
  console.log(`wrote ${target}`)
}

async function bench() {
  process.env.CCM_CORE_PTY = MODE
  const { launch, OUT } = require('./cdp.cjs')
  const WS = require(path.join(ROOT, 'node_modules/ws'))
  const PORT = Number(process.env.CCM_E2E_PORT) || 4399
  const tools = path.join(OUT, 'tools')
  fs.mkdirSync(tools, { recursive: true })
  const echoJs = path.join(tools, 'echo.js')
  const floodJs = path.join(tools, 'flood.js')
  fs.writeFileSync(echoJs, "process.stdin.setRawMode(true);process.stdin.resume();process.stdin.on('data',(d)=>process.stdout.write(d));process.stdout.write('ECHO-READY\\n')\n")
  fs.writeFileSync(
    floodJs,
    `const chunk = Buffer.alloc(65536, 'abcdefghijklmnopqrstuvwxyz0123456789\\n');let left = ${FLOOD_MB} * 16;const t0 = Date.now();process.stdout.write('FLOOD-START\\n');(function pump(){while(left>0){left--;if(!process.stdout.write(chunk)){process.stdout.once('drain',pump);return}}process.stdout.write('\\nFLOOD-DONE '+(Date.now()-t0)+'\\n')})()\n`,
  )
  const app = await launch()
  const { evaluate, waitFor } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    const ep = await evaluate(`window.ccmDesktop.ptyEndpoint()`)
    const origin = `http://127.0.0.1:${PORT}`
    class Pane {
      constructor() {
        this.ws = new WS(ep.url, [ep.protocol], { origin })
        this.chunks = []
        this.ready = null
        this.onBinary = null
        this.watch = new Set(['ECHO-READY', 'FLOOD-START', 'FLOOD-DONE'])
        this.found = new Set()
        this.ws.on('message', (data, binary) => {
          if (binary) {
            const buf = Buffer.from(data)
            this.bytes = (this.bytes ?? 0) + buf.length
            const joined = Buffer.concat([this.tail ?? Buffer.alloc(0), buf])
            this.tail = joined.subarray(-256)
            if (this.watch.size) {
              const text = joined.subarray(0, Math.min(joined.length, 4096 + 256)).toString('latin1')
              for (const w of this.watch) if (text.includes(w)) this.found.add(w)
            }
            this.onBinary?.(buf)
          } else {
            const m = JSON.parse(data.toString('utf8'))
            if (m.t === 'ready') this.ready = m
            if (m.t === 'ready' && m.pid > 0) this.pid = m.pid
            if (m.t === 'pid') this.pid = m.pid
          }
        })
      }
      async start() {
        await new Promise((resolve, reject) => (this.ws.once('open', resolve), this.ws.once('error', reject)))
        this.ws.send(JSON.stringify({ t: 'spawn', preset: 'shell', cols: 120, rows: 30, cwd: os.tmpdir() }))
        const end = Date.now() + 20000
        while (!this.ready && Date.now() < end) await sleep(50)
        if (!this.ready) throw new Error('pane did not become ready')
        while (!this.pid && Date.now() < end) await sleep(50)
        return { ...this.ready, pid: this.pid ?? 0 }
      }
      write(text) {
        this.ws.send(Buffer.from(text, 'utf8'))
      }
      async prompt(tag) {
        const end = Date.now() + 40000
        while (Date.now() < end) {
          this.write(`echo ('${tag}'+'-ok')\r`)
          if (await this.seen(`${tag}-ok`, 2000)) return true
        }
        return false
      }
      async seen(text, ms = 20000) {
        const end = Date.now() + ms
        while (Date.now() < end) {
          if (this.found.has(text) || (this.tail && this.tail.toString('latin1').includes(text))) return true
          await sleep(25)
        }
        return false
      }
    }
    const hostPidOf = (shellPid) => {
      const procs = psj('Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name | ConvertTo-Json -Compress')
      const me = procs.find((p) => p.ProcessId === shellPid)
      return { host: me?.ParentProcessId ?? 0, name: procs.find((p) => p.ProcessId === me?.ParentProcessId)?.Name ?? '' }
    }
    const memOf = (pid) => {
      const r = psj(`$p = Get-Process -Id ${pid}; [pscustomobject]@{ ws = $p.WorkingSet64; priv = $p.PrivateMemorySize64; peak = $p.PeakWorkingSet64; cpu = $p.TotalProcessorTime.TotalSeconds } | ConvertTo-Json -Compress`)
      return { workingSetMB: round(r.ws / 1048576), privateMB: round(r.priv / 1048576), peakWorkingSetMB: round(r.peak / 1048576), cpuSeconds: r.cpu }
    }

    const first = new Pane()
    const ready = await first.start()
    const host = hostPidOf(ready.pid)
    console.log(`host process of the first shell: pid ${host.host} (${host.name})`)
    first.write(`node '${echoJs}'\r`)
    if (!(await first.seen('ECHO-READY'))) throw new Error('echo helper did not start')
    await sleep(3000)
    const rss1 = memOf(host.host)

    const lat = []
    for (let i = 0; i < SAMPLES + 20; i++) {
      const ch = String.fromCharCode(97 + (i % 26))
      const t0 = process.hrtime.bigint()
      await new Promise((resolve) => {
        const timer = setTimeout(resolve, 2000)
        first.onBinary = (buf) => {
          if (buf.includes(ch)) {
            clearTimeout(timer)
            first.onBinary = null
            if (i >= 20) lat.push(Number(process.hrtime.bigint() - t0) / 1e6)
            resolve()
          }
        }
        first.write(ch)
      })
      await sleep(15)
    }
    const echo = { samples: lat.length, p50: round(pct(lat, 0.5), 2), p90: round(pct(lat, 0.9), 2), p99: round(pct(lat, 0.99), 2), max: round(Math.max(...lat), 2), mean: round(lat.reduce((a, b) => a + b, 0) / lat.length, 2) }
    console.log(`echo ${JSON.stringify(echo)}`)

    const panes = [first]
    for (let i = 1; i < 8; i++) {
      const p = new Pane()
      await p.start()
      panes.push(p)
    }
    await sleep(4000)
    const rss8 = memOf(host.host)

    const flooder = panes[1]
    if (!(await flooder.prompt('flooder'))) throw new Error('flood pane never answered')
    flooder.write(`node '${floodJs}'\r`)
    if (!(await flooder.seen('FLOOD-START'))) throw new Error('flood helper did not start')
    const before = memOf(host.host)
    const bytes0 = flooder.bytes ?? 0
    const t0 = Date.now()
    const done = await flooder.seen('FLOOD-DONE', 120000)
    const wall = (Date.now() - t0) / 1000
    const after = memOf(host.host)
    const gotMB = ((flooder.bytes ?? 0) - bytes0) / 1048576
    const flood = {
      completed: done,
      wallSeconds: round(wall, 2),
      deliveredMB: round(gotMB),
      mbPerSecond: round(gotMB / wall),
      cpuSeconds: round(after.cpuSeconds - before.cpuSeconds, 2),
      cpuPct: round(((after.cpuSeconds - before.cpuSeconds) / wall) * 100),
      peakWorkingSetMB: after.peakWorkingSetMB,
      workingSetAfterMB: after.workingSetMB,
    }
    console.log(`flood ${JSON.stringify(flood)}`)
    fs.writeFileSync(path.join(OUT, 'bench.json'), JSON.stringify({ mode: MODE, hostName: host.name, rss1, rss8, echo, flood }, null, 2))
    for (const p of panes) p.ws.close()
    await sleep(800)
  } finally {
    app.stop()
  }
}

if (MODE) {
  bench().then(
    () => process.exit(0),
    (e) => {
      console.error('bench failed:', e.message)
      process.exit(1)
    },
  )
} else orchestrate()
