import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { inflateRawSync } from 'node:zlib'
import { afterAll, describe, expect, it } from 'vitest'
import {
  localFileStamp,
  assembleCandidates,
  buildZip,
  crashCandidates,
  crc32,
  decodeText,
  dumpCandidates,
  dumpOk,
  filterEvents,
  logCandidates,
  macReportOk,
  manifestText,
  selectEntries,
  werFolderOk,
  zipItems,
  type Candidate,
  type Roots,
} from '../src/bugReport'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-bug-')))
afterAll(() => rmSync(tmp, { recursive: true, force: true }))

const text = (name: string, body: string): Candidate => ({ name, kind: 'log', body: { text: body } })

function readZip(buf: Buffer): Map<string, Buffer> {
  const eocd = buf.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]))
  const count = buf.readUInt16LE(eocd + 10)
  let p = buf.readUInt32LE(eocd + 16)
  const out = new Map<string, Buffer>()
  for (let i = 0; i < count; i++) {
    expect(buf.readUInt32LE(p)).toBe(0x02014b50)
    const method = buf.readUInt16LE(p + 10)
    const crc = buf.readUInt32LE(p + 16)
    const csize = buf.readUInt32LE(p + 20)
    const usize = buf.readUInt32LE(p + 24)
    const nlen = buf.readUInt16LE(p + 28)
    const elen = buf.readUInt16LE(p + 30)
    const clen = buf.readUInt16LE(p + 32)
    const off = buf.readUInt32LE(p + 42)
    const name = buf.subarray(p + 46, p + 46 + nlen).toString('utf8')
    const lnlen = buf.readUInt16LE(off + 26)
    const lelen = buf.readUInt16LE(off + 28)
    const start = off + 30 + lnlen + lelen
    const raw = buf.subarray(start, start + csize)
    const data = method === 8 ? inflateRawSync(raw) : raw
    expect(data.length).toBe(usize)
    expect(crc32(data)).toBe(crc)
    out.set(name, Buffer.from(data))
    p += 46 + nlen + elen + clen
  }
  return out
}

describe('bug report selection', () => {
  it('keeps everything under the budget', () => {
    const sel = selectEntries([text('report.txt', 'abcd'), text('logs/a.log', 'xyz')], 100)
    expect(sel.total).toBe(7)
    expect(sel.cut).toEqual([])
    expect(sel.skipped).toEqual([])
    expect(manifestText(sel, 100)).toContain('logs/a.log\t3\n')
  })

  it('cuts a text to its last bytes and skips what no longer fits', () => {
    const sel = selectEntries([text('report.txt', 'abcd'), text('logs/a.log', '0123456789'), text('logs/b.log', 'zz')], 10)
    expect(sel.cut).toEqual(['logs/a.log'])
    expect(sel.skipped).toEqual(['logs/b.log'])
    expect(sel.entries[1]!.text).toBe('456789')
    expect(sel.total).toBe(10)
    const m = manifestText(sel, 10)
    expect(m).toContain('logs/a.log\t6\tcut (kept last 6 of 10 bytes)')
    expect(m).toContain('logs/b.log\t2\tskipped')
  })

  it('never splits a character', () => {
    expect(selectEntries([text('logs/a.log', 'aéé')], 3).entries[0]!.text).toBe('é')
  })

  it('skips a dump that does not fit', () => {
    const sel = selectEntries([text('report.txt', 'abcd'), { name: 'dumps/Orrery.exe.1.dmp', kind: 'dump', body: { file: 'x', size: 50 } }], 20)
    expect(sel.skipped).toEqual(['dumps/Orrery.exe.1.dmp'])
    expect(sel.entries).toHaveLength(1)
  })
})

