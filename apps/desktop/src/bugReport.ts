import { execFile } from 'node:child_process'
import { createWriteStream, readdirSync, readFileSync, statSync, openSync, readSync, closeSync, fstatSync } from 'node:fs'
import path from 'node:path'
import { deflateRawSync } from 'node:zlib'
import { maskHookText } from './claudeHooks/model'

export const BUG_REPORT_LIMIT = 52_428_800
const MAX_ERRORS = 1_048_576
const MAX_AGE_MS = 30 * 86_400_000
const EVENTS_TIMEOUT_MS = 10_000
const PROCESS_NAMES = ['orrery.exe', 'orrery-core.exe', 'orrery-shell.exe']

export type BugKind = 'log' | 'crash' | 'dump' | 'report'

export type Body = { text: string } | { file: string; size: number }

export interface Candidate {
  name: string
  kind: BugKind
  body: Body
}

export interface Chosen {
  name: string
  bytes: number
  original: number
  cut: boolean
  text?: string
  file?: string
}

export interface Selection {
  entries: Chosen[]
  cut: string[]
  skipped: string[]
  skippedSizes: number[]
  total: number
}

const bodyLen = (b: Body): number => ('text' in b ? Buffer.byteLength(b.text) : b.size)

export function selectEntries(cands: Candidate[], limit: number): Selection {
  const sel: Selection = { entries: [], cut: [], skipped: [], skippedSizes: [], total: 0 }
  for (const c of cands) {
    const len = bodyLen(c.body)
    const left = limit - sel.total
    if ('file' in c.body) {
      if (len <= left) {
        sel.total += len
        sel.entries.push({ name: c.name, bytes: len, original: len, cut: false, file: c.body.file })
      } else {
        sel.skipped.push(c.name)
        sel.skippedSizes.push(len)
      }
      continue
    }
    if (len <= left) {
      sel.total += len
      sel.entries.push({ name: c.name, bytes: len, original: len, cut: false, text: c.body.text })
      continue
    }
    const buf = Buffer.from(c.body.text, 'utf8')
    let start = len - left
    while (start < buf.length && (buf[start]! & 0xc0) === 0x80) start++
    const kept = buf.subarray(start)
    if (kept.length === 0) {
      sel.skipped.push(c.name)
      sel.skippedSizes.push(len)
      continue
    }
    sel.total += kept.length
    sel.cut.push(c.name)
    sel.entries.push({ name: c.name, bytes: kept.length, original: len, cut: true, text: kept.toString('utf8') })
  }
  return sel
}

export function manifestText(sel: Selection, limit: number): string {
  const lines = [`Orrery bug report, budget ${limit} bytes uncompressed, used ${sel.total}`]
  for (const e of sel.entries) lines.push(e.cut ? `${e.name}\t${e.bytes}\tcut (kept last ${e.bytes} of ${e.original} bytes)` : `${e.name}\t${e.bytes}`)
  sel.skipped.forEach((n, i) => lines.push(`${n}\t${sel.skippedSizes[i]}\tskipped (over budget)`))
  return lines.join('\n') + '\n'
}

export function decodeText(buf: Buffer): string {
  if (buf.length >= 2 && buf[0] === 0xff && buf[1] === 0xfe) return buf.subarray(2).toString('utf16le')
  if (buf.length >= 3 && buf[0] === 0xef && buf[1] === 0xbb && buf[2] === 0xbf) return buf.subarray(3).toString('utf8')
  return buf.toString('utf8')
}

export function filterEvents(text: string): string {
  const blocks: string[] = []
  for (const line of text.split(/\r?\n/)) {
    if (line.startsWith('Event[') || blocks.length === 0) blocks.push('')
    blocks[blocks.length - 1] += line + '\n'
  }
  return blocks
    .filter((b) => {
      const low = b.toLowerCase()
      return PROCESS_NAMES.some((n) => low.includes(n))
    })
    .join('\n')
}

export const werFolderOk = (name: string): boolean => name.startsWith('AppCrash_Orrery') || name.startsWith('AppCrash_orrery')

export const dumpOk = (name: string): boolean => {
  const low = name.toLowerCase()
  return low.startsWith('orrery') && low.endsWith('.dmp')
}

export const macReportOk = (name: string): boolean => {
  const low = name.toLowerCase()
  return ['orrery', 'orrery-core', 'orrery-shell'].some((p) => low.startsWith(p)) && (low.endsWith('.ips') || low.endsWith('.crash'))
}

export interface Roots {
  logs: string
  home: string
  localAppData: string | undefined
  platform: NodeJS.Platform
}

