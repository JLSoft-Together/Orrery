import { promises as fs } from 'node:fs'
import path from 'node:path'
import type { OrganizeJunkItem, OrganizeJunkResult, OrganizeJunkState } from '@ccm/shared'
import { checkDeletable, isUnder, type GuardContext } from '../diskMap/guard'
import { mapLimit } from '../diskMap/delete'

export interface RawEntry {
  path: string
  name: string
  dir: boolean
  count?: number
  cloud?: boolean
  link?: boolean
  mount?: boolean
  denied?: boolean
  hidden?: boolean
}

export interface JunkDeps {
  platform: NodeJS.Platform
  guard: () => Promise<GuardContext | null>
  search: (q: Record<string, unknown>) => Promise<RawEntry[] | null>
  skipTree: string[]
  allowTree?: string[]
  readShortcut?: (p: string) => string | null
  volumes?: () => Promise<Array<{ mount: string; network: boolean }>>
  readdir?: (p: string) => Promise<string[]>
  stat?: (p: string) => Promise<unknown>
  lstat?: (p: string) => Promise<{ isSymbolicLink(): boolean; isDirectory(): boolean }>
  readlink?: (p: string) => Promise<string>
}

const MAX_ITEMS = 2000
const SEARCH_LIMIT = 10_000
const SKIP_SEGMENTS = new Set(['node_modules', '$recycle.bin', 'system volume information', 'windows', 'program files', 'program files (x86)', '__pycache__', 'build', 'dist', 'target', 'out'])
const APP_SEGMENTS = new Set(['appdata', 'library', 'programdata'])

const codeOf = (err: unknown): string => {
  const c = (err as { code?: unknown } | null)?.code
  return typeof c === 'string' ? c : ''
}

function rawOf(v: unknown): RawEntry | null {
  if (!v || typeof v !== 'object') return null
  const o = v as Record<string, unknown>
  if (typeof o.path !== 'string' || typeof o.name !== 'string') return null
  return { path: o.path, name: o.name, dir: o.dir === true, count: typeof o.count === 'number' ? o.count : undefined, cloud: o.cloud === true, link: o.link === true, mount: o.mount === true, denied: o.denied === true, hidden: o.hidden === true }
}

export function rawItems(v: unknown): RawEntry[] | null {
  const items = (v as { items?: unknown } | null)?.items
  return Array.isArray(items) ? items.map(rawOf).filter((x): x is RawEntry => x !== null) : null
}

export function skippedPath(p: string, platform: NodeJS.Platform, skipTree: string[], allowTree: string[] = []): boolean {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const segs = p.slice(lib.parse(p).root.length).split(/[\\/]/).filter(Boolean)
  const allowed = allowTree.some((a) => a && isUnder(p, a, platform))
  if (segs.some((s) => s.startsWith('.') || SKIP_SEGMENTS.has(s.toLowerCase()) || (!allowed && APP_SEGMENTS.has(s.toLowerCase())))) return true
  return skipTree.some((k) => k && isUnder(p, k, platform))
}

