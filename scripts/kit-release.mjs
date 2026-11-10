import { spawnSync } from 'node:child_process'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describeZip, KIT_FILE, kitNames, makeKitPin, packDaemonKit, readKitPin, writeKitPin } from '../apps/desktop/daemon-kit.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const DAEMON = path.join(ROOT, 'apps', 'layout-daemon')
const JAR = path.join(DAEMON, 'build', 'libs', 'layout-daemon.jar')
const LOOSE = path.join(DAEMON, 'build', 'loose-kit')
const GRADLE_JAVA = { min: 17, max: 24 }

export function parseArgs(argv, current) {
  const out = { version: String(current), skipGradle: false, zip: null, help: false }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--version') out.version = String(argv[++i] ?? '')
    else if (a === '--skip-gradle') out.skipGradle = true
    else if (a === '--zip') out.zip = argv[++i] ?? null
    else if (a === '--help' || a === '-h') out.help = true
    else throw new Error(`unknown argument ${a}`)
  }
  if (!/^[1-9][0-9]*$/.test(out.version)) throw new Error(`--version must be a positive integer, got "${out.version}"`)
  return out
}

export function releaseCommand(version, zip) {
  const { tag } = kitNames(version)
  return `gh release create ${tag} "${zip}" --repo JLSoft-Together/Orrery --title "Layout daemon kit ${version}" --notes "XML Preview render daemon kit ${version}. Downloaded on demand by Orrery (Settings, Integrations, XML Preview)."`
}

function jdkMajor(home) {
  try {
    const m = /^JAVA_VERSION="(?:1\.)?(\d+)/m.exec(readFileSync(path.join(home, 'release'), 'utf8'))
    return m ? Number(m[1]) : null
  } catch {
    return null
  }
}

function gradleJavaHome(env) {
  const fits = (home) => {
    const v = home ? jdkMajor(home) : null
    return v !== null && v >= GRADLE_JAVA.min && v <= GRADLE_JAVA.max ? v : null
  }
  if (fits(env.JAVA_HOME?.trim())) return env.JAVA_HOME
  const toolchains = path.join(env.GRADLE_USER_HOME?.trim() || path.join(os.homedir(), '.gradle'), 'jdks')
  const homes = existsSync(toolchains) ? readdirSync(toolchains).flatMap((d) => [path.join(toolchains, d), path.join(toolchains, d, 'Contents', 'Home')]) : []
  if (process.platform === 'win32') homes.push(path.win32.join(env.ProgramFiles || 'C:\\Program Files', 'Android', 'Android Studio', 'jbr'))
  if (process.platform === 'darwin') homes.push('/Applications/Android Studio.app/Contents/jbr/Contents/Home')
  const best = homes.map((home) => ({ home, v: fits(home) })).filter((j) => j.v !== null).sort((a, b) => b.v - a.v)[0]
  return best?.home ?? env.JAVA_HOME
}

function buildDaemon() {
  const env = { ...process.env }
  const javaHome = gradleJavaHome(env)
  if (javaHome) env.JAVA_HOME = javaHome
  const gradlew = path.join(DAEMON, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew')
  return spawnSync(gradlew, ['shadowJar', 'stageLooseKit', '--console=plain'], { cwd: DAEMON, env, stdio: 'inherit', shell: process.platform === 'win32' }).status === 0
}

function main() {
  const current = readKitPin(KIT_FILE).version
  let args
  try {
    args = parseArgs(process.argv.slice(2), current)
  } catch (err) {
    console.error(`kit-release: ${err.message}`)
    process.exit(1)
  }
  if (args.help) {
    console.log('node scripts/kit-release.mjs [--version N] [--skip-gradle] [--zip file]\n  builds the daemon kit zip (Gradle, JDK 17-24), writes apps/layout-daemon/kit.json, prints the gh command that publishes it')
    return
  }
  const { asset } = kitNames(args.version)
  const zip = args.zip ? path.resolve(args.zip) : path.join(DAEMON, 'build', 'dist', asset)
  let info
  if (args.zip) {
    info = describeZip(zip)
  } else {
    if (!args.skipGradle && !buildDaemon()) {
      console.error('kit-release: Gradle build failed (needs a JDK 17-24 in JAVA_HOME or ~/.gradle/jdks; Gradle 8.14 does not run on Java 25)')
      process.exit(1)
    }
    if (!existsSync(JAR) || !existsSync(path.join(LOOSE, 'kit.json'))) {
      console.error('kit-release: build/libs/layout-daemon.jar or build/loose-kit is missing; run without --skip-gradle')
      process.exit(1)
    }
    info = packDaemonKit({ jar: JAR, kitDir: LOOSE, out: zip })
  }
  writeKitPin(makeKitPin(args.version, info.size, info.sha256))
  console.log(`kit-release: ${path.relative(ROOT, KIT_FILE)} -> version ${args.version}, ${info.size} bytes, sha256 ${info.sha256}`)
  console.log('kit-release: publish the zip once, then commit kit.json:')
  console.log(releaseCommand(args.version, zip))
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
