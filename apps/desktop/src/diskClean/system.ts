import { execFile } from 'node:child_process'
import fs from 'node:fs/promises'
import path from 'node:path'
import type { CleanItem, CleanLock } from '@ccm/shared'
import { coreRows } from '../../../collector/src/procSource'
import { Cancelled } from '../agentData/fsTree'
import { isUnder, macTempTree } from '../diskMap/guard'
import type { CleanCtx, CleanProvider } from './providers'

export interface SysRoots {
  platform: NodeJS.Platform
  home: string
  local: string
  temp: string
  windir: string
  programData: string
  downloads: string
  macTmp: string
  bin: string | null
}

export interface BinOps {
  size: () => Promise<{ bytes: number; count: number } | null>
  empty: () => Promise<void>
}

export interface SystemDeps {
  roots: SysRoots
  procs: () => Promise<string[]>
  apps?: () => Promise<string[]>
  bin: BinOps
  busy: (file: string) => Promise<boolean>
}

export interface Member {
  path: string
  size: number
  mtime: number
  file: boolean
}

export const HOUR_MS = 3_600_000
export const DAY_MS = 24 * HOUR_MS
export const TEMP_AGE_MS = DAY_MS
export const LOG_AGE_MS = 7 * DAY_MS
export const INSTALLER_AGE_MS = 30 * DAY_MS
export const MEMBERS_MAX = 20_000
const MEMBER_BATCH = 32

export interface SysRootsInput {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  downloads?: string
  tmpdir?: string
  fake?: string | null
}

export function sysRoots({ platform, home, env, downloads, tmpdir, fake }: SysRootsInput): SysRoots {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const j = (...p: string[]): string => lib.join(...p)
  if (fake) {
    const h = j(fake, 'home')
    return {
      platform,
      home: h,
      local: j(h, 'AppData', 'Local'),
      temp: j(fake, 'Temp'),
      windir: j(fake, 'Windows'),
      programData: j(fake, 'ProgramData'),
      downloads: j(h, 'Downloads'),
      macTmp: j(fake, 'var', 'T'),
      bin: j(fake, 'Bin'),
    }
  }
  const local = env.LOCALAPPDATA || j(home, 'AppData', 'Local')
  return {
    platform,
    home,
    local,
    temp: env.TEMP || env.TMP || j(local, 'Temp'),
    windir: env.SystemRoot || env.windir || 'C:\\Windows',
    programData: env.ProgramData || 'C:\\ProgramData',
    downloads: downloads || j(home, 'Downloads'),
    macTmp: env.TMPDIR || tmpdir || '',
    bin: null,
  }
}

const WIN_FORBIDDEN = ['C:\\Windows', 'C:\\Program Files', 'C:\\Program Files (x86)', 'C:\\ProgramData']
const MAC_FORBIDDEN = ['/System', '/Library', '/usr', '/bin', '/sbin', '/Applications', '/private/etc', '/etc']
const HOME_FORBIDDEN = ['Documents', 'Desktop', 'Pictures', 'Music', 'Videos', 'Movies', '.ssh', '.config', '.gnupg', 'OneDrive', 'iCloudDrive']
const MAC_HOME_FORBIDDEN = [['Library', 'Keychains'], ['Library', 'Mobile Documents'], ['Library', 'CloudStorage'], ['Library', 'Application Support', 'MobileSync']]

export function forbiddenBases(roots: SysRoots, env: NodeJS.ProcessEnv = {}): string[] {
  const lib = roots.platform === 'win32' ? path.win32 : path.posix
  const out: string[] = []
  if (roots.platform === 'win32') {
    out.push(...WIN_FORBIDDEN, roots.windir, roots.programData)
    for (const k of ['ProgramFiles', 'ProgramFiles(x86)', 'ProgramW6432', 'SystemRoot', 'windir', 'ProgramData']) if (env[k]) out.push(env[k]!)
  } else {
    out.push(...MAC_FORBIDDEN)
    for (const parts of MAC_HOME_FORBIDDEN) out.push(lib.join(roots.home, ...parts))
  }
  for (const n of HOME_FORBIDDEN) out.push(lib.join(roots.home, n))
  return out.filter(Boolean)
}

