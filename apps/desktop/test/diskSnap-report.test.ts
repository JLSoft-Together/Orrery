import { describe, expect, it } from 'vitest'
import { buildReport, composeReport, DiskReporter, fmtBytes, FREE_DROP_BYTES, largestDirs, shortPath } from '../src/diskMap/report'
import { buildCompared, type Snapshot } from '../src/diskMap/snapshots'

const GB = 1024 ** 3
const HOME = 'C:\\Users\\dauph'

const mk = (at: number, free: number, dirs: [string, number][]): Snapshot => ({
  v: 1,
  at,
  roots: [{ path: 'C:\\', size: 100 * GB, dirs }],
  volumes: [{ mount: 'C:\\', label: 'OS', free: free * GB, total: 500 * GB }],
})

const before = mk(Date.UTC(2026, 10, 14, 12), 100, [
  ['C:\\Users', 50 * GB],
  ['C:\\Users\\dauph', 48 * GB],
  ['C:\\Users\\dauph\\AppData', 20 * GB],
  ['C:\\Users\\dauph\\AppData\\Local', 19 * GB],
  ['C:\\Users\\dauph\\Downloads', 10 * GB],
  ['C:\\Users\\dauph\\Videos', 8 * GB],
  ['C:\\Windows', 20 * GB],
  ['C:\\Program Files', 15 * GB],
  ['C:\\Games', 4 * GB],
  ['C:\\Temp', 1 * GB],
])
const after = mk(Date.UTC(2026, 10, 15, 12), 80, [
  ['C:\\Users', 70 * GB],
  ['C:\\Users\\dauph', 68 * GB],
  ['C:\\Users\\dauph\\AppData', 20 * GB],
  ['C:\\Users\\dauph\\AppData\\Local', 19 * GB],
  ['C:\\Users\\dauph\\Downloads', 30 * GB],
  ['C:\\Users\\dauph\\Videos', 8 * GB],
  ['C:\\Windows', 20 * GB],
  ['C:\\Program Files', 15 * GB],
  ['C:\\Games', 4 * GB],
  ['C:\\Temp', 1 * GB],
])

describe('shortPath', () => {
  it('shortens the home folder to ~ and long paths to first ... last two', () => {
    expect(shortPath('C:\\Users\\dauph', HOME, 'win32')).toBe('~')
    expect(shortPath('C:\\Users\\dauph\\Downloads', HOME, 'win32')).toBe('~\\Downloads')
    expect(shortPath('c:\\users\\DAUPH\\Downloads\\x', HOME, 'win32')).toBe('~\\Downloads\\x')
    expect(shortPath('C:\\Users\\dauph\\a\\b\\c\\d\\e', HOME, 'win32')).toBe('~\\...\\d\\e')
    expect(shortPath('D:\\Games\\Steam\\steamapps\\common\\Big', HOME, 'win32')).toBe('D:\\...\\common\\Big')
    expect(shortPath('D:\\Games', HOME, 'win32')).toBe('D:\\Games')
    expect(shortPath('/Users/dauph/Library/Caches', '/Users/dauph', 'darwin')).toBe('~/Library/Caches')
    expect(shortPath('/var/lib/docker/overlay2/x/y', '/home/me', 'linux')).toBe('/var/.../x/y')
  })

  it('does not treat a sibling folder as the home folder', () => {
    expect(shortPath('C:\\Users\\dauph2\\x', HOME, 'win32')).toBe('C:\\Users\\dauph2\\x')
  })
})

describe('fmtBytes', () => {
  it('formats with units', () => {
    expect(fmtBytes(0, 'en')).toBe('0 B')
    expect(fmtBytes(1536, 'en')).toBe('1.5 KB')
    expect(fmtBytes(20 * GB, 'en')).toBe('20 GB')
  })
})