describe('bug report sources', () => {
  it('filters names', () => {
    expect(werFolderOk('AppCrash_Orrery.exe_abc') && werFolderOk('AppCrash_orrery-core.exe_1') && !werFolderOk('AppCrash_chrome.exe_1')).toBe(true)
    expect(dumpOk('Orrery.exe.1234.dmp') && dumpOk('orrery-core.exe.9.dmp') && !dumpOk('chrome.exe.1.dmp') && !dumpOk('Orrery.txt')).toBe(true)
    expect(macReportOk('Orrery-2026.ips') && macReportOk('orrery-core_1.crash') && macReportOk('orrery-shell-1.ips') && !macReportOk('Safari.ips') && !macReportOk('Orrery.txt')).toBe(true)
  })

  it('keeps only Orrery event blocks', () => {
    const raw = 'Event[0]:\n  Application: Chrome.exe\n\nEvent[1]:\n  Faulting application name: ORRERY.EXE, version 4\n\nEvent[2]:\n  orrery-core.exe crashed\n'
    const out = filterEvents(raw)
    expect(out).toContain('ORRERY.EXE')
    expect(out).toContain('orrery-core.exe')
    expect(out).not.toContain('Chrome')
    expect(filterEvents('nothing here\n')).toBe('')
  })

  it('decodes UTF-16 and BOM text', () => {
    expect(decodeText(Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from('Sig=ok é', 'utf16le')]))).toBe('Sig=ok é')
    expect(decodeText(Buffer.from([0xef, 0xbb, 0xbf, 0x68, 0x69]))).toBe('hi')
  })

  it('collects, masks, orders and zips', () => {
    const base = path.join(tmp, 'a')
    const logs = path.join(base, 'logs')
    const local = path.join(base, 'local')
    const home = path.join(base, 'home')
    const wer = path.join(local, 'Microsoft', 'Windows', 'WER', 'ReportArchive', 'AppCrash_Orrery.exe_1')
    const other = path.join(local, 'Microsoft', 'Windows', 'WER', 'ReportArchive', 'AppCrash_Other_2')
    for (const d of [logs, wer, other, path.join(local, 'CrashDumps'), home]) mkdirSync(d, { recursive: true })
    writeFileSync(path.join(logs, 'shell.log'), `start ${home}/p\nAPI_KEY=abcdef123456\n`)
    writeFileSync(path.join(logs, 'notes.txt'), 'no')
    writeFileSync(path.join(wer, 'Report.wer'), 'EventType=APPCRASH\n')
    writeFileSync(path.join(other, 'Report.wer'), 'other')
    writeFileSync(path.join(local, 'CrashDumps', 'Orrery.exe.1234.dmp'), Buffer.alloc(64, 7))
    writeFileSync(path.join(local, 'CrashDumps', 'chrome.exe.1.dmp'), Buffer.alloc(8, 7))
    const roots: Roots = { logs, home, localAppData: local, platform: 'win32' }
    const crashes = crashCandidates(roots, 'Event[0]:\n orrery-shell.exe fault\n', Date.now())
    expect(crashes.map((c) => c.name)).toEqual(['crashes/events.txt', 'crashes/AppCrash_Orrery.exe_1/Report.wer'])
    const all = assembleCandidates('REPORT', 'err at X', crashes, logCandidates(roots), dumpCandidates(roots), roots)
    expect(all.map((c) => c.name)).toEqual(['report.txt', 'page-errors.txt', 'crashes/events.txt', 'crashes/AppCrash_Orrery.exe_1/Report.wer', 'logs/shell.log', 'dumps/Orrery.exe.1234.dmp'])
    const sel = selectEntries(all, 52_428_800)
    const files = readZip(buildZip(zipItems(sel)))
    expect([...files.keys()]).toHaveLength(7)
    const log = files.get('logs/shell.log')!.toString('utf8')
    expect(log).toContain('start ~')
    expect(log).not.toContain('abcdef123456')
    expect(files.get('dumps/Orrery.exe.1234.dmp')!.length).toBe(64)
    expect(files.get('manifest.txt')!.toString('utf8')).toContain('report.txt\t6')
  })

  it('leaves out old WER reports', () => {
    const base = path.join(tmp, 'b')
    const local = path.join(base, 'local')
    const wer = path.join(local, 'Microsoft', 'Windows', 'WER', 'ReportQueue', 'AppCrash_orrery-core.exe_9')
    mkdirSync(wer, { recursive: true })
    writeFileSync(path.join(wer, 'Report.wer'), 'x')
    const roots: Roots = { logs: path.join(base, 'none'), home: base, localAppData: local, platform: 'win32' }
    expect(crashCandidates(roots, null, Date.now() + 40 * 86_400_000)).toEqual([])
    expect(crashCandidates(roots, null, Date.now())).toHaveLength(1)
  })
})

describe('zip writer', () => {
  it('round-trips compressed, stored and empty entries', () => {
    const big = Buffer.from('hello world\n'.repeat(500))
    const noise = Buffer.from(Array.from({ length: 64 }, (_, i) => (i * 97 + 13) & 0xff))
    const files = readZip(buildZip([{ name: 'a/é.txt', data: big }, { name: 'b.bin', data: noise }, { name: 'c.txt', data: Buffer.alloc(0) }]))
    expect(files.get('a/é.txt')!.equals(big)).toBe(true)
    expect(files.get('b.bin')!.equals(noise)).toBe(true)
    expect(files.get('c.txt')!.length).toBe(0)
  })
})

describe('localFileStamp', () => {
  it('uses the local date and minute', () => {
    expect(localFileStamp(new Date(2026, 10, 11, 15, 46, 59))).toBe('20261111-1546')
    expect(localFileStamp(new Date(2026, 0, 2, 3, 4))).toBe('20260102-0304')
  })
})
