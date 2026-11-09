// Bundles the Electron main process and the collector into stage/, the app dir electron-builder packs.
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chooseKitSource, describeZip, fetchReleaseZip, packDaemonKit, releaseZipUrl, sha256Of } from './daemon-kit.mjs'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const STAGE = path.join(HERE, 'stage')
const pkg = JSON.parse(readFileSync(path.join(HERE, 'package.json'), 'utf8'))

const daemonDir = path.join(HERE, '..', 'layout-daemon')
const jar = path.join(daemonDir, 'build', 'libs', 'layout-daemon.jar')
const skipDaemon = process.env.CCM_SKIP_DAEMON_BUILD === '1'

// Java range the daemon's Gradle wrapper (8.14) runs on; raise the max with the wrapper.
const GRADLE_JAVA = { min: 17, max: 24 }

function jdkMajor(home) {
  try {
    const m = /^JAVA_VERSION="(?:1\.)?(\d+)/m.exec(readFileSync(path.join(home, 'release'), 'utf8'))
    return m ? Number(m[1]) : null
  } catch {
    return null
  }
}

/** JAVA_HOME when Gradle runs on it, else the newest fitting JDK of Gradle's own toolchains (~/.gradle/jdks) or the Android Studio JBR (paths as `studioJbr` in src/run/android/env.ts). */
function gradleJavaHome(env) {
  const fits = (home) => {
    const v = home ? jdkMajor(home) : null
    return v !== null && v >= GRADLE_JAVA.min && v <= GRADLE_JAVA.max ? v : null
  }
  if (fits(env.JAVA_HOME?.trim())) return env.JAVA_HOME
  const toolchains = path.join(env.GRADLE_USER_HOME?.trim() || path.join(os.homedir(), '.gradle'), 'jdks')
  const homes = existsSync(toolchains) ? readdirSync(toolchains).map((d) => path.join(toolchains, d)) : []
  if (process.platform === 'win32') homes.push(path.win32.join(env.ProgramFiles || 'C:\\Program Files', 'Android', 'Android Studio', 'jbr'))
  if (process.platform === 'darwin') homes.push('/Applications/Android Studio.app/Contents/jbr/Contents/Home')
  const best = homes.map((home) => ({ home, v: fits(home) })).filter((j) => j.v !== null).sort((a, b) => b.v - a.v)[0]
  return best?.home ?? env.JAVA_HOME
}

/** Gradle in apps/layout-daemon with a JAVA_HOME it runs on. */
function buildDaemon(tasks) {
  const env = { ...process.env }
  const javaHome = gradleJavaHome(env)
  if (javaHome) env.JAVA_HOME = javaHome
  const gradlew = path.join(daemonDir, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew')
  return spawnSync(gradlew, [...tasks, '--console=plain'], { cwd: daemonDir, env, stdio: 'inherit', shell: process.platform === 'win32' }).status === 0
}

const zipOnly = process.argv.includes('--daemon-zip-only')
const dist = process.argv.includes('--dist') || zipOnly
const zipFile = path.join(daemonDir, 'build', 'dist', `layout-daemon-${pkg.version}.zip`)
let kit = null
if (dist) {
  const ci = process.env.CI === 'true'
  const zipExists = existsSync(zipFile)
  const reuseFlag = process.env.CCM_REUSE_DAEMON_ZIP === '1'
  const release = !ci && !(zipExists && reuseFlag) ? await fetchReleaseZip(pkg.version) : null
  const source = chooseKitSource({ ci, reuseFlag, zipExists, release })
  if (source === 'release') {
    mkdirSync(path.dirname(zipFile), { recursive: true })
    writeFileSync(zipFile, release)
    kit = { version: pkg.version, size: release.length, sha256: sha256Of(release) }
    console.log(`build: using the layout daemon zip already published at ${releaseZipUrl(pkg.version)}`)
  } else if (source === 'local') {
    if (!skipDaemon && !buildDaemon(['shadowJar', 'stageLooseKit'])) {
      console.error('build: layout-daemon jar / loose kit build failed (needs a JDK 17–24 in JAVA_HOME or ~/.gradle/jdks; Gradle 8.14 does not run on Java 25)')
      process.exit(1)
    }
    if (!existsSync(jar)) {
      console.error(`build: ${jar} is missing; run "./gradlew shadowJar" in apps/layout-daemon with JAVA_HOME = JDK 17+`)
      process.exit(1)
    }
    if (!existsSync(path.join(daemonDir, 'build', 'loose-kit', 'kit.json'))) {
      console.error(`build: the loose layout kit is missing; run "./gradlew stageLooseKit" in apps/layout-daemon`)
      process.exit(1)
    }
    kit = { version: pkg.version, ...packDaemonKit({ jar, kitDir: path.join(daemonDir, 'build', 'loose-kit'), out: zipFile }) }
    if (!ci) console.warn(`build: WARNING no layout daemon zip on release v${pkg.version}; the installer pins this local zip, upload exactly ${zipFile} (sha256 ${kit.sha256}) to release v${pkg.version} or XML Preview will fail with daemon-sha256-mismatch`)
  } else {
    kit = { version: pkg.version, ...describeZip(zipFile) }
  }
  console.log(`build: layout-daemon-${pkg.version}.zip ${kit.size} bytes sha256 ${kit.sha256}`)
  if (zipOnly) process.exit(0)
} else if (!skipDaemon) {
  // Dev runs the jar, loose kit and layoutlib-dist straight from build/ (main.ts); rebuilt only when one is gone.
  const built = [jar, path.join(daemonDir, 'build', 'loose-kit', 'kit.json'), path.join(daemonDir, 'build', 'layoutlib-dist')].every((p) => existsSync(p))
  if (!built && !buildDaemon(['assemble'])) {
    console.warn('build: layout-daemon build failed, the XML preview stays off (needs a JDK 17–24 in JAVA_HOME or ~/.gradle/jdks; Gradle 8.14 does not run on Java 25)')
  }
}

rmSync(STAGE, { recursive: true, force: true })
mkdirSync(STAGE, { recursive: true })
const { build } = await import('esbuild')

// win-procs.ts reads import.meta only when `require` is missing (ESM under tsx / vitest), never in these CJS bundles.
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
  external: ['electron', 'koffi'],
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
  // koffi is native: shipped as extraResources (koffi/node_modules), required by path CCM_KOFFI.
  external: ['bufferutil', 'utf-8-validate', 'koffi'],
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
  external: ['@lydell/node-pty', 'bufferutil', 'utf-8-validate', 'koffi'],
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
