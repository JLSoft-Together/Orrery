import { execFile } from 'node:child_process'
import fs from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import type { InstalledApp, InstalledAppScope } from '@ccm/shared'

export type RegValue = string | number | string[]

export interface RegKey {
  key: string
  values: Map<string, RegValue>
}

export interface RegSource {
  key: string
  scope: InstalledAppScope
  arch: '64' | '32' | null
}

const UNINSTALL = 'Microsoft\\Windows\\CurrentVersion\\Uninstall'

export const REG_SOURCES: RegSource[] = [
  { key: `HKEY_LOCAL_MACHINE\\SOFTWARE\\${UNINSTALL}`, scope: 'machine', arch: '64' },
  { key: `HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\${UNINSTALL}`, scope: 'machine', arch: '32' },
  { key: `HKEY_CURRENT_USER\\Software\\${UNINSTALL}`, scope: 'user', arch: null },
]

function unquote(s: string): string | null {
  if (!s.startsWith('"')) return null
  let out = ''
  for (let i = 1; i < s.length; i++) {
    const c = s[i]!
    if (c === '\\' && i + 1 < s.length) {
      out += s[++i]
      continue
    }
    if (c === '"') return out
    out += c
  }
  return null
}

function nameEnd(line: string): number {
  for (let i = 1; i < line.length; i++) {
    if (line[i] === '\\') {
      i++
      continue
    }
    if (line[i] === '"') return i
  }
  return -1
}

function hexBytes(s: string): Buffer {
  const parts = s.split(',').map((x) => x.trim()).filter(Boolean)
  const bytes: number[] = []
  for (const p of parts) {
    if (!/^[0-9a-f]{1,2}$/i.test(p)) break
    bytes.push(parseInt(p, 16))
  }
  return Buffer.from(bytes)
}

function utf16(buf: Buffer): string {
  const even = buf.length - (buf.length % 2)
  return buf.subarray(0, even).toString('utf16le')
}

export function parseRegValue(raw: string): RegValue | null {
  if (raw.startsWith('"')) return unquote(raw)
  const dword = /^dword:([0-9a-f]{1,8})$/i.exec(raw)
  if (dword) return parseInt(dword[1]!, 16)
  const hex = /^hex(?:\(([0-9a-f]+)\))?:(.*)$/i.exec(raw)
  if (!hex) return null
  const type = (hex[1] ?? '3').toLowerCase()
  const buf = hexBytes(hex[2]!)
  if (type === '1' || type === '2') return utf16(buf).replace(/\0+$/, '')
  if (type === '7') return utf16(buf).split('\0').filter(Boolean)
  if (type === 'b' && buf.length >= 8) return Number(buf.readBigUInt64LE(0))
  if (type === '4' && buf.length >= 4) return buf.readUInt32LE(0)
  return null
}

export function parseRegExport(text: string): RegKey[] {
  const lines = text.replace(/^\uFEFF/, '').split(/\r?\n/)
  const keys: RegKey[] = []
  let cur: RegKey | null = null
  for (let i = 0; i < lines.length; i++) {
    let line = lines[i]!
    while (/,\\$/.test(line) && i + 1 < lines.length) line = line.slice(0, -1) + lines[++i]!.trim()
    if (line.startsWith('[') && line.endsWith(']')) {
      const key = line.slice(1, -1)
      cur = key.startsWith('-') ? null : { key, values: new Map() }
      if (cur) keys.push(cur)
      continue
    }
    if (!cur || !line) continue
    let name: string
    let rest: string
    if (line.startsWith('@=')) {
      name = ''
      rest = line.slice(2)
    } else if (line.startsWith('"')) {
      const end = nameEnd(line)
      if (end < 0 || line[end + 1] !== '=') continue
      name = unquote(line.slice(0, end + 1)) ?? ''
      rest = line.slice(end + 2)
    } else continue
    const value = parseRegValue(rest)
    if (value !== null) cur.values.set(name.toLowerCase(), value)
  }
  return keys
}

const str = (v: RegValue | undefined): string | null => (typeof v === 'string' && v.trim() ? v.trim() : null)
const num = (v: RegValue | undefined): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : typeof v === 'string' && /^\d+$/.test(v.trim()) ? Number(v.trim()) : null)

export function expandEnv(s: string, env: NodeJS.ProcessEnv): string {
  const lower = new Map(Object.entries(env).map(([k, v]) => [k.toLowerCase(), v]))
  return s.replace(/%([A-Za-z0-9_()]+)%/g, (all, name: string) => lower.get(name.toLowerCase()) ?? all)
}

export function installDate(v: string | null): number | null {
  const m = v ? /^(\d{4})(\d{2})(\d{2})$/.exec(v) : null
  if (!m) return null
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])]
  if (mo < 1 || mo > 12 || d < 1 || d > 31 || y < 1990) return null
  return new Date(y, mo - 1, d).getTime()
}