export function isForbidden(p: string, roots: SysRoots, extra: readonly string[] = []): boolean {
  const pf = roots.platform
  if (isUnder(roots.home, p, pf)) return true
  return [...forbiddenBases(roots), ...extra].some((b) => b && isUnder(p, b, pf))
}

const STAT_BATCH = 128

export async function treeInfo(root: string, signal?: AbortSignal, stopAbove = Infinity): Promise<{ bytes: number; newest: number; readable: boolean }> {
  let bytes = 0
  let newest = 0
  let readable = true
  const stack: string[] = [root]
  let first = true
  while (stack.length) {
    if (signal?.aborted) throw new Cancelled()
    const dir = stack.pop()!
    let entries: import('node:fs').Dirent[]
    try {
      entries = await fs.readdir(dir, { withFileTypes: true })
    } catch {
      if (first) readable = false
      first = false
      continue
    }
    first = false
    const paths = entries.filter((e) => !e.isSymbolicLink()).map((e) => path.join(dir, e.name))
    for (let i = 0; i < paths.length; i += STAT_BATCH) {
      const chunk = paths.slice(i, i + STAT_BATCH)
      const stats = await Promise.all(chunk.map((p) => fs.lstat(p).catch(() => null)))
      for (let k = 0; k < chunk.length; k++) {
        const st = stats[k]
        if (!st || st.isSymbolicLink()) continue
        newest = Math.max(newest, st.mtimeMs)
        if (st.isDirectory()) stack.push(chunk[k]!)
        else if (st.isFile()) bytes += st.size
      }
      if (newest > stopAbove) return { bytes, newest, readable }
    }
  }
  return { bytes, newest, readable }
}

async function isDir(p: string): Promise<boolean> {
  const st = await fs.lstat(p).catch(() => null)
  return !!st && st.isDirectory() && !st.isSymbolicLink()
}

export interface MemberRule {
  olderMs: number
  filesOnly?: boolean
  match?: (name: string, file: boolean) => boolean
}

export async function membersOf(ctx: CleanCtx, sys: SystemDeps, dir: string, rule: MemberRule): Promise<{ list: Member[]; readable: boolean }> {
  let entries: import('node:fs').Dirent[]
  try {
    entries = await fs.readdir(dir, { withFileTypes: true })
  } catch {
    return { list: [], readable: false }
  }
  const cutoff = ctx.now - rule.olderMs
  const examine = async (e: import('node:fs').Dirent): Promise<Member | null> => {
    if (e.isSymbolicLink()) return null
    const file = e.isFile()
    if (!file && !e.isDirectory()) return null
    if (rule.filesOnly && !file) return null
    if (rule.match && !rule.match(e.name, file)) return null
    const p = path.join(dir, e.name)
    if (isForbidden(p, sys.roots) || !ctx.keepMember(p)) return null
    const st = await fs.lstat(p).catch(() => null)
    if (!st || st.isSymbolicLink() || st.mtimeMs > cutoff) return null
    if (file) {
      if (await sys.busy(p).catch(() => true)) return null
      return { path: p, size: st.size, mtime: st.mtimeMs, file: true }
    }
    const info = await treeInfo(p, ctx.signal, cutoff)
    if (Math.max(info.newest, st.mtimeMs) > cutoff) return null
    return { path: p, size: info.bytes, mtime: st.mtimeMs, file: false }
  }
  const list: Member[] = []
  for (let i = 0; i < entries.length && list.length < MEMBERS_MAX; i += MEMBER_BATCH) {
    if (ctx.signal.aborted) throw new Cancelled()
    for (const m of await Promise.all(entries.slice(i, i + MEMBER_BATCH).map(examine))) if (m && list.length < MEMBERS_MAX) list.push(m)
  }
  return { list, readable: true }
}

