import { execFile } from 'node:child_process'
import fs from 'node:fs/promises'
import path from 'node:path'
import type { CleanItem } from '@ccm/shared'
import { realDir } from './caches'
import type { CleanCtx, CleanProvider } from './providers'

export interface ProcInfo {
  name: string
  cmd: string
}

export interface SdkSpots {
  sdkRoot: string | null
  avdHome: string
  gradleHome: string
  xcodeBase: string
}

export interface SdkDeps extends SdkSpots {
  procs: () => Promise<ProcInfo[]>
  simctl?: (args: string[]) => Promise<string | null>
}

export interface SdkSpotsInput {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  fake?: string | null
}

export const ARCHIVE_KEEP_MS = 90 * 86_400_000
const READ_MAX = 512 * 1024

export function sdkSpots({ platform, home, env, fake }: SdkSpotsInput): SdkSpots {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const j = (...p: string[]): string => lib.join(...p)
  if (fake) return { sdkRoot: j(fake, 'sdk'), avdHome: j(fake, 'avd'), gradleHome: j(fake, 'gradle'), xcodeBase: j(fake, 'Xcode') }
  const local = env.LOCALAPPDATA || j(home, 'AppData', 'Local')
  const sdkRoot = env.ANDROID_HOME || env.ANDROID_SDK_ROOT || (platform === 'win32' ? j(local, 'Android', 'Sdk') : platform === 'darwin' ? j(home, 'Library', 'Android', 'sdk') : j(home, 'Android', 'Sdk'))
  const userHome = env.ANDROID_USER_HOME || (env.ANDROID_SDK_HOME ? j(env.ANDROID_SDK_HOME, '.android') : j(home, '.android'))
  const avdHome = env.ANDROID_AVD_HOME || j(userHome, 'avd')
  return { sdkRoot, avdHome, gradleHome: env.GRADLE_USER_HOME || j(home, '.gradle'), xcodeBase: j(home, 'Library', 'Developer', 'Xcode') }
}

export function parseProcLines(text: string): ProcInfo[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => {
      const tab = l.indexOf('\t')
      return tab < 0 ? { name: l.toLowerCase(), cmd: '' } : { name: l.slice(0, tab).toLowerCase(), cmd: l.slice(tab + 1) }
    })
}

export interface AndroidUse {
  compile: Set<number>
  tools: Set<string>
  gradle: Set<string>
  android: boolean
  compileUnsure: boolean
  toolsUnsure: boolean
  gradleUnsure: boolean
}

const BUILD_FILES = [
  ['build.gradle'],
  ['build.gradle.kts'],
  ['app', 'build.gradle'],
  ['app', 'build.gradle.kts'],
  ['android', 'build.gradle'],
  ['android', 'build.gradle.kts'],
  ['android', 'app', 'build.gradle'],
  ['android', 'app', 'build.gradle.kts'],
]
const WRAPPER_FILES = [
  ['gradle', 'wrapper', 'gradle-wrapper.properties'],
  ['android', 'gradle', 'wrapper', 'gradle-wrapper.properties'],
]

async function readSmall(p: string): Promise<string | null> {
  const st = await fs.lstat(p).catch(() => null)
  if (!st?.isFile() || st.size > READ_MAX) return null
  return fs.readFile(p, 'utf8').catch(() => null)
}

