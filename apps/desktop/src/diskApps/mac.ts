import { execFile } from 'node:child_process'
import fs from 'node:fs/promises'
import path from 'node:path'
import type { InstalledApp } from '@ccm/shared'

export interface PlistInfo {
  bundleId: string | null
  name: string | null
  version: string | null
}

const decodeXml = (s: string): string =>
  s
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_, n: string) => String.fromCodePoint(Number(n)))
    .replace(/&#x([0-9a-f]+);/gi, (_, n: string) => String.fromCodePoint(parseInt(n, 16)))
    .replace(/&amp;/g, '&')

export function parsePlistStrings(xml: string): Map<string, string> {
  const out = new Map<string, string>()
  const body = /<plist[^>]*>\s*<dict>([\s\S]*)<\/dict>\s*<\/plist>/.exec(xml)?.[1]
  if (!body) return out
  let depth = 0
  const re = /<(\/?)(dict|array)\b[^>]*?(\/?)>|<key>([^<]*)<\/key>\s*<string>([^<]*)<\/string>|<key>[^<]*<\/key>/g
  let m: RegExpExecArray | null
  while ((m = re.exec(body))) {
    if (m[2]) {
      if (m[3] === '/') continue
      depth += m[1] === '/' ? -1 : 1
      continue
    }
    if (depth === 0 && m[4] !== undefined && m[5] !== undefined) out.set(decodeXml(m[4].trim()), decodeXml(m[5].trim()))
  }
  return out
}

export function plistInfo(map: Map<string, string>): PlistInfo {
  const pick = (...keys: string[]): string | null => {
    for (const k of keys) {
      const v = map.get(k)?.trim()
      if (v) return v
    }
    return null
  }
  return {
    bundleId: pick('CFBundleIdentifier'),
    name: pick('CFBundleDisplayName', 'CFBundleName'),
    version: pick('CFBundleShortVersionString', 'CFBundleVersion'),
  }
}

function run(file: string, args: string[], timeout = 15_000): Promise<string | null> {
  return new Promise((resolve) => {
    execFile(file, args, { timeout, maxBuffer: 8_000_000, env: { ...process.env, LC_ALL: 'C' } }, (err, stdout) => resolve(err ? null : String(stdout)))
  })
}

async function readPlist(file: string): Promise<Map<string, string>> {
  const buf = await fs.readFile(file).catch(() => null)
  if (!buf) return new Map()
  if (buf.subarray(0, 6).toString('latin1') !== 'bplist') return parsePlistStrings(buf.toString('utf8'))
  const xml = await run('/usr/bin/plutil', ['-convert', 'xml1', '-o', '-', file])
  return xml ? parsePlistStrings(xml) : new Map()
}

export function blockedMac(appPath: string, bundleId: string | null, selfBundle: string | null): InstalledApp['blocked'] {
  if (selfBundle && path.posix.normalize(appPath) === path.posix.normalize(selfBundle)) return 'self'
  if (bundleId === 'dev.ccm.monitor') return 'self'
  if (appPath.startsWith('/System/')) return 'system'
  if (bundleId && /^com\.apple\./i.test(bundleId)) return 'system'
  return null
}

export function lastUsedOf(text: string | null, count: number): (number | null)[] {
  const parts = text === null ? [] : text.split('\0')
  return Array.from({ length: count }, (_, i) => {
    const v = parts[i]?.trim()
    if (!v || v === '(null)') return null
    const ms = Date.parse(v.replace(' ', 'T').replace(/ ([+-]\d{2})(\d{2})$/, '$1:$2'))
    return Number.isFinite(ms) ? ms : null
  })
}

async function duBytes(dir: string): Promise<number | null> {
  const out = await run('/usr/bin/du', ['-sk', dir], 20_000)
  const kb = out ? Number(/^(\d+)/.exec(out.trim())?.[1]) : NaN
  return Number.isFinite(kb) ? kb * 1024 : null
}

export async function readMacApps(opts: { roots: string[]; selfBundle: string | null; indexSizes?: (paths: string[]) => Promise<Record<string, number | null> | null> }): Promise<InstalledApp[]> {
  const found: { path: string; scope: InstalledApp['scope'] }[] = []
  for (const [i, root] of opts.roots.entries()) {
    const entries = await fs.readdir(root, { withFileTypes: true }).catch(() => [])
    for (const e of entries) if (e.isDirectory() && e.name.endsWith('.app')) found.push({ path: path.posix.join(root, e.name), scope: i === 0 ? 'machine' : 'user' })
  }
  const index = found.length ? ((await opts.indexSizes?.(found.map((f) => f.path)).catch(() => null)) ?? null) : null
  const used = found.length ? lastUsedOf(await run('/usr/bin/mdls', ['-name', 'kMDItemLastUsedDate', '-raw', ...found.map((f) => f.path)]), found.length) : []
  const apps: InstalledApp[] = []
  let next = 0
  const worker = async (): Promise<void> => {
    while (next < found.length) {
      const i = next++
      const f = found[i]!
      const st = await fs.stat(f.path).catch(() => null)
      const info = plistInfo(await readPlist(path.posix.join(f.path, 'Contents', 'Info.plist')))
      const fromIndex = index?.[f.path] ?? null
      const size = fromIndex ?? (await duBytes(f.path))
      apps[i] = {
        id: f.path,
        name: info.name ?? path.posix.basename(f.path, '.app'),
        publisher: null,
        version: info.version,
        installedAt: st ? Math.round(st.birthtimeMs || st.mtimeMs) : null,
        size,
        sizeFrom: fromIndex !== null ? 'index' : size !== null ? 'du' : null,
        location: f.path,
        source: 'bundle',
        scope: f.scope,
        arch: null,
        bundleId: info.bundleId,
        lastUsed: used[i] ?? null,
        uninstall: null,
        blocked: blockedMac(f.path, info.bundleId, opts.selfBundle),
      }
    }
  }
  await Promise.all(Array.from({ length: Math.min(4, found.length) }, worker))
  return apps.filter(Boolean)
}