async function filesItem(ctx: CleanCtx, sys: SystemDeps, category: string, kind: string, dir: string, rule: MemberRule): Promise<CleanItem | null> {
  if (!(await isDir(dir))) return null
  if (isForbidden(dir, sys.roots)) return null
  const { list, readable } = await membersOf(ctx, sys, dir, rule)
  if (!readable) {
    const item: CleanItem = { path: dir, size: 0, group: 'system', category, kind, locked: 'denied', unknown: true }
    ctx.emit([item])
    return item
  }
  if (!list.length) return null
  const item: CleanItem = { path: dir, size: list.reduce((s, m) => s + m.size, 0), group: 'system', category, kind, action: 'files', files: list.length }
  ctx.attach(dir, list)
  ctx.emit([item])
  return item
}

async function adminItem(ctx: CleanCtx, category: string, kind: string, dir: string): Promise<CleanItem | null> {
  if (!(await isDir(dir))) return null
  const info = await treeInfo(dir, ctx.signal)
  const item: CleanItem = { path: dir, size: info.bytes, group: 'system', category, kind, locked: 'admin', ...(info.readable ? {} : { unknown: true }) }
  ctx.emit([item])
  return item
}

async function collect(ctx: CleanCtx, jobs: Array<() => Promise<CleanItem | null>>): Promise<CleanItem[]> {
  const out: CleanItem[] = []
  for (const job of jobs) {
    if (ctx.signal.aborted) break
    const it = await job()
    if (it) out.push(it)
  }
  return out
}

function sysOf(ctx: CleanCtx): SystemDeps | null {
  return ctx.system ?? null
}

const j = (roots: SysRoots, ...p: string[]): string => (roots.platform === 'win32' ? path.win32 : path.posix).join(...p)

export const tempProvider: CleanProvider = {
  id: 'temp',
  group: 'system',
  preselect: true,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    if (r.platform === 'win32') {
      return collect(ctx, [() => filesItem(ctx, sys, 'temp', 'userTemp', r.temp, { olderMs: TEMP_AGE_MS }), () => adminItem(ctx, 'temp', 'winTemp', j(r, r.windir, 'Temp'))])
    }
    if (!r.macTmp) return []
    const skipApple = (name: string): boolean => !name.toLowerCase().startsWith('com.apple.')
    return collect(ctx, [
      () => filesItem(ctx, sys, 'temp', 'userTemp', r.macTmp, { olderMs: TEMP_AGE_MS, match: skipApple }),
      () => filesItem(ctx, sys, 'temp', 'userCache', j(r, path.posix.dirname(r.macTmp.replace(/\/+$/, '')), 'C'), { olderMs: TEMP_AGE_MS, match: skipApple }),
    ])
  },
}

export interface BrowserSpot {
  id: string
  name: string
  bases: string[]
  caches: string[]
  procs: string[]
}

export const CHROMIUM_CACHES = ['Cache', 'Code Cache', 'GPUCache']
export const FIREFOX_CACHES = ['cache2']

export function browserSpots(r: SysRoots): BrowserSpot[] {
  if (r.platform === 'win32') {
    return [
      { id: 'chrome', name: 'Chrome', bases: [j(r, r.local, 'Google', 'Chrome', 'User Data')], caches: CHROMIUM_CACHES, procs: ['chrome.exe'] },
      { id: 'edge', name: 'Edge', bases: [j(r, r.local, 'Microsoft', 'Edge', 'User Data')], caches: CHROMIUM_CACHES, procs: ['msedge.exe'] },
      { id: 'brave', name: 'Brave', bases: [j(r, r.local, 'BraveSoftware', 'Brave-Browser', 'User Data')], caches: CHROMIUM_CACHES, procs: ['brave.exe'] },
      { id: 'firefox', name: 'Firefox', bases: [j(r, r.local, 'Mozilla', 'Firefox', 'Profiles')], caches: FIREFOX_CACHES, procs: ['firefox.exe'] },
    ]
  }
  if (r.platform !== 'darwin') return []
  const lc = j(r, r.home, 'Library', 'Caches')
  const as = j(r, r.home, 'Library', 'Application Support')
  return [
    { id: 'chrome', name: 'Chrome', bases: [j(r, lc, 'Google', 'Chrome'), j(r, as, 'Google', 'Chrome')], caches: CHROMIUM_CACHES, procs: ['google chrome'] },
    { id: 'edge', name: 'Edge', bases: [j(r, lc, 'Microsoft Edge'), j(r, as, 'Microsoft Edge')], caches: CHROMIUM_CACHES, procs: ['microsoft edge'] },
    { id: 'brave', name: 'Brave', bases: [j(r, lc, 'BraveSoftware', 'Brave-Browser'), j(r, as, 'BraveSoftware', 'Brave-Browser')], caches: CHROMIUM_CACHES, procs: ['brave browser'] },
    { id: 'firefox', name: 'Firefox', bases: [j(r, lc, 'Firefox', 'Profiles')], caches: FIREFOX_CACHES, procs: ['firefox'] },
  ]
}