describe('composeReport', () => {
  it('is null when free space fell by 5 GB or less', () => {
    expect(composeReport(before, mk(2, 95, []), 'bgscan', 'vi', 'win32', HOME)).toBeNull()
    expect(composeReport(before, mk(2, 100 - 5, []), 'bgscan', 'vi', 'win32', HOME)).toBeNull()
    expect(composeReport(before, mk(2, 120, []), 'bgscan', 'vi', 'win32', HOME)).toBeNull()
  })

  it('lists the drive, the top growers and the largest folders without file names', () => {
    const text = composeReport(before, after, 'bgscan', 'vi', 'win32', HOME)!
    expect(text).toContain('Quét nền ổ đĩa xong')
    expect(text).toContain('Ổ C: còn trống 80 GB / 500 GB, giảm 20 GB')
    expect(text).toContain('80 GB')
    expect(text).toContain('giảm 20 GB')
    expect(text).toContain('Phình nhiều nhất:')
    expect(text).toContain('1. ~\\Downloads  +20 GB (30 GB)')
    expect(text).toContain('Lớn nhất:')
    expect(text).not.toContain('C:\\Users\\dauph')
    const lines = text.split('\n')
    expect(lines.filter((l) => /^\d+\. /.test(l)).length).toBeLessThanOrEqual(10)
    expect(text).not.toMatch(/\.(txt|pdf|zip|exe|mp4)\b/i)
  })

  it('uses at most five of each list', () => {
    const many: [string, number][] = []
    for (let i = 0; i < 12; i++) many.push([`C:\\d${i}`, (i + 1) * GB])
    const grown: [string, number][] = many.map(([p, s]) => [p, s + 3 * GB])
    const text = composeReport(mk(1, 100, many), mk(2, 50, grown), 'alert', 'en', 'win32', HOME)!
    expect(text).toContain('Disk almost full')
    expect(text.split('\n').filter((l) => /^\d+\. /.test(l))).toHaveLength(10)
    expect(text).toContain('Grew the most:')
    expect(text).toContain('Largest:')
  })

  it('largest folders are disjoint subtrees', () => {
    const c = buildCompared(after, after, 'win32')
    const out = largestDirs(c, 'win32', 5).map((o) => o.path)
    expect(out[0]).toBe('C:\\Users\\dauph')
    expect(out).not.toContain('C:\\Users\\dauph\\Downloads')
    for (const a of out) for (const b of out) if (a !== b) expect(b.toLowerCase().startsWith(`${a.toLowerCase()}\\`)).toBe(false)
  })

  it('buildReport prints locale text', () => {
    const t = buildReport({ trigger: 'alert', locale: 'en', platform: 'win32', home: HOME, since: Date.UTC(2026, 10, 14, 12), drops: [{ label: 'C:', free: 80 * GB, total: 500 * GB, drop: 20 * GB }], growth: [], largest: [] })
    expect(t).toBe(`Disk almost full\nDrive C: has 80 GB free of 500 GB, down 20 GB since the scan on 14/11/2026.`)
  })
})

describe('DiskReporter', () => {
  const rig = (enabled = true) => {
    const sent: string[] = []
    const clock = { now: 1_800_000_000_000 }
    const r = new DiskReporter({ platform: 'win32', home: HOME, locale: () => 'vi', enabled: () => enabled, send: async (t) => (sent.push(t), true), now: () => clock.now })
    return { r, sent, clock }
  }

  it('sends once when free space fell more than 5 GB', async () => {
    const { r, sent } = rig()
    expect(await r.report(before, after, 'bgscan')).toBe(true)
    expect(sent).toHaveLength(1)
    expect(FREE_DROP_BYTES).toBe(5 * GB)
  })

  it('sends nothing when the switch is off', async () => {
    const { r, sent } = rig(false)
    expect(await r.report(before, after, 'bgscan')).toBe(false)
    expect(sent).toEqual([])
  })

  it('sends nothing without a baseline, a current snapshot or a big drop', async () => {
    const { r, sent } = rig()
    expect(await r.report(null, after, 'bgscan')).toBe(false)
    expect(await r.report(before, null, 'bgscan')).toBe(false)
    expect(await r.report(before, mk(2, 99, []), 'bgscan')).toBe(false)
    expect(sent).toEqual([])
  })

  it('does not repeat a disk alert within 6 hours, but a scan report is not throttled', async () => {
    const { r, sent, clock } = rig()
    expect(await r.report(before, after, 'alert')).toBe(true)
    expect(await r.report(before, after, 'alert')).toBe(false)
    expect(await r.report(before, after, 'bgscan')).toBe(true)
    clock.now += 6 * 3600_000 + 1
    expect(await r.report(before, after, 'alert')).toBe(true)
    expect(sent).toHaveLength(3)
  })

  it('a failed send does not start the alert gap and never throws', async () => {
    let n = 0
    const r = new DiskReporter({ platform: 'win32', home: HOME, locale: () => 'en', enabled: () => true, send: async () => (n++ === 0 ? Promise.reject(new Error('x')) : true) })
    expect(await r.report(before, after, 'alert')).toBe(false)
    expect(await r.report(before, after, 'alert')).toBe(true)
  })
})
