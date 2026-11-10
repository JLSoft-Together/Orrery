import { execFileSync, spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { appendFile, mkdir, mkdtemp, readdir, readFile, rm, utimes, writeFile } from 'node:fs/promises'
import net from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import WebSocket from 'ws'
import { TRACE_FORMAT } from './trace-compare.mjs'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const DEMO = path.join(REPO, 'scripts', 'demo-fixture.mjs')
const ENTRY = path.join(REPO, 'apps', 'collector', 'src', 'index.ts')
const CORE_KEYS = ['CCM_CORE_PROCS', 'CCM_CORE_USAGE', 'CCM_CORE_TRANSCRIPT', 'CCM_CORE_SIDEFILES', 'CCM_CORE_CODEX']

const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, '').split('=')
    return [k, v ?? 'true']
  }),
)
const OUT = args.out ? path.resolve(args.out) : null
const SESSIONS = Number(args.sessions ?? 30)
const SUBAGENTS = Number(args.subagents ?? 6)
const SEED = Number(args.seed ?? 8704)
const STEPS = Number(args.steps ?? 10)
const QUIET_MS = Number(args.quiet ?? 800)
const MAX_WAIT_MS = Number(args.maxWait ?? 60_000)
const STEP_MIN_MS = Number(args.stepMin ?? 5_700)
const GAP_MS = Number(args.gap ?? 40)
const NUDGE_MS = Number(args.nudge ?? 500)
const SPLIT_MS = Number(args.split ?? 250)
const CORE = args.core ?? 'node'
const ENDED = Math.min(3, SESSIONS)

if (!OUT) {
  console.error('usage: node scripts/trace-capture.mjs --out=<file.trace.json> [--sessions=30 --subagents=6 --seed=8704 --steps=10 --quiet=800 --core=node|inherit --keep]')
  process.exit(2)
}
if (CORE !== 'node' && CORE !== 'inherit') throw new Error('--core must be node or inherit')

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

function freePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer()
    srv.once('error', reject)
    srv.listen(0, '127.0.0.1', () => {
      const { port } = srv.address()
      srv.close(() => resolve(port))
    })
  })
}

async function walk(d, out = []) {
  for (const e of await readdir(d, { withFileTypes: true })) {
    const p = path.join(d, e.name)
    if (e.isDirectory()) await walk(p, out)
    else out.push(p)
  }
  return out
}

function sleeper() {
  const child =
    process.platform === 'win32'
      ? spawn(path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'PING.EXE'), ['-n', '86400', '127.0.0.1'], { stdio: 'ignore', windowsHide: true })
      : spawn('sleep', ['86400'], { stdio: 'ignore' })
  if (!child.pid) throw new Error('could not start a placeholder process')
  return child
}

async function loadSource(root) {
  const projects = path.join(root, 'projects')
  const jsonl = []
  const metas = []
  for (const f of (await walk(projects)).sort()) {
    const rel = path.relative(projects, f)
    const text = await readFile(f, 'utf8')
    if (f.endsWith('.jsonl')) jsonl.push({ rel, lines: text.split(/(?<=\n)/).filter(Boolean) })
    else if (f.endsWith('.meta.json')) metas.push({ rel, text, toolUseId: JSON.parse(text).toolUseId })
  }
  const registry = []
  for (const f of (await readdir(path.join(root, 'sessions'))).sort()) registry.push(JSON.parse(await readFile(path.join(root, 'sessions', f), 'utf8')))
  registry.sort((a, b) => a.name.localeCompare(b.name) || a.startedAt - b.startedAt)
  return { jsonl, metas, registry }
}

function sessionKey(rel) {
  const [slug, second] = rel.split(path.sep)
  return `${slug}/${second.replace(/\.jsonl$/, '')}`
}