export const procName = (p: string): string => path.basename(p.replace(/\\/g, '/')).toLowerCase()

export async function runningBrowsers(sys: SystemDeps): Promise<Set<string>> {
  const names = new Set((await sys.procs().catch(() => [] as string[])).map(procName))
  return new Set(browserSpots(sys.roots).filter((b) => b.procs.some((p) => names.has(p))).map((b) => b.id))
}

export function browserCacheOk(p: string, spot: BrowserSpot, roots: SysRoots): boolean {
  const lib = roots.platform === 'win32' ? path.win32 : path.posix
  if (!spot.caches.includes(lib.basename(p))) return false
  const profile = lib.dirname(p)
  return spot.bases.some((b) => lib.dirname(profile).toLowerCase() === b.toLowerCase())
}

export const browserProvider: CleanProvider = {
  id: 'browser',
  group: 'system',
  preselect: true,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const running = await runningBrowsers(sys)
    const out: CleanItem[] = []
    for (const spot of browserSpots(sys.roots)) {
      for (const base of spot.bases) {
        const profiles = await fs.readdir(base, { withFileTypes: true }).catch(() => [] as import('node:fs').Dirent[])
        for (const prof of profiles) {
          if (ctx.signal.aborted) return out
          if (!prof.isDirectory() || prof.isSymbolicLink()) continue
          for (const c of spot.caches) {
            const p = path.join(base, prof.name, c)
            if (!(await isDir(p)) || !browserCacheOk(p, spot, sys.roots) || isForbidden(p, sys.roots)) continue
            const locked: CleanLock | undefined = running.has(spot.id) ? 'browser' : undefined
            const item: CleanItem = { path: p, size: await ctx.size(p), group: 'system', category: 'browser', kind: spot.id, detail: `${spot.name}, ${prof.name}, ${c}`, ...(locked ? { locked } : {}) }
            out.push(item)
            ctx.emit([item])
          }
        }
      }
    }
    return out
  },
}

export const crashProvider: CleanProvider = {
  id: 'crash',
  group: 'system',
  preselect: true,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    if (r.platform === 'win32') {
      return collect(ctx, [() => filesItem(ctx, sys, 'crash', 'crashDumps', j(r, r.local, 'CrashDumps'), { olderMs: 0 }), () => adminItem(ctx, 'crash', 'minidump', j(r, r.windir, 'Minidump'))])
    }
    return collect(ctx, [() => filesItem(ctx, sys, 'crash', 'diagnostic', j(r, r.home, 'Library', 'Logs', 'DiagnosticReports'), { olderMs: 0 })])
  },
}

