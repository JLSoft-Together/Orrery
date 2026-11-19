import fs from 'node:fs/promises'
import path from 'node:path'
import type { AppLeftover, AppLeftoverBase } from '@ccm/shared'

export interface LeftoverRoot {
  base: AppLeftoverBase
  dir: string
  files?: boolean
}

export interface LeftoverApp {
  name: string
  publisher: string | null
  bundleId: string | null
  location: string | null
}

export interface LeftoverQuery extends LeftoverApp {
  installed: boolean
  others: LeftoverApp[]
}

export interface DirEntry {
  name: string
  dir: boolean
  link: boolean
}

export interface LeftoverIo {
  readdir: (dir: string) => Promise<DirEntry[]>
  exists: (p: string) => Promise<boolean>
}

export const realLeftoverIo: LeftoverIo = {
  readdir: async (dir) => (await fs.readdir(dir, { withFileTypes: true }).catch(() => [])).map((e) => ({ name: e.name, dir: e.isDirectory(), link: e.isSymbolicLink() })),
  exists: async (p) => (await fs.lstat(p).catch(() => null)) !== null,
}

export const GENERIC = new Set([
  'microsoft', 'microsoftcorporation', 'google', 'googlellc', 'apple', 'appleinc', 'adobe', 'mozilla', 'intel', 'nvidia', 'nvidiacorporation', 'amd', 'oracle',
  'packages', 'programs', 'temp', 'tmp', 'cache', 'caches', 'commonfiles', 'windows', 'system', 'crashdumps', 'crashreports', 'logs', 'log', 'data', 'config',
  'settings', 'publishers', 'squirreltemp', 'electron', 'node', 'npm', 'pip', 'python', 'java', 'app', 'apps', 'application', 'applications', 'updater', 'update',
  'updates', 'installer', 'setup', 'uninstall', 'default', 'user', 'users', 'shared', 'common', 'local', 'roaming', 'cloud', 'sync', 'helper', 'tools', 'software',
  'virtualstore', 'connecteddevicesplatform', 'd3dscache', 'diagnostics', 'documents', 'downloads', 'desktop', 'pictures', 'music', 'videos', 'library',
  'preferences', 'containers', 'groupcontainers', 'savedapplicationstate', 'applicationsupport', 'comapple', 'orrery', 'agentmonitor', 'ccm',
])

const SUFFIXES = /\b(incorporated|inc|llc|ltd|limited|corp|corporation|co|company|gmbh|ag|sa|srl|bv|oy|ab|pty|plc|kk)\b\.?/gi

export function norm(s: string): string {
  return s.normalize('NFC').toLowerCase().replace(/[^\p{L}\p{N}]+/gu, '')
}

export function appKey(name: string): string {
  const core = name
    .replace(/\.app$/i, '')
    .replace(/\([^)]*\)/g, ' ')
    .replace(/\b(x64|x86|amd64|arm64|64-bit|32-bit|64 bit|32 bit)\b/gi, ' ')
    .replace(/\s+(v(ersion)?|phiên bản)\s*\d[\w.\-]*/gi, ' ')
    .replace(/\s+v?\d+(\.\d+)+[\w.\-]*/g, ' ')
    .replace(/\s+\d+$/g, ' ')
  return norm(core)
}

export function publisherKey(p: string | null): string {
  if (!p) return ''
  return norm(p.replace(/^công ty (tnhh|cổ phần)\s*/i, '').replace(SUFFIXES, ' '))
}

const isUnder = (child: string, base: string, lib: typeof path.win32): boolean => {
  const c = lib.normalize(child).toLowerCase()
  const b = lib.normalize(base).toLowerCase().replace(/[\\/]+$/, '')
  return c === b || c.startsWith(b + lib.sep)
}

type Hit = Omit<AppLeftover, 'size'>