function replayer(source, live) {
  const copied = new Map()
  const placed = new Set()
  const seen = new Set()
  const groups = new Map()
  const group = (rel) => {
    const key = sessionKey(rel)
    if (!groups.has(key)) groups.set(key, { subs: [], main: null, metas: [] })
    return groups.get(key)
  }
  for (const src of source.jsonl) {
    const g = group(src.rel)
    if (src.rel.includes(`${path.sep}subagents${path.sep}`)) g.subs.push(src)
    else g.main = src
  }
  for (const meta of source.metas) group(meta.rel).metas.push(meta)
  const append = async (src, k) => {
    const upto = Math.ceil((src.lines.length * k) / STEPS)
    const from = copied.get(src.rel) ?? 0
    const target = path.join(live, 'projects', src.rel)
    if (from === 0) await mkdir(path.dirname(target), { recursive: true })
    if (upto <= from && existsSync(target)) return null
    const chunk = src.lines.slice(from, upto).join('')
    await appendFile(target, chunk)
    for (const m of chunk.matchAll(/"id":"(toolu_[^"]+)","name":"Agent"/g)) seen.add(m[1])
    copied.set(src.rel, upto)
    return target
  }
  return async (k) => {
    const touched = []
    for (const g of groups.values()) {
      const files = []
      touched.push(files)
      for (const src of g.subs) if (placed.has(src.rel)) files.push(await append(src, k))
      if (files.some(Boolean)) await sleep(SPLIT_MS)
      if (g.main) files.push(await append(g.main, k))
      for (const meta of g.metas) {
        const jsonlRel = meta.rel.replace('.meta.json', '.jsonl')
        if (placed.has(jsonlRel) || !meta.toolUseId || !seen.has(meta.toolUseId)) continue
        const target = path.join(live, 'projects', meta.rel)
        await mkdir(path.dirname(target), { recursive: true })
        await writeFile(target, meta.text)
        await writeFile(path.join(live, 'projects', jsonlRel), '', { flag: 'a' })
        copied.set(jsonlRel, 0)
        placed.add(jsonlRel)
      }
      await sleep(GAP_MS)
    }
    if (NUDGE_MS <= 0) return
    await sleep(NUDGE_MS)
    for (const files of touched) {
      const at = new Date()
      for (const file of files) if (file) await utimes(file, at, at)
      await sleep(GAP_MS)
    }
  }
}