export const logsProvider: CleanProvider = {
  id: 'logs',
  group: 'system',
  preselect: true,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    if (r.platform === 'win32') {
      const wer = (base: string, n: string): string => j(r, base, 'Microsoft', 'Windows', 'WER', n)
      return collect(ctx, [
        () => filesItem(ctx, sys, 'logs', 'werArchive', wer(r.local, 'ReportArchive'), { olderMs: 0 }),
        () => filesItem(ctx, sys, 'logs', 'werQueue', wer(r.local, 'ReportQueue'), { olderMs: 0 }),
        () => adminItem(ctx, 'logs', 'werArchiveAll', wer(r.programData, 'ReportArchive')),
        () => adminItem(ctx, 'logs', 'werQueueAll', wer(r.programData, 'ReportQueue')),
      ])
    }
    return collect(ctx, [() => filesItem(ctx, sys, 'logs', 'userLogs', j(r, r.home, 'Library', 'Logs'), { olderMs: LOG_AGE_MS, match: (n) => n !== 'DiagnosticReports' })])
  },
}

export const thumbsProvider: CleanProvider = {
  id: 'thumbs',
  group: 'system',
  preselect: false,
  platforms: ['win32'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    return collect(ctx, [() => filesItem(ctx, sys, 'thumbs', 'thumbcache', j(r, r.local, 'Microsoft', 'Windows', 'Explorer'), { olderMs: 0, filesOnly: true, match: (n) => /^thumbcache_.*\.db$/i.test(n) })])
  },
}

export function installerName(name: string, platform: NodeJS.Platform): boolean {
  const n = name.toLowerCase()
  if (platform === 'darwin') return n.endsWith('.dmg') || n.endsWith('.pkg')
  if (n.endsWith('.exe') || n.endsWith('.msi') || n.endsWith('.dmg') || n.endsWith('.pkg')) return true
  return n.endsWith('.zip') && /(setup|install)/.test(n)
}

export const installersProvider: CleanProvider = {
  id: 'installers',
  group: 'system',
  preselect: false,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    const out: CleanItem[] = []
    const entries = await fs.readdir(r.downloads, { withFileTypes: true }).catch(() => [] as import('node:fs').Dirent[])
    const cutoff = ctx.now - INSTALLER_AGE_MS
    for (const e of entries) {
      if (ctx.signal.aborted) break
      if (!e.isFile() || e.isSymbolicLink() || !installerName(e.name, r.platform)) continue
      const p = path.join(r.downloads, e.name)
      if (isForbidden(p, r) || !ctx.keepMember(p)) continue
      const st = await fs.lstat(p).catch(() => null)
      if (!st || !st.isFile() || st.mtimeMs > cutoff) continue
      const item: CleanItem = { path: p, size: st.size, group: 'system', category: 'installers', kind: 'installer', file: true, lastTouched: st.mtimeMs }
      out.push(item)
      ctx.emit([item])
    }
    return out
  },
}

export const binProvider: CleanProvider = {
  id: 'bin',
  group: 'system',
  preselect: false,
  platforms: ['win32', 'darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    const where = r.bin ?? (r.platform === 'win32' ? `${(r.windir.match(/^[A-Za-z]:/)?.[0] ?? 'C:')}\\$Recycle.Bin` : j(r, r.home, '.Trash'))
    const info = await sys.bin.size().catch(() => null)
    const item: CleanItem = info
      ? { path: where, size: info.bytes, files: info.count, group: 'system', category: 'bin', kind: r.platform === 'darwin' ? 'trash' : 'recycle', action: 'bin' }
      : { path: where, size: 0, group: 'system', category: 'bin', kind: r.platform === 'darwin' ? 'trash' : 'recycle', action: 'bin', locked: 'denied', unknown: true }
    ctx.emit([item])
    return [item]
  },
}

export const updateProvider: CleanProvider = {
  id: 'update',
  group: 'system',
  preselect: false,
  platforms: ['win32'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    return collect(ctx, [
      () => adminItem(ctx, 'update', 'winUpdate', j(r, r.windir, 'SoftwareDistribution', 'Download')),
      () => adminItem(ctx, 'update', 'delivery', j(r, r.windir, 'ServiceProfiles', 'NetworkService', 'AppData', 'Local', 'Microsoft', 'Windows', 'DeliveryOptimization', 'Cache')),
    ])
  },
}

