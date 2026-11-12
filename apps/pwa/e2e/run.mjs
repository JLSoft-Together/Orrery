import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { cleanup, killTree, startRelay, waitHealth } from './lib/procs.mjs'
import { runScenario, STEPS_PER_BROWSER } from './lib/scenario.mjs'
import { writeBlankVideo } from './lib/y4m.mjs'
import { startStatic } from './lib/server.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const pwa = resolve(here, '..')
const root = resolve(pwa, '../..')
const flags = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, '').split('=')
    return [k, v ?? '1']
  }),
)
const browsers = (flags.browsers ?? 'webkit,chromium').split(',')
const relayPort = Number(flags.relayPort ?? 8791)
const inspectorPort = Number(flags.inspectorPort ?? 9331)
const relayHttp = `http://127.0.0.1:${relayPort}`
const exe = process.platform === 'win32' ? 'pwa-e2e-peer.exe' : 'pwa-e2e-peer'
const peerExe = join(root, 'apps/core/target/debug', exe)
const qaDir = join(pwa, 'e2e/.out/shots-55a')
const qaDirB = join(root, 'docs/qa/v98/WP-55b')
const results = []
const videoPath = join(tmpdir(), 'orrery-pwa-fake-camera.y4m')

function sh(cmd, args, options = {}) {
  const r = spawnSync(cmd, args, { stdio: 'inherit', ...options })
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(' ')} failed (${r.status})`)
}

async function waitForCargo() {
  const end = Date.now() + 10 * 60_000
  while (Date.now() < end) {
    const list = spawnSync('tasklist', ['/FI', 'IMAGENAME eq cargo.exe'], { encoding: 'utf8' }).stdout ?? ''
    if (!/cargo\.exe/i.test(list)) return
    console.log('cargo is running elsewhere, waiting')
    await new Promise((r) => setTimeout(r, 15_000))
  }
}

async function build() {
  sh(process.execPath, [join(pwa, 'scripts/ensure-wasm.mjs')])
  sh(process.execPath, [join(root, 'node_modules/vite/bin/vite.js'), 'build'], { cwd: pwa })
  if (flags['no-peer-build'] && existsSync(peerExe)) return
  await waitForCargo()
  sh('cargo', ['build', '-j', '4', '--manifest-path', join(here, 'peer/Cargo.toml')], { env: { ...process.env, CARGO_TARGET_DIR: join(root, 'apps/core/target') } })
}

async function launch(name) {
  if (name === 'webkit') return webkit.launch()
  const args = ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream', `--use-file-for-fake-video-capture=${videoPath}`]
  try {
    return await chromium.launch({ channel: 'chrome', args })
  } catch {
    return chromium.launch({ args })
  }
}

async function main() {
  mkdirSync(qaDir, { recursive: true })
  mkdirSync(qaDirB, { recursive: true })
  writeBlankVideo(videoPath)
  if (!flags['no-build']) await build()
  const persist = mkdtempSync(join(tmpdir(), 'orrery-pwa-relay-'))
  const relay = startRelay({ relayDir: join(root, 'apps/relay'), port: relayPort, inspectorPort, persist })
  const stat = await startStatic({ dist: join(pwa, 'dist'), relayWs: `ws://127.0.0.1:${relayPort}` })
  try {
    await waitHealth(relayHttp, 90_000).catch((e) => {
      throw new Error(`${e.message}\n${relay.log.join('')}`)
    })
    const env = { base: `http://127.0.0.1:${stat.port}`, relayHttp, peerExe, qaDir, qaDirB, videoPath, webHash: stat.state.web }
    for (const browserName of browsers) {
      try {
        await runScenario({ browserName, launch, env, record: (r) => results.push(r) })
      } catch (error) {
        console.log(`${browserName}: stopped: ${String(error?.message ?? error).split('\n')[0]}`)
      }
    }
  } finally {
    if (flags.relaylog) console.log(relay.log.join('').split(/\r?\n/).slice(-40).join('\n'))
    stat.server.close()
    cleanup()
    killTree(relay.child)
    try {
      rmSync(persist, { recursive: true, force: true })
    } catch {}
  }
  mkdirSync(join(pwa, 'e2e/.out'), { recursive: true })
  writeFileSync(join(pwa, 'e2e/.out/results.json'), JSON.stringify({ at: new Date().toISOString(), results }, null, 2))
  for (const r of results) console.log(`${r.ok ? 'PASS' : 'FAIL'} [${r.browser}] ${r.name}${r.ok ? ` (${r.ms} ms)` : ` -> ${r.error}`}`)
  const pass = results.filter((r) => r.ok).length
  const fail = results.length - pass
  const expected = browsers.length * STEPS_PER_BROWSER
  console.log(`e2e: ${pass} pass, ${fail} fail, ${results.length}/${expected} steps ran`)
  process.exitCode = fail === 0 && results.length === expected ? 0 : 1
}

process.on('SIGINT', () => {
  cleanup()
  process.exit(130)
})
main().catch((error) => {
  cleanup()
  console.error(error)
  process.exit(1)
})