export function readBuildFile(text: string, use: AndroidUse): void {
  const isApp = /com\.android\.(application|library)|android\s*\{/.test(text)
  let sawCompile = false
  for (const m of text.matchAll(/compileSdk(?:Version)?\s*(?:=|\(|\s)\s*([^\s)\n]+)/g)) {
    sawCompile = true
    const raw = m[1]!.replace(/["')]/g, '').replace(/^android-/, '')
    if (/^\d+$/.test(raw)) use.compile.add(Number(raw))
    else use.compileUnsure = true
  }
  if (/compileSdk\s*\{/.test(text)) use.compileUnsure = true
  let sawTools = false
  for (const m of text.matchAll(/buildToolsVersion\s*(?:=|\(|\s)\s*["']([\d.]+(?:[-\w]*)?)["']/g)) {
    sawTools = true
    use.tools.add(m[1]!)
  }
  if (isApp) {
    use.android = true
    if (!sawCompile && /android\s*\{/.test(text)) use.compileUnsure = true
    if (!sawTools && /android\s*\{/.test(text)) use.toolsUnsure = true
  }
}

export function readWrapper(text: string): string | null {
  const m = /distributionUrl\s*=\s*\S*?(gradle-[\w.\-]+?-(?:bin|all))\.zip/.exec(text)
  return m ? m[1]! : null
}

export async function androidUse(roots: readonly string[]): Promise<AndroidUse> {
  const use: AndroidUse = { compile: new Set(), tools: new Set(), gradle: new Set(), android: false, compileUnsure: false, toolsUnsure: false, gradleUnsure: false }
  let wrappers = 0
  for (const root of roots) {
    for (const parts of BUILD_FILES) {
      const text = await readSmall(path.join(root, ...parts))
      if (text) readBuildFile(text, use)
    }
    for (const parts of WRAPPER_FILES) {
      const text = await readSmall(path.join(root, ...parts))
      if (!text) continue
      wrappers++
      const d = readWrapper(text)
      if (d) use.gradle.add(d)
      else use.gradleUnsure = true
    }
  }
  if (!use.android) {
    use.compileUnsure = true
    use.toolsUnsure = true
  }
  if (!wrappers) use.gradleUnsure = true
  return use
}

export function versionCmp(a: string, b: string): number {
  const pa = a.split(/[.\-]/)
  const pb = b.split(/[.\-]/)
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? ''
    const y = pb[i] ?? ''
    const nx = /^\d+$/.test(x) ? Number(x) : NaN
    const ny = /^\d+$/.test(y) ? Number(y) : NaN
    if (!Number.isNaN(nx) && !Number.isNaN(ny)) {
      if (nx !== ny) return nx - ny
      continue
    }
    if (x === y) continue
    if (!x) return Number.isNaN(ny) ? 1 : -1
    if (!y) return Number.isNaN(nx) ? -1 : 1
    if (!Number.isNaN(nx)) return 1
    if (!Number.isNaN(ny)) return -1
    return x < y ? -1 : 1
  }
  return 0
}

export async function subdirs(dir: string): Promise<Array<{ name: string; path: string; mtime: number }>> {
  const names = await fs.readdir(dir).catch(() => [] as string[])
  const out: Array<{ name: string; path: string; mtime: number }> = []
  for (const n of names) {
    const p = path.join(dir, n)
    const st = await fs.lstat(p).catch(() => null)
    if (st && st.isDirectory() && !st.isSymbolicLink()) out.push({ name: n, path: p, mtime: st.mtimeMs })
  }
  return out
}

function sdkOf(ctx: CleanCtx): SdkDeps | null {
  return ctx.sdk ?? null
}

async function push(ctx: CleanCtx, out: CleanItem[], base: Omit<CleanItem, 'size' | 'group'>): Promise<void> {
  if (ctx.signal.aborted) return
  const size = base.file ? ((await fs.lstat(base.path).catch(() => null))?.size ?? 0) : await ctx.size(base.path)
  const item: CleanItem = { ...base, size, group: 'dev' }
  out.push(item)
  ctx.emit([item])
}

const imageKey = (s: string): string =>
  s
    .replace(/\\/g, '/')
    .replace(/^.*?system-images\//, '')
    .replace(/\/+$/, '')
    .toLowerCase()

export async function avdImages(avdHome: string): Promise<{ used: Set<string>; readable: boolean }> {
  const used = new Set<string>()
  const list = await fs.readdir(avdHome).catch(() => null)
  if (!list) return { used, readable: false }
  for (const n of list) {
    if (!n.endsWith('.avd')) continue
    const text = await readSmall(path.join(avdHome, n, 'config.ini'))
    if (!text) continue
    for (const m of text.matchAll(/^image\.sysdir\.\d+\s*=\s*(.+)$/gm)) used.add(imageKey(m[1]!.trim()))
  }
  return { used, readable: true }
}

export const sdkProvider: CleanProvider = {
  id: 'sdk',
  group: 'dev',
  preselect: true,
  scan: async (ctx) => {
    const sdk = sdkOf(ctx)
    if (!sdk?.sdkRoot) return []
    const root = await realDir(sdk.sdkRoot)
    if (!root) {
      ctx.note?.('noSdk')
      return []
    }
    const use = await androidUse(ctx.roots)
    const out: CleanItem[] = []
    const platforms = (await subdirs(path.join(root, 'platforms'))).filter((d) => d.name.startsWith('android-'))
    const levels = platforms.map((d) => Number(d.name.slice(8))).filter((n) => Number.isFinite(n))
    const newest = levels.length ? Math.max(...levels) : null
    for (const d of platforms) {
      const level = Number(d.name.slice(8))
      if (Number.isFinite(level) && (level === newest || use.compile.has(level))) continue
      const hold = use.compileUnsure || !Number.isFinite(level)
      await push(ctx, out, { path: d.path, category: 'sdk', kind: 'platform', detail: d.name, lastTouched: d.mtime, restore: `sdkmanager "platforms;${d.name}"`, ...(hold ? { hold: true } : {}) })
    }
    const tools = await subdirs(path.join(root, 'build-tools'))
    const top = tools.map((d) => d.name).sort(versionCmp).pop()
    for (const d of tools) {
      if (d.name === top || use.tools.has(d.name)) continue
      await push(ctx, out, { path: d.path, category: 'sdk', kind: 'buildTools', detail: d.name, lastTouched: d.mtime, restore: `sdkmanager "build-tools;${d.name}"`, ...(use.toolsUnsure ? { hold: true } : {}) })
    }
    const avd = await avdImages(sdk.avdHome)
    for (const api of await subdirs(path.join(root, 'system-images'))) {
      for (const tag of await subdirs(api.path)) {
        for (const abi of await subdirs(tag.path)) {
          const key = `${api.name}/${tag.name}/${abi.name}`.toLowerCase()
          if (avd.used.has(key)) continue
          await push(ctx, out, {
            path: abi.path,
            category: 'sdk',
            kind: 'sysImage',
            detail: `${api.name};${tag.name};${abi.name}`,
            lastTouched: abi.mtime,
            restore: `sdkmanager "system-images;${api.name};${tag.name};${abi.name}"`,
            ...(avd.readable ? {} : { hold: true }),
          })
        }
      }
    }
    return out
  },
}

export function avdRunning(name: string, procs: readonly ProcInfo[]): boolean {
  const n = name.toLowerCase()
  return procs.some((p) => {
    if (!/^(emulator|qemu-system)/.test(p.name.replace(/\.exe$/, ''))) return false
    const c = ` ${p.cmd.toLowerCase()} `
    return c.includes(` @${n} `) || c.includes(` -avd ${n} `) || c.includes(`"@${n}"`)
  })
}

async function newestInside(dir: string): Promise<number> {
  let best = 0
  for (const n of await fs.readdir(dir).catch(() => [] as string[])) {
    const st = await fs.lstat(path.join(dir, n)).catch(() => null)
    if (st && st.mtimeMs > best) best = st.mtimeMs
  }
  return best
}

async function hasLock(dir: string): Promise<boolean> {
  return (await fs.readdir(dir).catch(() => [] as string[])).some((n) => n.endsWith('.lock'))
}

export async function avdBusy(item: Pick<CleanItem, 'path' | 'kind' | 'detail'>, sdk: SdkDeps): Promise<boolean> {
  const name = item.kind === 'avdIni' ? path.basename(item.path).replace(/\.ini$/i, '') : path.basename(item.path).replace(/\.avd$/i, '')
  const dir = item.kind === 'avdIni' ? path.join(path.dirname(item.path), `${name}.avd`) : item.path
  return (await hasLock(dir)) || avdRunning(name, await sdk.procs().catch(() => [] as ProcInfo[]))
}

export const avdProvider: CleanProvider = {
  id: 'avd',
  group: 'dev',
  preselect: false,
  scan: async (ctx) => {
    const sdk = sdkOf(ctx)
    if (!sdk) return []
    const home = await realDir(sdk.avdHome)
    if (!home) return []
    const procs = await sdk.procs().catch(() => [] as ProcInfo[])
    const out: CleanItem[] = []
    for (const d of await subdirs(home)) {
      if (!d.name.endsWith('.avd')) continue
      const name = d.name.slice(0, -4)
      const running = (await hasLock(d.path)) || avdRunning(name, procs)
      const last = (await newestInside(d.path)) || d.mtime
      const restore = 'Android Studio > Device Manager'
      const lock = running ? { locked: 'app' as const } : {}
      await push(ctx, out, { path: d.path, category: 'avd', kind: 'avd', detail: name, lastTouched: last, restore, hold: true, ...lock })
      const ini = path.join(home, `${name}.ini`)
      const st = await fs.lstat(ini).catch(() => null)
      if (st?.isFile()) await push(ctx, out, { path: ini, category: 'avd', kind: 'avdIni', detail: `${name}.ini`, lastTouched: last, file: true, hold: true, ...lock })
    }
    return out
  },
}

export const gradleDistProvider: CleanProvider = {
  id: 'gradle_dists',
  group: 'dev',
  preselect: true,
  scan: async (ctx) => {
    const sdk = sdkOf(ctx)
    if (!sdk) return []
    const home = await realDir(sdk.gradleHome)
    if (!home) return []
    const dists = (await subdirs(path.join(home, 'wrapper', 'dists'))).filter((d) => /^gradle-.+-(bin|all)$/.test(d.name))
    if (!dists.length) return []
    const use = await androidUse(ctx.roots)
    const ver = (n: string): string => n.replace(/^gradle-/, '').replace(/-(bin|all)$/, '')
    const top = dists.map((d) => ver(d.name)).sort(versionCmp).pop()
    const out: CleanItem[] = []
    for (const d of dists) {
      if (ver(d.name) === top || use.gradle.has(d.name)) continue
      await push(ctx, out, { path: d.path, category: 'gradle_dists', kind: 'gradleDist', detail: d.name, lastTouched: d.mtime, restore: './gradlew --version', ...(use.gradleUnsure ? { hold: true } : {}) })
    }
    return out
  },
}

export function parseSimctlUnavailable(json: string | null): string[] {
  if (!json) return []
  try {
    const data = JSON.parse(json) as { devices?: Record<string, Array<{ isAvailable?: boolean; dataPath?: string }>> }
    const out: string[] = []
    for (const list of Object.values(data.devices ?? {})) for (const d of list ?? []) if (d.isAvailable === false && typeof d.dataPath === 'string') out.push(d.dataPath)
    return out
  } catch {
    return []
  }
}

export const iosProvider: CleanProvider = {
  id: 'ios',
  group: 'dev',
  preselect: true,
  platforms: ['darwin'],
  scan: async (ctx) => {
    const sdk = sdkOf(ctx)
    if (!sdk) return []
    const out: CleanItem[] = []
    for (const fam of ['iOS DeviceSupport', 'watchOS DeviceSupport', 'tvOS DeviceSupport']) {
      const list = await subdirs(path.join(sdk.xcodeBase, fam))
      const ver = (n: string): string => (/^[\w ]*?(\d+(?:\.\d+)*)/.exec(n)?.[1] ?? n)
      const top = list.map((d) => d.name).sort((a, b) => versionCmp(ver(a), ver(b))).pop()
      for (const d of list) {
        if (d.name === top) continue
        await push(ctx, out, { path: d.path, category: 'ios', kind: 'deviceSupport', detail: d.name, lastTouched: d.mtime, restore: 'Xcode > Window > Devices and Simulators' })
      }
    }
    if (sdk.simctl && !ctx.signal.aborted) {
      const dirs = parseSimctlUnavailable(await sdk.simctl(['simctl', 'list', 'devices', 'unavailable', '-j']).catch(() => null))
      if (dirs.length) {
        let size = 0
        for (const d of dirs) size += await ctx.size(d).catch(() => 0)
        const item: CleanItem = { path: 'simctl:unavailable', size, group: 'dev', category: 'ios', kind: 'simUnavailable', action: 'command', files: dirs.length, restore: 'xcrun simctl delete unavailable' }
        out.push(item)
        ctx.emit([item])
      }
    }
    return out
  },
}

export const SDK_PROVIDERS: CleanProvider[] = [sdkProvider, avdProvider, gradleDistProvider, iosProvider]

const EXEC_MAX = 4 * 1024 * 1024

export function runFile(bin: string, pre: string[] = [], env?: NodeJS.ProcessEnv): (args: string[], timeoutMs?: number) => Promise<{ ok: boolean; code?: string; stdout: string }> {
  return (args, timeoutMs = 15_000) =>
    new Promise((resolve) => {
      execFile(bin, [...pre, ...args], { timeout: timeoutMs, windowsHide: true, maxBuffer: EXEC_MAX, encoding: 'utf8', ...(env ? { env } : {}) }, (err, stdout) => {
        if (!err) return resolve({ ok: true, stdout: String(stdout) })
        const e = err as NodeJS.ErrnoException & { killed?: boolean; signal?: string | null }
        const code = e.code === 'ENOENT' ? 'ENOENT' : e.killed || e.signal === 'SIGTERM' ? 'ETIMEDOUT' : typeof e.code === 'string' ? e.code : 'EXIT'
        resolve({ ok: false, code, stdout: String(stdout ?? '') })
      })
    })
}

export interface SdkDepsInput extends SdkSpotsInput {
  procs: () => Promise<ProcInfo[]>
}

export function sdkDeps(input: SdkDepsInput): SdkDeps {
  const spots = sdkSpots(input)
  const fakeProcs = input.fake ? path.join(input.fake, 'procs.txt') : null
  const procs = fakeProcs ? async () => parseProcLines(await fs.readFile(fakeProcs, 'utf8').catch(() => '')) : input.procs
  const xcrun = input.platform === 'darwin' && !input.fake ? runFile('xcrun') : null
  return { ...spots, procs, ...(xcrun ? { simctl: async (args: string[]) => ((await xcrun(args, 15_000)).stdout || null) } : {}) }
}