const APP_CACHE_SKIP = new Set(['google', 'microsoft edge', 'bravesoftware', 'firefox', 'yarn', 'pip', 'cocoapods', 'org.swift.swiftpm', 'com.apple.dt.xcode'])

export async function runningApps(sys: SystemDeps): Promise<Set<string>> {
  const out = new Set<string>()
  for (const a of await (sys.apps?.() ?? Promise.resolve([] as string[])).catch(() => [] as string[])) out.add(a.toLowerCase())
  for (const p of await sys.procs().catch(() => [] as string[])) out.add(procName(p))
  return out
}

export function appCacheLock(name: string, running: ReadonlySet<string>): boolean {
  const n = name.toLowerCase()
  if (running.has(n)) return true
  const last = n.split('.').pop() ?? n
  return n.includes('.') && running.has(last)
}

export const appCacheProvider: CleanProvider = {
  id: 'appcache',
  group: 'system',
  preselect: false,
  platforms: ['darwin'],
  scan: async (ctx) => {
    const sys = sysOf(ctx)
    if (!sys) return []
    const r = sys.roots
    const base = j(r, r.home, 'Library', 'Caches')
    const running = await runningApps(sys)
    const out: CleanItem[] = []
    for (const e of await fs.readdir(base, { withFileTypes: true }).catch(() => [] as import('node:fs').Dirent[])) {
      if (ctx.signal.aborted) break
      if (!e.isDirectory() || e.isSymbolicLink()) continue
      const low = e.name.toLowerCase()
      if (low.startsWith('com.apple.') || APP_CACHE_SKIP.has(low)) continue
      const p = path.join(base, e.name)
      if (isForbidden(p, r)) continue
      const item: CleanItem = { path: p, size: await ctx.size(p), group: 'system', category: 'appcache', kind: 'app', detail: e.name, ...(appCacheLock(e.name, running) ? { locked: 'app' as const } : {}) }
      out.push(item)
      ctx.emit([item])
    }
    return out
  },
}

export const SYSTEM_PROVIDERS: CleanProvider[] = [tempProvider, browserProvider, crashProvider, logsProvider, thumbsProvider, installersProvider, binProvider, updateProvider, appCacheProvider]

export async function busyWin(file: string): Promise<boolean> {
  try {
    const h = await fs.open(file, 'r+')
    await h.close()
    return false
  } catch (err) {
    const code = (err as { code?: string }).code
    return code === 'EBUSY' || code === 'EPERM' || code === 'EACCES'
  }
}

function run(file: string, args: string[], timeout = 30_000): Promise<string | null> {
  return new Promise((resolve) => execFile(file, args, { timeout, windowsHide: true, maxBuffer: 16_000_000 }, (err, out) => resolve(err ? null : String(out))))
}

const PS_BIN =
  "Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public static class OrreryBin{[StructLayout(LayoutKind.Sequential)]public struct Q{public int cbSize;public long i64Size;public long i64NumItems;}[DllImport(\"shell32.dll\",CharSet=CharSet.Unicode)]public static extern int SHQueryRecycleBinW(string r,ref Q q);[DllImport(\"shell32.dll\",CharSet=CharSet.Unicode)]public static extern int SHEmptyRecycleBinW(IntPtr h,string r,uint f);}';"

export function parseBinQuery(out: string | null): { bytes: number; count: number } | null {
  const m = /^\s*(-?\d+)\s+(\d+)\s+(\d+)\s*$/m.exec(out ?? '')
  if (!m || Number(m[1]) !== 0) return null
  return { bytes: Number(m[2]), count: Number(m[3]) }
}