const mtime = (p: string): number => {
  try {
    return statSync(p).mtimeMs
  } catch {
    return 0
  }
}

const recent = (p: string, now: number): boolean => {
  const t = mtime(p)
  return t > 0 && now - t <= MAX_AGE_MS
}

const listDir = (dir: string): string[] => {
  try {
    return readdirSync(dir).map((n) => path.join(dir, n))
  } catch {
    return []
  }
}

const isDir = (p: string): boolean => {
  try {
    return statSync(p).isDirectory()
  } catch {
    return false
  }
}

const isFile = (p: string): boolean => {
  try {
    return statSync(p).isFile()
  } catch {
    return false
  }
}

const byNewest = (a: string, b: string): number => mtime(b) - mtime(a)

function readTail(file: string, max: number): Buffer | null {
  let fd: number
  try {
    fd = openSync(file, 'r')
  } catch {
    return null
  }
  try {
    const len = fstatSync(fd).size
    const start = Math.max(0, len - max)
    const buf = Buffer.alloc(len - start)
    let off = 0
    while (off < buf.length) {
      const n = readSync(fd, buf, off, buf.length - off, start + off)
      if (n <= 0) break
      off += n
    }
    return buf.subarray(0, off)
  } catch {
    return null
  } finally {
    closeSync(fd)
  }
}

const mask = (text: string, roots: Roots): string => maskHookText(text, roots.home, roots.platform)

function readMasked(file: string, roots: Roots): string | null {
  const buf = readTail(file, BUG_REPORT_LIMIT)
  return buf ? mask(decodeText(buf), roots) : null
}

export function logCandidates(roots: Roots): Candidate[] {
  const out: Candidate[] = []
  for (const f of listDir(roots.logs).filter((p) => isFile(p) && p.toLowerCase().endsWith('.log')).sort(byNewest)) {
    const text = readMasked(f, roots)
    if (text !== null) out.push({ name: `logs/${path.basename(f)}`, kind: 'log', body: { text } })
  }
  return out
}

export function crashCandidates(roots: Roots, events: string | null, now: number): Candidate[] {
  const out: Candidate[] = []
  const ev = events ? mask(filterEvents(events), roots) : ''
  if (ev.trim()) out.push({ name: 'crashes/events.txt', kind: 'crash', body: { text: ev } })
  if (roots.localAppData) {
    for (const sub of ['ReportArchive', 'ReportQueue']) {
      const dirs = listDir(path.join(roots.localAppData, 'Microsoft', 'Windows', 'WER', sub))
        .filter((d) => isDir(d) && werFolderOk(path.basename(d)))
        .sort(byNewest)
      for (const d of dirs) {
        const wer = path.join(d, 'Report.wer')
        if (!recent(wer, now)) continue
        const text = readMasked(wer, roots)
        if (text !== null) out.push({ name: `crashes/${path.basename(d)}/Report.wer`, kind: 'crash', body: { text } })
      }
    }
  }
  if (roots.platform === 'darwin') {
    const files = listDir(path.join(roots.home, 'Library', 'Logs', 'DiagnosticReports'))
      .filter((p) => isFile(p) && macReportOk(path.basename(p)) && recent(p, now))
      .sort(byNewest)
    for (const p of files) {
      const text = readMasked(p, roots)
      if (text !== null) out.push({ name: `crashes/${path.basename(p)}`, kind: 'crash', body: { text } })
    }
  }
  return out
}

export function dumpCandidates(roots: Roots): Candidate[] {
  if (!roots.localAppData) return []
  return listDir(path.join(roots.localAppData, 'CrashDumps'))
    .filter((p) => isFile(p) && dumpOk(path.basename(p)))
    .sort(byNewest)
    .map((p) => ({ name: `dumps/${path.basename(p)}`, kind: 'dump' as const, body: { file: p, size: statSync(p).size } }))
}

export function assembleCandidates(report: string, errors: string, crashes: Candidate[], logs: Candidate[], dumps: Candidate[], roots: Roots): Candidate[] {
  const out: Candidate[] = [{ name: 'report.txt', kind: 'report', body: { text: report } }]
  const err = Array.from(errors).slice(0, MAX_ERRORS).join('')
  if (err.trim()) out.push({ name: 'page-errors.txt', kind: 'report', body: { text: mask(err, roots) } })
  return [...out, ...crashes, ...logs, ...dumps]
}