function cleanLocation(v: string | null, env: NodeJS.ProcessEnv): string | null {
  if (!v) return null
  let p = expandEnv(v.trim().replace(/^"(.*)"$/, '$1'), env)
  if (!/^[a-z]:\\/i.test(p) && !p.startsWith('\\\\')) return null
  while (p.length > 3 && /[\\/]$/.test(p)) p = p.slice(0, -1)
  return p
}

const UPDATE_TYPES = new Set(['update', 'hotfix', 'security update', 'service pack', 'update rollup'])

export function hiddenEntry(values: Map<string, RegValue>): boolean {
  if (num(values.get('systemcomponent')) === 1) return true
  if (str(values.get('parentkeyname'))) return true
  const rel = str(values.get('releasetype'))?.toLowerCase()
  if (rel && UPDATE_TYPES.has(rel)) return true
  const name = str(values.get('displayname'))
  if (!name) return true
  if (/^(security )?update for |^hotfix for |\(KB\d{6,}\)$/i.test(name)) return true
  return false
}

export function appsFromKeys(keys: RegKey[], src: RegSource, env: NodeJS.ProcessEnv): InstalledApp[] {
  const out: InstalledApp[] = []
  const prefix = `${src.key}\\`.toLowerCase()
  for (const k of keys) {
    const lower = k.key.toLowerCase()
    if (!lower.startsWith(prefix) || lower.slice(prefix.length).includes('\\')) continue
    if (hiddenEntry(k.values)) continue
    const est = num(k.values.get('estimatedsize'))
    out.push({
      id: k.key,
      name: str(k.values.get('displayname'))!,
      publisher: str(k.values.get('publisher')),
      version: str(k.values.get('displayversion')),
      installedAt: installDate(str(k.values.get('installdate'))),
      size: est !== null && est > 0 ? est * 1024 : null,
      sizeFrom: est !== null && est > 0 ? 'estimate' : null,
      location: cleanLocation(str(k.values.get('installlocation')), env),
      source: 'registry',
      scope: src.scope,
      arch: src.arch,
      bundleId: null,
      lastUsed: null,
      uninstall: str(k.values.get('uninstallstring')),
      blocked: null,
    })
  }
  return out
}

const RANK = (a: InstalledApp): number => (a.uninstall ? 0 : 4) + (a.scope === 'machine' ? (a.arch === '64' ? 0 : 1) : 2)

export function dedupeApps(apps: InstalledApp[]): InstalledApp[] {
  const best = new Map<string, InstalledApp>()
  for (const a of apps) {
    const key = `${a.name.toLowerCase()}|${(a.version ?? '').toLowerCase()}|${(a.publisher ?? '').toLowerCase()}`
    const prev = best.get(key)
    if (!prev || RANK(a) < RANK(prev)) best.set(key, a)
  }
  return [...best.values()]
}

export function regExe(env: NodeJS.ProcessEnv): string {
  return path.win32.join(env.SystemRoot ?? env.SYSTEMROOT ?? 'C:\\Windows', 'System32', 'reg.exe')
}

function exportKey(exe: string, key: string, file: string): Promise<boolean> {
  return new Promise((resolve) => {
    execFile(exe, ['export', key, file, '/y'], { windowsHide: true, timeout: 30_000 }, (err) => resolve(!err))
  })
}

export async function readRegistryText(env: NodeJS.ProcessEnv = process.env): Promise<{ src: RegSource; text: string }[] | null> {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'orrery-apps-'))
  try {
    const exe = regExe(env)
    const out: { src: RegSource; text: string }[] = []
    await Promise.all(
      REG_SOURCES.map(async (src, i) => {
        const file = path.join(dir, `k${i}.reg`)
        if (!(await exportKey(exe, src.key, file))) return
        const buf = await fs.readFile(file).catch(() => null)
        if (buf) out.push({ src, text: decodeReg(buf) })
      }),
    )
    return out.length ? out : null
  } finally {
    await fs.rm(dir, { recursive: true, force: true }).catch(() => undefined)
  }
}

export function decodeReg(buf: Buffer): string {
  if (buf[0] === 0xff && buf[1] === 0xfe) return buf.subarray(2).toString('utf16le')
  return buf.toString('utf8')
}

export async function readFixtureText(dir: string): Promise<{ src: RegSource; text: string }[] | null> {
  const out: { src: RegSource; text: string }[] = []
  for (let i = 0; i < REG_SOURCES.length; i++) {
    const buf = await fs.readFile(path.join(dir, `k${i}.reg`)).catch(() => null)
    if (buf) out.push({ src: REG_SOURCES[i]!, text: decodeReg(buf) })
  }
  return out
}

export async function readWindowsApps(read: () => Promise<{ src: RegSource; text: string }[] | null>, env: NodeJS.ProcessEnv): Promise<InstalledApp[] | null> {
  const texts = await read()
  if (!texts) return null
  const all: InstalledApp[] = []
  for (const { src, text } of texts) all.push(...appsFromKeys(parseRegExport(text), src, env))
  return dedupeApps(all)
}