export function winBin(windir: string): BinOps {
  const ps = path.join(windir, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
  const exec = (script: string): Promise<string | null> => run(ps, ['-NoProfile', '-NonInteractive', '-Command', PS_BIN + script])
  return {
    size: async () => parseBinQuery(await exec('$q=New-Object OrreryBin+Q;$q.cbSize=[Runtime.InteropServices.Marshal]::SizeOf($q);$r=[OrreryBin]::SHQueryRecycleBinW($null,[ref]$q);"$r $($q.i64Size) $($q.i64NumItems)"')),
    empty: async () => {
      const out = await exec('[OrreryBin]::SHEmptyRecycleBinW([IntPtr]::Zero,$null,7)')
      const code = Number((out ?? '').trim())
      if (out === null || (code !== 0 && code !== -2147418113)) throw Object.assign(new Error('empty failed'), { code: 'EBIN' })
    },
  }
}

export function macBin(home: string): BinOps {
  const trash = path.join(home, '.Trash')
  return {
    size: async () => {
      const names = await fs.readdir(trash).catch(() => null)
      if (!names) return null
      return { bytes: (await treeInfo(trash)).bytes, count: names.filter((n) => n !== '.DS_Store').length }
    },
    empty: async () => {
      const out = await run('osascript', ['-e', 'tell application "Finder" to empty trash'], 300_000)
      if (out === null) throw Object.assign(new Error('empty failed'), { code: 'EBIN' })
    },
  }
}

export function dirBin(dir: string): BinOps {
  return {
    size: async () => {
      const names = await fs.readdir(dir).catch(() => null)
      if (!names) return null
      return { bytes: (await treeInfo(dir)).bytes, count: names.length }
    },
    empty: async () => {
      for (const n of await fs.readdir(dir).catch(() => [] as string[])) await fs.rm(path.join(dir, n), { recursive: true, force: true })
    },
  }
}

export function parseTasklistNames(text: string): string[] {
  const out: string[] = []
  for (const line of text.split(/\r?\n/)) {
    const m = /^"([^"]+)"/.exec(line.trim())
    if (m) out.push(m[1]!)
  }
  return out
}

export function parseLsappinfo(text: string): string[] {
  const out: string[] = []
  for (const m of text.matchAll(/bundleID="([^"]+)"/g)) out.push(m[1]!)
  for (const m of text.matchAll(/^\s*\d+\)\s+"([^"]+)"/gm)) out.push(m[1]!)
  return out
}

export interface SystemDepsInput extends SysRootsInput {
  table?: () => Promise<Array<{ name: string }> | null>
}

export function systemDeps(input: SystemDepsInput): SystemDeps {
  const roots = sysRoots(input)
  const { platform, fake } = input
  const procsFile = fake ? path.join(fake, 'procs.txt') : null
  const procs = async (): Promise<string[]> => {
    if (procsFile) return (await fs.readFile(procsFile, 'utf8').catch(() => '')).split(/\r?\n/).map((s) => s.trim()).filter(Boolean)
    const rows = await (input.table ?? coreRows)().catch(() => null)
    if (rows) return rows.map((r) => procName(r.name))
    if (platform === 'win32') {
      const text = await run(path.join(roots.windir, 'System32', 'tasklist.exe'), ['/FO', 'CSV', '/NH'])
      return parseTasklistNames(text ?? '').map(procName)
    }
    const text = await run('ps', ['-A', '-o', 'comm='])
    return (text ?? '').split(/\r?\n/).map((l) => procName(l.trim())).filter(Boolean)
  }
  const apps = async (): Promise<string[]> => {
    if (procsFile || platform !== 'darwin') return []
    return parseLsappinfo((await run('lsappinfo', ['list'])) ?? '')
  }
  const bin = roots.bin ? dirBin(roots.bin) : platform === 'win32' ? winBin(roots.windir) : macBin(roots.home)
  return { roots, procs, apps, bin, busy: platform === 'win32' ? busyWin : async () => false }
}

export async function macTempAllow(roots: SysRoots, real: (p: string) => Promise<string> = (p) => fs.realpath(p)): Promise<string[]> {
  if (roots.platform !== 'darwin' || !roots.macTmp) return []
  const t = roots.macTmp.replace(/\/+$/, '')
  const out: string[] = []
  for (const p of [t, path.posix.join(path.posix.dirname(t), 'C')]) {
    const r = macTempTree(await real(p).catch(() => null))
    if (r) out.push(r)
  }
  return out
}
