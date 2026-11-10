// Bundles the Electron main process and the collector into stage/, the app dir electron-builder packs.
import { spawnSync } from 'node:child_process'
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { kitEmbed, readKitPin } from './daemon-kit.mjs'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const STAGE = path.join(HERE, 'stage')
const pkg = JSON.parse(readFileSync(path.join(HERE, 'package.json'), 'utf8'))

const coreDir = path.join(HERE, '..', 'core')
const coreExe = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const coreRelease = path.join(coreDir, 'target', 'release', coreExe)
const bridgeExe = process.platform === 'win32' ? 'orrery-bridge.exe' : 'orrery-bridge'
const bridgeRelease = path.join(coreDir, 'target', 'release', bridgeExe)
const coreBundle = (arch) => path.join(coreDir, 'target', 'bundle', arch, coreExe)
const skipCore = process.env.CCM_SKIP_CORE_BUILD === '1'
const MAC_CORE_TARGETS = { arm64: 'aarch64-apple-darwin', x64: 'x86_64-apple-darwin' }

function toolPath(name) {
  const exe = process.platform === 'win32' ? `${name}.exe` : name
  if (spawnSync(exe, ['--version'], { stdio: 'ignore' }).status === 0) return exe
  const home = process.env.CARGO_HOME?.trim() || path.join(os.homedir(), '.cargo')
  const local = path.join(home, 'bin', exe)
  return existsSync(local) && spawnSync(local, ['--version'], { stdio: 'ignore' }).status === 0 ? local : null
}

function coreJobs() {
  const asked = Number(process.env.JIN_MAX_THREADS)
  const cap = Number.isInteger(asked) && asked > 0 ? asked : 6
  return String(Math.max(1, Math.min(cap, 6, os.cpus().length)))
}

function cargoBuildCore(cargo, triple) {
  const args = ['build', '--release', '--locked', '-p', 'core-bin', '-p', 'core-bridge', '-j', coreJobs(), ...(triple ? ['--target', triple] : [])]
  return spawnSync(cargo, args, { cwd: coreDir, stdio: 'inherit' }).status === 0
}

function ensureRustTarget(triple) {
  const rustup = toolPath('rustup')
  if (!rustup) return false
  const listed = spawnSync(rustup, ['target', 'list', '--installed'], { encoding: 'utf8' })
  if (listed.status === 0 && listed.stdout.split(/\r?\n/).includes(triple)) return true
  return spawnSync(rustup, ['target', 'add', triple], { cwd: coreDir, stdio: 'inherit' }).status === 0
}

function stageCore(arch, from) {
  const to = coreBundle(arch)
  mkdirSync(path.dirname(to), { recursive: true })
  copyFileSync(from, to)
  chmodSync(to, 0o755)
  const bridgeFrom = path.join(path.dirname(from), bridgeExe)
  const bridgeTo = path.join(path.dirname(to), bridgeExe)
  if (existsSync(bridgeFrom)) {
    copyFileSync(bridgeFrom, bridgeTo)
    chmodSync(bridgeTo, 0o755)
  }
}

