import { spawnSync } from 'node:child_process'
import { chmodSync, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { kitEmbed, readKitPin } from '../apps/desktop/daemon-kit.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const SHELL = path.join(ROOT, 'apps', 'shell')
const CORE = path.join(ROOT, 'apps', 'core')
const OUT = path.join(SHELL, 'release')
const TAURI_CLI = '@tauri-apps/cli@2.12.4'
const SETUP_MAX_MB = 35

export const TARGETS = {
  win: [{ arch: 'x64', triple: 'x86_64-pc-windows-msvc', bundle: 'nsis' }],
  mac: [
    { arch: 'arm64', triple: 'aarch64-apple-darwin', bundle: 'dmg' },
    { arch: 'x64', triple: 'x86_64-apple-darwin', bundle: 'dmg' },
  ],
}

export function artifactName(version, step) {
  return step.bundle === 'nsis' ? `Orrery-Setup-${version}.exe` : `Orrery-${version}-${step.arch}.dmg`
}

export function sidecarName(name, triple) {
  return `${name}-${triple}${triple.includes('windows') ? '.exe' : ''}`
}

export function pickOs(argv, platform) {
  if (argv.includes('--win')) return 'win'
  if (argv.includes('--mac')) return 'mac'
  return platform === 'win32' ? 'win' : platform === 'darwin' ? 'mac' : null
}

export function jobs(env, cpus) {
  const asked = Number(env.JIN_MAX_THREADS)
  const cap = Number.isInteger(asked) && asked > 0 ? asked : 6
  return String(Math.max(1, Math.min(cap, 6, cpus)))
}

function run(cmd, args, opts = {}) {
  console.log(`dist-tauri: ${cmd} ${args.join(' ')}`)
  const r = spawnSync(cmd, args, { stdio: 'inherit', shell: process.platform === 'win32', ...opts })
  if (r.status !== 0) {
    console.error(`dist-tauri: ${cmd} failed (${r.status ?? r.signal})`)
    process.exit(1)
  }
}

export function daemonKit(pin = readKitPin()) {
  const kit = kitEmbed(pin)
  if (!kit) {
    console.error('dist-tauri: apps/layout-daemon/kit.json is not pinned yet (placeholder sha256); run "node scripts/kit-release.mjs" once and publish the kit first')
    process.exit(1)
  }
  return kit
}

function stageSidecars(triple) {
  const dir = path.join(SHELL, 'binaries')
  mkdirSync(dir, { recursive: true })
  const ext = triple.includes('windows') ? '.exe' : ''
  for (const name of ['orrery-core', 'orrery-bridge']) {
    const from = path.join(CORE, 'target', triple, 'release', `${name}${ext}`)
    const to = path.join(dir, sidecarName(name, triple))
    copyFileSync(from, to)
    chmodSync(to, 0o755)
  }
}

function findBundle(triple, bundle) {
  const dir = path.join(SHELL, 'target', triple, 'release', 'bundle', bundle)
  const ext = bundle === 'nsis' ? '.exe' : '.dmg'
  const hit = existsSync(dir) ? readdirSync(dir).find((f) => f.endsWith(ext)) : undefined
  return hit ? path.join(dir, hit) : null
}

function main() {
  const which = pickOs(process.argv.slice(2), process.platform)
  if (!which) {
    console.error(`dist-tauri: unsupported platform "${process.platform}" (Windows or macOS only)`)
    process.exit(1)
  }
  const version = JSON.parse(readFileSync(path.join(ROOT, 'apps', 'desktop', 'package.json'), 'utf8')).version
  const j = jobs(process.env, os.cpus().length)
  run('npm', ['run', 'build', '-w', '@ccm/web'], { cwd: ROOT })
  run('node', [path.join(ROOT, 'scripts', 'gen-desk-shim.mjs')])
  daemonKit()
  rmSync(OUT, { recursive: true, force: true })
  mkdirSync(OUT, { recursive: true })
  for (const step of TARGETS[which]) {
    run('rustup', ['target', 'add', step.triple])
    run('cargo', ['build', '--release', '--locked', '-p', 'core-bin', '-p', 'core-bridge', '-j', j, '--target', step.triple], { cwd: CORE })
    stageSidecars(step.triple)
    const env = { ...process.env, CARGO_BUILD_JOBS: j }
    run('npx', ['--yes', TAURI_CLI, 'build', '--config', 'tauri.release.json', '--target', step.triple, '--bundles', step.bundle, '--ci'], { cwd: SHELL, env })
    const built = findBundle(step.triple, step.bundle)
    if (!built) {
      console.error(`dist-tauri: no ${step.bundle} bundle for ${step.triple}`)
      process.exit(1)
    }
    const to = path.join(OUT, artifactName(version, step))
    copyFileSync(built, to)
    const mb = statSync(to).size / 1024 / 1024
    console.log(`dist-tauri: ${path.relative(ROOT, to)} ${mb.toFixed(1)} MB`)
    if (step.bundle === 'nsis' && mb > SETUP_MAX_MB) console.warn(`dist-tauri: WARNING Setup is ${mb.toFixed(1)} MB, over ${SETUP_MAX_MB} MB`)
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