async function main() {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'ccm-trace-'))
  const src = path.join(tmp, 'src')
  const live = path.join(tmp, 'live')
  const data = path.join(tmp, 'data')
  const sleepers = []
  let collector = null
  let ws = null
  try {
    execFileSync(process.execPath, [DEMO, `--root=${src}`, `--sessions=${SESSIONS}`, `--subagents=${SUBAGENTS}`, `--seed=${SEED}`, '--tick=5', '--duration=1', '--peers=10'], { stdio: 'ignore', timeout: 120_000 })
    const source = await loadSource(src)
    await mkdir(path.join(live, 'sessions'), { recursive: true })
    await mkdir(data, { recursive: true })
    await writeFile(path.join(tmp, 'res-fixture.json'), JSON.stringify({ procs: [], ports: {} }))
    const pids = {}
    const sessions = source.registry.map((r, i) => {
      const child = sleeper()
      sleepers.push(child)
      pids[String(child.pid)] = `p${i}`
      const record = { ...r, pid: child.pid }
      return { record, child, id: `${child.pid}:${r.procStart ?? r.startedAt ?? 0}`, file: path.join(live, 'sessions', `${child.pid}.json`) }
    })
    const place = replayer(source, live)
    const late = Math.max(2, Math.floor(STEPS / 2))
    await place(1)
    for (const [i, s] of sessions.entries()) if (i % 2 === 0) await writeFile(s.file, JSON.stringify(s.record))

    const port = await freePort()
    if (port === 4317) throw new Error('refusing port 4317')
    const env = { ...process.env }
    for (const k of ['CCM_APP_KEY', 'CCM_TOAST_LINK', 'CCM_RES_FIXTURE']) delete env[k]
    if (CORE === 'node') {
      delete env.CCM_CORE_BIN
      for (const k of CORE_KEYS) env[k] = 'node'
    }
    Object.assign(env, {
      CLAUDE_CONFIG_DIR: live,
      CODEX_HOME: path.join(tmp, 'codex'),
      CODEWHALE_HOME: path.join(tmp, 'codewhale'),
      CCM_DATA_DIR: data,
      CCM_PORT: String(port),
      CCM_CODEX: '0',
      CCM_DEEPSEEK: '0',
      CCM_TOAST: '0',
      CCM_VERIFY_PROCESSES: '0',
      CCM_RES_FIXTURE: path.join(tmp, 'res-fixture.json'),
      CCM_STATUSLINE_COMMAND: 'node statusline-bridge.mjs',
      CCM_STATUSLINE_BRIDGE: path.join(tmp, 'statusline-bridge.mjs'),
      CCM_WEB_DIST: path.join(tmp, 'web'),
    })
    const log = []
    collector = spawn(process.execPath, ['--import', 'tsx', ENTRY, '--dev'], { cwd: REPO, env, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true })
    collector.stdout.on('data', (d) => log.push(String(d)))
    collector.stderr.on('data', (d) => log.push(String(d)))
    const exited = new Promise((resolve) => collector.once('exit', resolve))

    const wallStart = Date.now()
    const deadline = Date.now() + MAX_WAIT_MS
    for (;;) {
      if (collector.exitCode !== null) throw new Error(`collector exited ${collector.exitCode}\n${log.join('')}`)
      try {
        const res = await fetch(`http://127.0.0.1:${port}/health`)
        if (res.ok) break
      } catch {}
      if (Date.now() > deadline) throw new Error(`collector did not answer /health\n${log.join('')}`)
      await sleep(200)
    }

    const steps = []
    let current = null
    let lastAt = Date.now()
    let loading = true
    let scanning = false
    ws = new WebSocket(`ws://127.0.0.1:${port}/ws`)
    ws.on('message', (raw) => {
      const msg = JSON.parse(String(raw))
      lastAt = Date.now()
      for (const e of msg.events) {
        if (e.type === 'snapshot') {
          loading = e.payload.loading === true
          scanning = e.payload.usage?.scan?.state === 'scanning'
        }
        if (e.type === 'sessions.loading') loading = e.payload.loading
        if (e.type === 'usage.scan' || e.type === 'usage.reset') scanning = (e.payload.scan ?? e.payload).state === 'scanning'
      }
      current.frames.push(msg)
    })
    const begin = (name) => {
      current = { name, sent: [], frames: [] }
      steps.push(current)
    }
    const send = (m) => {
      current.sent.push(m)
      ws.send(JSON.stringify(m))
    }
    const settle = async (minMs = 300) => {
      const started = Date.now()
      await sleep(minMs)
      while (Date.now() - started < MAX_WAIT_MS) {
        if (!loading && !scanning && Date.now() - lastAt >= QUIET_MS) return
        await sleep(50)
      }
      throw new Error(`step ${current.name} did not settle in ${MAX_WAIT_MS} ms`)
    }

    begin('boot')
    await new Promise((resolve, reject) => {
      ws.once('open', resolve)
      ws.once('error', reject)
    })
    await settle(STEP_MIN_MS)
    begin('snapshot-boot')
    send({ type: 'resync' })
    await settle()
    for (let k = 2; k <= STEPS; k++) {
      begin(`replay-${k}`)
      await place(k)
      if (k === late) for (const [i, s] of sessions.entries()) if (i % 2 === 1) await writeFile(s.file, JSON.stringify(s.record))
      await settle(STEP_MIN_MS)
    }
    begin('client')
    const pick = (i) => sessions[Math.min(i, sessions.length - 1)].id
    send({ type: 'terminal.alias', terminalId: pick(4), alias: 'Trace alias' })
    send({ type: 'terminal.note', terminalId: pick(4), note: 'trace note' })
    send({ type: 'terminal.snooze', terminalId: pick(5), minutes: 30 })
    send({ type: 'favorite.toggle', terminalId: pick(6) })
    send({ type: 'favorite.add', dir: live })
    send({ type: 'terminal.alias', terminalId: 'nope:0', alias: 'x' })
    await settle(STEP_MIN_MS)
    begin('end')
    for (const s of sessions.slice(0, ENDED)) {
      await rm(s.file, { force: true })
      s.child.kill()
    }
    await settle(STEP_MIN_MS)
    begin('queries')
    send({ type: 'history.get' })
    send({ type: 'timeline.get' })
    send({ type: 'toolSpans.get' })
    send({ type: 'toolSpans.get', terminalId: pick(4) })
    send({ type: 'diagnostics.get' })
    send({ type: 'toolstats.get', days: 7, project: null })
    send({ type: 'heatmap.get' })
    await settle()
    begin('dismiss')
    send({ type: 'terminal.dismiss' })
    await settle()
    begin('final')
    send({ type: 'resync' })
    await settle()
    const wallEnd = Date.now()

    ws.close()
    collector.kill()
    await Promise.race([exited, sleep(5000)])
    const trace = {
      format: TRACE_FORMAT,
      scenario: `demo-${SESSIONS}x${SUBAGENTS}-seed${SEED}-steps${STEPS}`,
      source: 'node',
      core: CORE,
      wall: { start: wallStart, end: wallEnd },
      pids,
      replace: [
        [live, '<root>'],
        [data, '<data>'],
        [tmp, '<tmp>'],
      ],
      ignore: ['resources'],
      durations: ['workingMs', 'waitingMs', 'totalMs', 'medianMs'],
      steps,
    }
    await mkdir(path.dirname(OUT), { recursive: true })
    await writeFile(OUT, JSON.stringify(trace))
    const frames = steps.reduce((n, s) => n + s.frames.length, 0)
    const events = steps.reduce((n, s) => n + s.frames.reduce((m, f) => m + f.events.length, 0), 0)
    console.log(`[trace-capture] ${trace.scenario}: ${steps.length} steps, ${frames} frames, ${events} events, ${((wallEnd - wallStart) / 1000).toFixed(1)} s → ${OUT}`)
  } finally {
    ws?.terminate()
    if (collector && collector.exitCode === null) collector.kill()
    for (const c of sleepers) if (c.exitCode === null) c.kill()
    if (!args.keep) await rm(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => undefined)
    else console.log(`[trace-capture] kept ${tmp}`)
  }
}

main().catch((err) => {
  console.error(`[trace-capture] ${err instanceof Error ? err.message : err}`)
  process.exit(1)
})