export function readEvents(): Promise<string | null> {
  if (process.platform !== 'win32') return Promise.resolve(null)
  const query = '*[System[(EventID=1000 or EventID=1001) and TimeCreated[timediff(@SystemTime) <= 2592000000]]]'
  return new Promise((resolve) => {
    execFile(
      'wevtutil',
      ['qe', 'Application', `/q:${query}`, '/f:text', '/rd:true', '/c:200'],
      { timeout: EVENTS_TIMEOUT_MS, windowsHide: true, encoding: 'buffer', maxBuffer: 64 * 1024 * 1024 },
      (err, stdout) => resolve(err && !stdout?.length ? null : decodeText(stdout)),
    )
  })
}

const CRC_TABLE = (() => {
  const t = new Uint32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[n] = c >>> 0
  }
  return t
})()

export function crc32(buf: Uint8Array): number {
  let c = 0xffffffff
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]!) & 0xff]! ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

export interface ZipItem {
  name: string
  data: Buffer
}

export function buildZip(items: ZipItem[], now = new Date()): Buffer {
  const time = (now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1)
  const date = (Math.max(0, now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate()
  const parts: Buffer[] = []
  const central: Buffer[] = []
  let offset = 0
  for (const it of items) {
    const name = Buffer.from(it.name, 'utf8')
    const raw = it.data
    const packed = deflateRawSync(raw)
    const stored = packed.length >= raw.length
    const body = stored ? raw : packed
    const method = stored ? 0 : 8
    const crc = crc32(raw)
    const local = Buffer.alloc(30)
    local.writeUInt32LE(0x04034b50, 0)
    local.writeUInt16LE(20, 4)
    local.writeUInt16LE(0x0800, 6)
    local.writeUInt16LE(method, 8)
    local.writeUInt16LE(time, 10)
    local.writeUInt16LE(date, 12)
    local.writeUInt32LE(crc, 14)
    local.writeUInt32LE(body.length, 18)
    local.writeUInt32LE(raw.length, 22)
    local.writeUInt16LE(name.length, 26)
    local.writeUInt16LE(0, 28)
    parts.push(local, name, body)
    const cd = Buffer.alloc(46)
    cd.writeUInt32LE(0x02014b50, 0)
    cd.writeUInt16LE(20, 4)
    cd.writeUInt16LE(20, 6)
    cd.writeUInt16LE(0x0800, 8)
    cd.writeUInt16LE(method, 10)
    cd.writeUInt16LE(time, 12)
    cd.writeUInt16LE(date, 14)
    cd.writeUInt32LE(crc, 16)
    cd.writeUInt32LE(body.length, 20)
    cd.writeUInt32LE(raw.length, 24)
    cd.writeUInt16LE(name.length, 28)
    cd.writeUInt32LE(offset, 42)
    central.push(cd, name)
    offset += 30 + name.length + body.length
  }
  const cdSize = central.reduce((n, b) => n + b.length, 0)
  const end = Buffer.alloc(22)
  end.writeUInt32LE(0x06054b50, 0)
  end.writeUInt16LE(items.length, 8)
  end.writeUInt16LE(items.length, 10)
  end.writeUInt32LE(cdSize, 12)
  end.writeUInt32LE(offset, 16)
  return Buffer.concat([...parts, ...central, end])
}

export function zipItems(sel: Selection): ZipItem[] {
  const items: ZipItem[] = sel.entries.map((e) => ({ name: e.name, data: e.file !== undefined ? readFileSync(e.file) : Buffer.from(e.text ?? '', 'utf8') }))
  items.push({ name: 'manifest.txt', data: Buffer.from(manifestText(sel, BUG_REPORT_LIMIT), 'utf8') })
  return items
}

export function writeZipFile(file: string, items: ZipItem[]): Promise<number> {
  const zip = buildZip(items)
  return new Promise((resolve, reject) => {
    const out = createWriteStream(file)
    out.on('error', reject)
    out.on('finish', () => resolve(zip.length))
    out.end(zip)
  })
}

export function bugReportRoots(logs: string, home: string, platform: NodeJS.Platform, localAppData: string | undefined): Roots {
  return { logs, home, localAppData, platform }
}

export async function gatherCandidates(roots: Roots, report: string, errors: string, dumps: boolean): Promise<Candidate[]> {
  const crashes = crashCandidates(roots, await readEvents(), Date.now())
  return assembleCandidates(report, errors, crashes, logCandidates(roots), dumps ? dumpCandidates(roots) : [], roots)
}

export const previewItems = (cands: Candidate[]): { name: string; bytes: number; kind: BugKind }[] => cands.map((c) => ({ name: c.name, bytes: bodyLen(c.body), kind: c.kind }))

export const clampErrors = (v: unknown): string => (typeof v === 'string' ? Array.from(v).slice(0, MAX_ERRORS).join('') : '')