function buildCore(forDist) {
  const fail = (msg) => {
    if (forDist) {
      console.error(`build: ${msg}`)
      process.exit(1)
    }
    console.warn(`build: WARNING ${msg}; the Rust PTY core stays unavailable (CCM_CORE_PTY=rust will not start)`)
  }
  const plan = forDist
    ? process.platform === 'win32'
      ? [{ arch: 'x64', triple: null }]
      : process.platform === 'darwin'
        ? Object.entries(MAC_CORE_TARGETS).map(([arch, triple]) => ({ arch, triple }))
        : []
    : [{ arch: null, triple: null }]
  const built = (step) => (step.triple ? path.join(coreDir, 'target', step.triple, 'release', coreExe) : coreRelease)
  if (skipCore) {
    if (forDist && plan.some((s) => !existsSync(coreBundle(s.arch)) && !existsSync(built(s)))) {
      return fail('CCM_SKIP_CORE_BUILD=1 but orrery-core is not built; unset it or build apps/core first')
    }
    if (forDist) for (const s of plan) if (!existsSync(coreBundle(s.arch))) stageCore(s.arch, built(s))
    return
  }
  if (!forDist && existsSync(coreRelease) && existsSync(bridgeRelease)) return
  const cargo = toolPath('cargo')
  if (!cargo) return fail('cargo is not installed (https://rustup.rs); apps/core was not built')
  for (const step of plan) {
    if (step.triple && !ensureRustTarget(step.triple)) return fail(`rust target ${step.triple} is missing (rustup target add ${step.triple})`)
    if (!cargoBuildCore(cargo, step.triple) || !existsSync(built(step))) return fail('apps/core build failed (needs a Rust toolchain and, on Windows, the MSVC build tools)')
    if (forDist) stageCore(step.arch, built(step))
  }
}

const dist = process.argv.includes('--dist')
const kit = kitEmbed(readKitPin())
if (dist && !kit) {
  console.error('build: apps/layout-daemon/kit.json is not pinned yet (placeholder sha256); run "node scripts/kit-release.mjs" once and publish the kit before a dist build')
  process.exit(1)
}

buildCore(dist)

rmSync(STAGE, { recursive: true, force: true })
mkdirSync(STAGE, { recursive: true })
const { build } = await import('esbuild')

const common = { bundle: true, platform: 'node', format: 'cjs', target: 'node24', sourcemap: false, logLevel: 'info', logOverride: { 'empty-import-meta': 'silent' } }

// GramJS (large Telegram files) is its own bundle, required by main only when an upload starts.
const gramjsApart = {
  name: 'gramjs-apart',
  setup(b) {
    b.onResolve({ filter: /^\.\/gramjs$/ }, () => ({ path: './gramjs.cjs', external: true }))
  },
}

await build({
  ...common,
  entryPoints: [path.join(HERE, 'src', 'main.ts')],
  outfile: path.join(STAGE, 'main.cjs'),
  external: ['electron'],
  define: { __DAEMON_KIT__: kit ? JSON.stringify(kit) : 'undefined' },
  plugins: [gramjsApart],
  // Turns the import() of gramjs.cjs into require(), which loads from inside app.asar.
  supported: { 'dynamic-import': false },
})

await build({
  ...common,
  entryPoints: [path.join(HERE, 'src', 'telegram', 'gramjs.ts')],
  outfile: path.join(STAGE, 'gramjs.cjs'),
})

await build({
  ...common,
  entryPoints: [path.join(HERE, '..', 'collector', 'src', 'index.ts')],
  outfile: path.join(STAGE, 'collector.cjs'),
  // ws loads these optional native peers inside try/catch.
  external: ['bufferutil', 'utf-8-validate'],
  // config.ts touches import.meta only when CCM_WEB_DIST / CCM_STATUSLINE_BRIDGE are unset; the desktop host always sets both.
  logOverride: { 'empty-import-meta': 'silent' },
})

await build({
  ...common,
  entryPoints: [path.join(HERE, 'src', 'preload.ts')],
  outfile: path.join(STAGE, 'preload.cjs'),
  external: ['electron'],
})

await build({
  ...common,
  entryPoints: [path.join(HERE, 'src', 'pty', 'host.ts')],
  outfile: path.join(STAGE, 'pty-host.cjs'),
  // node-pty is native: shipped as extraResources (pty/node_modules) and required by path at runtime.
  external: ['@lydell/node-pty', 'bufferutil', 'utf-8-validate'],
})

// No dependencies on purpose: everything is bundled, so electron-builder ships no node_modules.
writeFileSync(
  path.join(STAGE, 'package.json'),
  JSON.stringify(
    {
      name: 'agent-monitor',
      productName: 'Orrery',
      version: pkg.version,
      description: pkg.description,
      author: 'Orrery',
      main: 'main.cjs',
    },
    null,
    2,
  ),
)