export async function findLeftovers(q: LeftoverQuery, roots: LeftoverRoot[], platform: NodeJS.Platform, io: LeftoverIo = realLeftoverIo): Promise<Hit[]> {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const key = appKey(q.name)
  const pub = publisherKey(q.publisher)
  const bundle = q.bundleId?.toLowerCase() ?? null
  const bundleKey = bundle ? norm(bundle) : ''
  const usable = (k: string, min: number): boolean => k.length >= min && !GENERIC.has(k)
  const otherKeys = new Set(q.others.map((o) => appKey(o.name)).filter((k) => k && k !== key))
  const otherBundles = new Set(q.others.map((o) => o.bundleId?.toLowerCase()).filter((b): b is string => !!b && b !== bundle))
  const pubShared = !!pub && q.others.some((o) => publisherKey(o.publisher) === pub)
  const otherLocations = [...q.others.map((o) => o.location), q.installed ? q.location : null].filter((l): l is string => !!l)
  const owned = (p: string): boolean => otherLocations.some((l) => isUnder(p, l, lib) || isUnder(l, p, lib))
  const hits = new Map<string, Hit>()
  const add = (p: string, base: AppLeftoverBase, match: 'exact' | 'loose', file: boolean): void => {
    const k = p.toLowerCase()
    const prev = hits.get(k)
    if (prev && (prev.match === 'exact' || match === 'loose')) return
    if (owned(p)) return
    hits.set(k, { path: p, base, match, file })
  }
  const nameMatch = (raw: string): 'exact' | 'loose' | null => {
    const lower = raw.toLowerCase()
    const n = norm(raw.replace(/\.(plist|savedstate)$/i, ''))
    if (!n || otherKeys.has(n)) return null
    if (bundle && otherBundles.has(lower.replace(/\.(plist|savedstate)$/i, ''))) return null
    if (bundle && (lower === bundle || lower === `${bundle}.plist` || lower === `${bundle}.savedstate`)) return 'exact'
    if (bundle && lower.startsWith(`${bundle}.`)) return 'loose'
    if (GENERIC.has(n)) return null
    if (usable(key, 3) && n === key) return 'exact'
    if (usable(bundleKey, 6) && n === bundleKey) return 'exact'
    if (usable(key, 4) && n.length >= 4 && (n.startsWith(key) || key.startsWith(n))) {
      for (const o of otherKeys) if (o.length >= 4 && (o.startsWith(n) || n.startsWith(o))) return null
      return 'loose'
    }
    return null
  }
  for (const root of roots) {
    const entries = await io.readdir(root.dir)
    for (const e of entries) {
      if (e.link) continue
      if (!e.dir && !root.files) continue
      const full = lib.join(root.dir, e.name)
      const n = norm(e.name)
      if (e.dir && pub && n === pub && usable(pub, 3)) {
        const inner = await io.readdir(full)
        let any = false
        for (const c of inner) {
          if (c.link || !c.dir) continue
          const m = nameMatch(c.name)
          if (m) {
            any = true
            add(lib.join(full, c.name), root.base, m, false)
          }
        }
        if (!any && !pubShared && n !== key) add(full, root.base, 'loose', false)
        if (n !== key) continue
      }
      const m = nameMatch(e.name)
      if (m) add(full, root.base, m, !e.dir)
    }
  }
  if (!q.installed && q.location && (await io.exists(q.location))) add(q.location, 'location', 'exact', false)
  const list = [...hits.values()]
  return list.filter((h) => !list.some((o) => o !== h && isUnder(h.path, o.path, lib) && o.path.toLowerCase() !== h.path.toLowerCase()))
}

export function windowsLeftoverRoots(env: NodeJS.ProcessEnv): LeftoverRoot[] {
  const out: LeftoverRoot[] = []
  if (env.APPDATA) out.push({ base: 'appdata', dir: env.APPDATA })
  if (env.LOCALAPPDATA) {
    out.push({ base: 'localappdata', dir: env.LOCALAPPDATA })
    out.push({ base: 'localappdata', dir: path.win32.join(env.LOCALAPPDATA, 'Programs') })
  }
  const pd = env.ProgramData ?? env.PROGRAMDATA
  if (pd) out.push({ base: 'programdata', dir: pd })
  return out
}

export function macLeftoverRoots(home: string): LeftoverRoot[] {
  const lib = path.posix.join(home, 'Library')
  return [
    { base: 'support', dir: path.posix.join(lib, 'Application Support') },
    { base: 'caches', dir: path.posix.join(lib, 'Caches') },
    { base: 'preferences', dir: path.posix.join(lib, 'Preferences'), files: true },
    { base: 'containers', dir: path.posix.join(lib, 'Containers') },
    { base: 'savedState', dir: path.posix.join(lib, 'Saved Application State') },
    { base: 'logs', dir: path.posix.join(lib, 'Logs') },
  ]
}
