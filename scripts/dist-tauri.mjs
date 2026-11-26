import { spawnSync } from 'node:child_process'
import { chmodSync, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { kitEmbed, readKitPin } from './lib/daemon-kit.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const SHELL = path.join(ROOT, 'apps', 'shell')
const CORE = path.join(ROOT, 'apps', 'core')
const OUT = path.join(SHELL, 'release')
const TAURI_CLI = '@tauri-apps/cli@2.12.4'
const SETUP_MAX_MB = 70
const MOBILE = path.join(ROOT, 'apps', 'mobile')
const APK_DIR = path.join(SHELL, 'resources', 'apk')

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

export function pubspecVersionName(text) {
  const m = /^version:\s*([0-9]+(?:\.[0-9]+)*)/m.exec(text)
  return m ? m[1] : null
}

export function normCert(text) {
  return String(text).replace(/[^0-9a-fA-F]/g, '').toLowerCase()
}

export function parseSignerCerts(output) {
  return [...String(output).matchAll(/certificate SHA-256 digest:\s*([0-9a-fA-F:]+)/g)].map((m) => normCert(m[1]))
}

export function certMatches(output, expected) {
  const want = normCert(expected)
  const got = parseSignerCerts(output)
  return want.length === 64 && got.length > 0 && got.every((c) => c === want)
}

function findApksigner(env = process.env) {
  const roots = [env.ANDROID_HOME, env.ANDROID_SDK_ROOT].filter(Boolean)
  const name = process.platform === 'win32' ? 'apksigner.bat' : 'apksigner'
  const hits = []
  for (const root of roots) {
    const tools = path.join(root, 'build-tools')
    if (!existsSync(tools)) continue
    for (const v of readdirSync(tools)) {
      const exe = path.join(tools, v, name)
      if (existsSync(exe)) hits.push({ v, exe })
    }
  }
  hits.sort((a, b) => a.v.localeCompare(b.v, undefined, { numeric: true }))
  return hits.length ? hits[hits.length - 1].exe : null
}

function fail(message) {
  console.error(`dist-tauri: ${message}`)
  process.exit(1)
}

export function stageApk(j) {
  const fromEnv = process.env.ORRERY_APK
  let source = fromEnv ? path.resolve(fromEnv) : null
  if (!source) {
    run('flutter', ['build', 'apk', '--release', '--target-platform', 'android-arm64'], { cwd: MOBILE, env: { ...process.env, GRADLE_OPTS: `${process.env.GRADLE_OPTS ?? ''} -Dorg.gradle.workers.max=${j}`.trim() } })
    source = path.join(MOBILE, 'build', 'app', 'outputs', 'flutter-apk', 'app-release.apk')
  }
  if (!existsSync(source) || statSync(source).size === 0) fail(`APK not found: ${source}`)
  const signer = findApksigner()
  if (!signer) fail('apksigner not found: set ANDROID_HOME or ANDROID_SDK_ROOT with build-tools installed')
  const certFile = path.join(MOBILE, 'android', 'cert.sha256')
  if (!existsSync(certFile)) fail(`missing ${path.relative(ROOT, certFile)}`)
  const r = spawnSync(process.platform === 'win32' ? `"${signer}"` : signer, ['verify', '--print-certs', process.platform === 'win32' ? `"${source}"` : source], { encoding: 'utf8', shell: process.platform === 'win32' })
  if (r.status !== 0) fail(`apksigner verify failed for ${source}`)
  if (!certMatches(r.stdout, readFileSync(certFile, 'utf8'))) fail('APK signing certificate does not match apps/mobile/android/cert.sha256')
  const version = pubspecVersionName(readFileSync(path.join(MOBILE, 'pubspec.yaml'), 'utf8'))
  if (!version) fail('cannot read versionName from apps/mobile/pubspec.yaml')
  mkdirSync(APK_DIR, { recursive: true })
  copyFileSync(source, path.join(APK_DIR, 'orrery-remote.apk'))
  writeFileSync(path.join(APK_DIR, 'version.txt'), `${version}
`)
  console.log(`dist-tauri: staged orrery-remote.apk ${version}`)
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
  const version = JSON.parse(readFileSync(path.join(ROOT, 'apps', 'shell', 'package.json'), 'utf8')).version
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
