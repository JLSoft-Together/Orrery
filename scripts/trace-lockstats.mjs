import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import net from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import WebSocket from 'ws'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const DEMO = path.join(REPO, 'scripts', 'demo-fixture.mjs')
const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, '').split('=')
    return [k, v ?? 'true']
  }),
)
const SESSIONS = Number(args.sessions ?? 30)
const SUBAGENTS = Number(args.subagents ?? 6)
const TICK_MS = Number(args.tick ?? 400)
const SECONDS = Number(args.seconds ?? 60)
const SEED = Number(args.seed ?? 8704)
const BIN = args['rust-bin'] ?? process.env.CCM_CORE_BIN ?? ['release', 'debug'].map((d) => path.join(REPO, 'apps', 'core', 'target', d, process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core')).find((p) => existsSync(p))
if (!BIN) throw new Error('orrery-core binary not found: cargo build --release --locked -j 4 -p core-bin, or --rust-bin=')

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

async function main() {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'ccm-lockstats-'))
  const root = path.join(tmp, 'root')
  const data = path.join(tmp, 'data')
  await mkdir(data, { recursive: true })
  await writeFile(path.join(tmp, 'res-fixture.json'), JSON.stringify({ procs: [], ports: {} }))
  const demo = spawn(process.execPath, [DEMO, `--root=${root}`, `--sessions=${SESSIONS}`, `--subagents=${SUBAGENTS}`, `--seed=${SEED}`, `--tick=${TICK_MS}`, `--duration=${SECONDS + 10}`, '--peers=10'], { stdio: 'ignore', windowsHide: true })
  let serve = null
  let ws = null
  try {
    await sleep(1500)
    const port = await freePort()
    if (port === 4317) throw new Error('refusing port 4317')
    const env = { ...process.env }
    for (const k of ['CCM_APP_KEY', 'CCM_TOAST_LINK', 'CCM_HOST_RPC']) delete env[k]
    Object.assign(env, {
      CLAUDE_CONFIG_DIR: root,
      CCM_DATA_DIR: data,
      CCM_PORT: String(port),
      CODEX_HOME: path.join(tmp, 'codex'),
      CODEWHALE_HOME: path.join(tmp, 'codewhale'),
      CCM_CODEX: '0',
      CCM_DEEPSEEK: '0',
      CCM_TOAST: '0',
      CCM_VERIFY_PROCESSES: '0',
      CCM_RES_FIXTURE: path.join(tmp, 'res-fixture.json'),
      CCM_WEB_DIST: path.join(tmp, 'web'),
      CCM_SERVE_STATS: '1',
    })
    serve = spawn(BIN, ['serve', '--dev'], { cwd: REPO, env, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    const err = []
    serve.stderr.on('data', (d) => err.push(String(d)))
    serve.stdout.on('data', () => undefined)
    const exited = new Promise((resolve) => serve.once('exit', resolve))
    const deadline = Date.now() + 30_000
    for (;;) {
      if (serve.exitCode !== null) throw new Error(`serve exited ${serve.exitCode}\n${err.join('')}`)
      const ok = await fetch(`http://127.0.0.1:${port}/health`).then((r) => r.ok).catch(() => false)
      if (ok) break
      if (Date.now() > deadline) throw new Error('serve did not answer /health')
      await sleep(200)
    }
    ws = new WebSocket(`ws://127.0.0.1:${port}/ws`)
    let frames = 0
    ws.on('message', () => frames++)
    const health = []
    const until = Date.now() + SECONDS * 1000
    while (Date.now() < until) {
      const t = performance.now()
      await fetch(`http://127.0.0.1:${port}/health`).catch(() => undefined)
      health.push(performance.now() - t)
      await sleep(500)
    }
    ws.close()
    serve.stdin.end(`${JSON.stringify({ type: 'shutdown' })}\n`)
    await Promise.race([exited, sleep(8000)])
    if (args.log) await writeFile(path.resolve(args.log), err.join(''))
    const line = err.join('').split(/\r?\n/).find((l) => l.includes('"serve.stats"'))
    health.sort((a, b) => a - b)
    const q = (p) => Math.round(health[Math.min(health.length - 1, Math.round((health.length - 1) * p))] * 10) / 10
    console.log(JSON.stringify({ sessions: SESSIONS, subagents: SUBAGENTS, tickMs: TICK_MS, seconds: SECONDS, frames, health: { n: health.length, p50: q(0.5), p99: q(0.99), max: q(1) }, stats: line ? JSON.parse(line) : null }))
  } finally {
    ws?.terminate()
    if (serve && serve.exitCode === null) serve.kill()
    if (demo.exitCode === null) await Promise.race([new Promise((resolve) => demo.once('exit', resolve)), sleep(30_000)])
    if (demo.exitCode === null) demo.kill()
    await sleep(500)
    await rm(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => undefined)
  }
}

main().catch((e) => {
  console.error(`[trace-lockstats] ${e instanceof Error ? e.message : e}`)
  process.exit(1)
})
