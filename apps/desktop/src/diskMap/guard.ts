import fs from 'node:fs/promises'
import path from 'node:path'
import { isAbsolutePath } from './map'

export type GuardReason = 'invalid' | 'relative' | 'root' | 'system' | 'protected' | 'outside' | 'changed'

export interface GuardContext {
  platform: NodeJS.Platform
  home: string
  keep: string[]
  roots: string[]
  keepTree?: string[]
  volumes?: string[]
}

export interface CanonicalIo {
  lstat: (p: string) => Promise<{ isSymbolicLink(): boolean }>
  realpath: (p: string) => Promise<string>
}

export interface Canonical {
  path: string
  link: boolean
  missing: boolean
}

export interface Resolved {
  reason: GuardReason | null
  canonical?: Canonical
}

export const realIo: CanonicalIo = {
  lstat: (p) => fs.lstat(p),
  realpath: (p) => fs.realpath(p),
}

const WIN_SUBTREE = ['Windows', '$Recycle.Bin', 'System Volume Information', 'Recovery', 'Boot', '$WinREAgent']
const WIN_SELF = ['Program Files', 'Program Files (x86)', 'ProgramData', 'Users', 'PerfLogs']
const MAC_SUBTREE = ['/System', '/usr', '/bin', '/sbin', '/etc', '/private', '/dev', '/cores', '/Library/Apple']
const MAC_SELF = ['/Library', '/Applications', '/Users', '/Volumes', '/opt', '/var', '/tmp']
const UNIX_SUBTREE = ['/usr', '/bin', '/sbin', '/etc', '/boot', '/proc', '/sys', '/dev', '/lib', '/lib32', '/lib64', '/libx32', '/run', '/snap']
const UNIX_SELF = ['/home', '/root', '/opt', '/var', '/tmp', '/srv', '/mnt', '/media']
const HOME_SELF_WIN = ['AppData', 'AppData\\Roaming', 'AppData\\Local', 'AppData\\LocalLow', '.config']
const HOME_SELF_MAC = ['Library', '.config']
const HOME_SELF_UNIX = ['.config', '.local', '.local/share']
const HOME_TREE_WIN = ['.ssh', '.gnupg']
const HOME_TREE_MAC = ['.ssh', '.gnupg', 'Library/Mobile Documents', 'Library/CloudStorage', 'Library/Keychains']
const HOME_TREE_UNIX = ['.ssh', '.gnupg']
const WIN_BAD_SEGMENT = /[. ]$|[:*?"<>|]/
const WIN_SHORT_NAME = /^[^~]{1,6}~\d{1,6}(\.[^.]{0,3})?$/

const libOf = (platform: NodeJS.Platform) => (platform === 'win32' ? path.win32 : path.posix)

export function normalizePath(p: string, platform: NodeJS.Platform): string {
  const lib = libOf(platform)
  let n = lib.normalize(p)
  if (platform === 'win32' && /^[a-z]:/.test(n)) n = n[0]!.toUpperCase() + n.slice(1)
  const root = lib.parse(n).root
  while (n.length > root.length && /[\\/]$/.test(n)) n = n.slice(0, -1)
  return n
}

function keyOf(p: string, platform: NodeJS.Platform): string {
  const n = normalizePath(p, platform)
  if (platform === 'darwin') return n.normalize('NFC').toLowerCase()
  return platform === 'win32' ? n.toLowerCase() : n
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

function segmentsOf(n: string, platform: NodeJS.Platform): string[] {
  const lib = libOf(platform)
  const root = lib.parse(n).root
  return n.slice(root.length).split(/[\\/]/).filter(Boolean)
}

function winLexicalBad(n: string): boolean {
  return segmentsOf(n, 'win32').some((s) => WIN_BAD_SEGMENT.test(s))
}

function aliasesOf(n: string, platform: NodeJS.Platform): string[] {
  if (platform !== 'win32') return [n]
  const m = /^\\\\[^\\]+\\([A-Za-z])\$(\\.*)?$/.exec(n)
  return m ? [n, normalizePath(`${m[1]!.toUpperCase()}:\\${(m[2] ?? '').replace(/^\\/, '')}`, platform)] : [n]
}

function homeLists(platform: NodeJS.Platform): { self: string[]; tree: string[] } {
  if (platform === 'win32') return { self: HOME_SELF_WIN, tree: HOME_TREE_WIN }
  if (platform === 'darwin') return { self: HOME_SELF_MAC, tree: HOME_TREE_MAC }
  return { self: HOME_SELF_UNIX, tree: HOME_TREE_UNIX }
}

function userParents(n: string, home: string, platform: NodeJS.Platform): string[] {
  const lib = libOf(platform)
  const out: string[] = []
  if (home) out.push(lib.dirname(home))
  if (platform === 'win32') {
    const root = path.win32.parse(n).root
    if (/^[A-Za-z]:\\$/.test(root)) out.push(root + 'Users')
  } else if (platform === 'darwin') {
    out.push('/Users', '/Volumes')
  } else {
    out.push('/home', '/mnt', '/media')
  }
  return out
}

function isProtected(n: string, ctx: GuardContext): boolean {
  const { platform } = ctx
  const lib = libOf(platform)
  const home = ctx.home ? normalizePath(ctx.home, platform) : ''
  const lists = homeLists(platform)
  const self = [home, ...(ctx.keep ?? []), ...(ctx.volumes ?? []), ...(home ? lists.self.map((s) => lib.join(home, s)) : [])]
  if (self.some((k) => k && isUnder(k, n, platform))) return true
  const tree = [...(home ? lists.tree.map((s) => lib.join(home, s)) : []), ...(ctx.keepTree ?? [])]
  if (tree.some((k) => k && (isUnder(n, k, platform) || isUnder(k, n, platform)))) return true
  const parent = lib.dirname(n)
  if (parent !== n && userParents(n, home, platform).some((u) => keyOf(u, platform) === keyOf(parent, platform))) return true
  if (platform === 'linux' && keyOf(lib.dirname(parent), platform) === '/media') return true
  if (home && keyOf(parent, platform) === keyOf(home, platform) && lib.basename(n).toLowerCase().startsWith('onedrive')) return true
  return false
}

function verdict(n: string, ctx: GuardContext, needRoot: boolean): GuardReason | null {
  for (const a of aliasesOf(n, ctx.platform)) {
    if (isVolumeRoot(a, ctx.platform)) return 'root'
    if (systemReason(a, ctx.platform)) return 'system'
    if (isProtected(a, ctx)) return 'protected'
  }
  if (needRoot && !(ctx.roots ?? []).some((r) => isUnder(n, r, ctx.platform))) return 'outside'
  return null
}

export function checkDeletable(p: unknown, ctx: GuardContext): GuardReason | null {
  if (typeof p !== 'string' || !p || p.includes('\0') || p.length > 32_000) return 'invalid'
  if (!isAbsolutePath(p, ctx.platform)) return 'relative'
  const n = normalizePath(p, ctx.platform)
  if (ctx.platform === 'win32' && winLexicalBad(n)) return 'invalid'
  return verdict(n, ctx, true)
}

function plainCanonical(raw: string, platform: NodeJS.Platform): string | null {
  if (platform !== 'win32') return raw
  const m = /^[\\/]{2}[?.][\\/](.*)$/.exec(raw)
  if (!m) return raw
  const rest = m[1]!
  if (/^[A-Za-z]:[\\/]/.test(rest)) return rest
  if (/^UNC[\\/]/i.test(rest)) return '\\\\' + rest.slice(4)
  return null
}

const codeOf = (err: unknown): string => {
  const c = (err as { code?: unknown } | null)?.code
  return typeof c === 'string' ? c : ''
}

export async function canonicalOf(p: string, platform: NodeJS.Platform, io: CanonicalIo = realIo): Promise<Canonical | null> {
  const lib = libOf(platform)
  const n = normalizePath(p, platform)
  let link: boolean
  try {
    link = (await io.lstat(n)).isSymbolicLink()
  } catch (err) {
    if (codeOf(err) !== 'ENOENT') return null
    try {
      const parent = await io.realpath(lib.dirname(n))
      const base = plainCanonical(parent, platform)
      return base === null ? null : { path: normalizePath(lib.join(base, lib.basename(n)), platform), link: false, missing: true }
    } catch (e) {
      return codeOf(e) === 'ENOENT' ? { path: n, link: false, missing: true } : null
    }
  }
  try {
    const raw = link ? lib.join(await io.realpath(lib.dirname(n)), lib.basename(n)) : await io.realpath(n)
    const plain = plainCanonical(raw, platform)
    return plain === null ? null : { path: normalizePath(plain, platform), link, missing: false }
  } catch {
    return null
  }
}

function canonicalVerdict(c: Canonical, ctx: GuardContext): GuardReason | null {
  const { platform } = ctx
  if (!isAbsolutePath(c.path, platform)) return 'invalid'
  if (platform === 'win32') {
    if (winLexicalBad(c.path)) return 'invalid'
    const segs = segmentsOf(c.path, platform)
    const checked = c.link || c.missing ? segs.slice(0, -1) : segs
    if (checked.some((s) => WIN_SHORT_NAME.test(s))) return 'invalid'
  }
  return verdict(c.path, ctx, false)
}

export async function resolveDeletable(p: unknown, ctx: GuardContext, io: CanonicalIo = realIo): Promise<Resolved> {
  const lex = checkDeletable(p, ctx)
  if (lex) return { reason: lex }
  const canonical = await canonicalOf(p as string, ctx.platform, io)
  if (!canonical) return { reason: 'invalid' }
  return { reason: canonicalVerdict(canonical, ctx), canonical }
}

export async function checkDeletableCanonical(p: unknown, ctx: GuardContext, io: CanonicalIo = realIo): Promise<GuardReason | null> {
  return (await resolveDeletable(p, ctx, io)).reason
}

export function sameCanonical(a: Canonical, b: Canonical, platform: NodeJS.Platform): boolean {
  return a.link === b.link && a.missing === b.missing && keyOf(a.path, platform) === keyOf(b.path, platform)
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
