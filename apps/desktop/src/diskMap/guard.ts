import path from 'node:path'
import { isAbsolutePath } from './map'

export type GuardReason = 'invalid' | 'relative' | 'root' | 'system' | 'protected' | 'outside'

export interface GuardContext {
  platform: NodeJS.Platform
  home: string
  keep: string[]
  roots: string[]
}

const WIN_SUBTREE = ['Windows', '$Recycle.Bin', 'System Volume Information', 'Recovery', 'Boot', '$WinREAgent']
const WIN_SELF = ['Program Files', 'Program Files (x86)', 'ProgramData', 'Users', 'PerfLogs']
const MAC_SUBTREE = ['/System', '/usr', '/bin', '/sbin', '/etc', '/private', '/dev', '/cores', '/Library/Apple']
const MAC_SELF = ['/Library', '/Applications', '/Users', '/Volumes', '/opt', '/var', '/tmp']
const UNIX_SUBTREE = ['/usr', '/bin', '/sbin', '/etc', '/boot', '/proc', '/sys', '/dev', '/lib', '/lib32', '/lib64', '/libx32', '/run', '/snap']
const UNIX_SELF = ['/home', '/root', '/opt', '/var', '/tmp', '/srv', '/mnt', '/media']

export function normalizePath(p: string, platform: NodeJS.Platform): string {
  const lib = platform === 'win32' ? path.win32 : path.posix
  let n = lib.normalize(p)
  if (platform === 'win32' && /^[a-z]:/.test(n)) n = n[0]!.toUpperCase() + n.slice(1)
  const root = lib.parse(n).root
  while (n.length > root.length && /[\\/]$/.test(n)) n = n.slice(0, -1)
  return n
}

function keyOf(p: string, platform: NodeJS.Platform): string {
  const n = normalizePath(p, platform)
  return platform === 'win32' || platform === 'darwin' ? n.toLowerCase() : n
}

export function isUnder(child: string, base: string, platform: NodeJS.Platform): boolean {
  const c = keyOf(child, platform)
  const b = keyOf(base, platform)
  if (c === b) return true
  const sep = platform === 'win32' ? '\\' : '/'
  return c.startsWith(b.endsWith(sep) ? b : b + sep)
}

function isVolumeRoot(n: string, platform: NodeJS.Platform): boolean {
  if (platform !== 'win32') return n === '/'
  if (/^[A-Za-z]:\\?$/.test(n)) return true
  return path.win32.parse(n).root.replace(/\\$/, '').toLowerCase() === n.replace(/\\$/, '').toLowerCase()
}

function systemReason(n: string, platform: NodeJS.Platform): boolean {
  if (platform === 'win32') {
    const root = path.win32.parse(n).root
    if (!/^[A-Za-z]:\\$/.test(root)) return false
    if (WIN_SUBTREE.some((d) => isUnder(n, root + d, platform))) return true
    if (WIN_SELF.some((d) => keyOf(n, platform) === keyOf(root + d, platform))) return true
    return false
  }
  const sub = platform === 'darwin' ? MAC_SUBTREE : UNIX_SUBTREE
  const self = platform === 'darwin' ? MAC_SELF : UNIX_SELF
  if (sub.some((d) => isUnder(n, d, platform))) return true
  return self.some((d) => keyOf(n, platform) === keyOf(d, platform))
}

export function checkDeletable(p: unknown, ctx: GuardContext): GuardReason | null {
  if (typeof p !== 'string' || !p || p.includes('\0') || p.length > 32_000) return 'invalid'
  if (!isAbsolutePath(p, ctx.platform)) return 'relative'
  const n = normalizePath(p, ctx.platform)
  if (isVolumeRoot(n, ctx.platform)) return 'root'
  if (systemReason(n, ctx.platform)) return 'system'
  const home = ctx.home ? normalizePath(ctx.home, ctx.platform) : ''
  if (home && isUnder(home, n, ctx.platform)) return 'protected'
  if (ctx.keep.some((k) => k && isUnder(k, n, ctx.platform))) return 'protected'
  if (!ctx.roots.some((r) => isUnder(n, r, ctx.platform))) return 'outside'
  return null
}

export function topLevel(paths: string[], platform: NodeJS.Platform): string[] {
  const seen = new Map<string, string>()
  for (const p of paths) {
    const k = keyOf(p, platform)
    if (!seen.has(k)) seen.set(k, normalizePath(p, platform))
  }
  const list = [...seen.values()]
  return list.filter((p) => !list.some((q) => q !== p && keyOf(q, platform) !== keyOf(p, platform) && isUnder(p, q, platform)))
}