function driveState(target: string, platform: NodeJS.Platform, vols: Array<{ mount: string; network: boolean }> | null): 'ok' | 'unknown' {
  if (platform === 'win32') {
    if (/^\\\\/.test(target)) return 'unknown'
    const m = /^([A-Za-z]):[\\/]/.exec(target)
    if (!m) return 'unknown'
    if (!vols) return 'ok'
    const v = vols.find((x) => x.mount.slice(0, 1).toUpperCase() === m[1]!.toUpperCase())
    return !v || v.network ? 'unknown' : 'ok'
  }
  if (!path.posix.isAbsolute(target)) return 'ok'
  const m = /^\/Volumes\/[^/]+/.exec(target)
  if (m && vols && !vols.some((x) => x.mount === m[0])) return 'unknown'
  if (/^\/(net|Network)\//.test(target)) return 'unknown'
  const v = vols?.find((x) => x.mount !== '/' && isUnder(target, x.mount, platform))
  return v?.network ? 'unknown' : 'ok'
}

export async function scanJunk(deps: JunkDeps): Promise<OrganizeJunkResult> {
  const ctx = await deps.guard()
  if (!ctx) return { indexed: false, items: [] }
  const { platform } = ctx
  const readdir = deps.readdir ?? ((p: string) => fs.readdir(p))
  const stat = deps.stat ?? ((p: string) => fs.stat(p))
  const lstat = deps.lstat ?? ((p: string) => fs.lstat(p))
  const readlink = deps.readlink ?? ((p: string) => fs.readlink(p))
  const allowed = (p: string): boolean => !skippedPath(p, platform, deps.skipTree, deps.allowTree) && checkDeletable(p, ctx) === null
  const items: OrganizeJunkItem[] = []
  let truncated = false

  const dirs = await deps.search({ only: 'dir', maxSize: 0, sort: 'name', limit: SEARCH_LIMIT })
  if (dirs === null) return { indexed: false, items: [] }
  if (dirs.length >= SEARCH_LIMIT) truncated = true
  const emptyCandidates = dirs.filter((d) => d.dir && d.count === 0 && !d.link && !d.mount && !d.denied && !d.hidden && !d.cloud && allowed(d.path))
  const checked = await mapLimit(emptyCandidates, 16, async (d) => {
    try {
      const st = await lstat(d.path)
      if (st.isSymbolicLink() || !st.isDirectory()) return null
      return (await readdir(d.path)).length === 0 ? d : null
    } catch {
      return null
    }
  })
  for (const d of checked) if (d) items.push({ path: d.path, name: d.name, kind: 'empty', state: 'empty' })

  const vols = deps.volumes ? await deps.volumes().catch(() => null) : null
  if (platform === 'win32' && deps.readShortcut) {
    const links = (await deps.search({ query: '*.lnk', only: 'file', sort: 'name', limit: SEARCH_LIMIT })) ?? []
    if (links.length >= SEARCH_LIMIT) truncated = true
    const cands = links.filter((l) => !l.dir && l.name.toLowerCase().endsWith('.lnk') && !l.cloud && !l.denied && allowed(l.path))
    const res = await mapLimit(cands, 8, async (l): Promise<OrganizeJunkItem | null> => {
      let target: string | null
      try {
        target = deps.readShortcut!(l.path)
      } catch {
        return null
      }
      if (!target) return null
      if (driveState(target, platform, vols) === 'unknown') return { path: l.path, name: l.name, kind: 'shortcut', state: 'unknown', target }
      try {
        await stat(target)
        return null
      } catch (err) {
        const code = codeOf(err)
        const state: OrganizeJunkState = code === 'ENOENT' || code === 'ENOTDIR' ? 'broken' : 'unknown'
        return { path: l.path, name: l.name, kind: 'shortcut', state, target }
      }
    })
    for (const r of res) if (r) items.push(r)
  } else if (platform === 'darwin') {
    const files = (await deps.search({ only: 'file', maxSize: 0, sort: 'name', limit: SEARCH_LIMIT })) ?? []
    if (files.length >= SEARCH_LIMIT) truncated = true
    const cands = files.filter((f) => f.link && !f.dir && allowed(f.path))
    const res = await mapLimit(cands, 8, async (f): Promise<OrganizeJunkItem | null> => {
      try {
        if (!(await lstat(f.path)).isSymbolicLink()) return null
        const raw = await readlink(f.path)
        const target = path.posix.resolve(path.posix.dirname(f.path), raw)
        if (driveState(target, platform, vols) === 'unknown') return { path: f.path, name: f.name, kind: 'shortcut', state: 'unknown', target }
        try {
          await stat(f.path)
          return null
        } catch (err) {
          return codeOf(err) === 'ENOENT' ? { path: f.path, name: f.name, kind: 'shortcut', state: 'broken', target } : null
        }
      } catch {
        return null
      }
    })
    for (const r of res) if (r) items.push(r)
  }
  if (items.length > MAX_ITEMS) truncated = true
  return { indexed: true, items: items.slice(0, MAX_ITEMS), ...(truncated ? { truncated: true } : {}) }
}
