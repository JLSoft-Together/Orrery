import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { rawItems, scanJunk, skippedPath, type RawEntry } from '../src/organize/junk'

let root: string

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-organize-junk-'))
})

afterEach(() => {
  fs.rmSync(root, { recursive: true, force: true })
})

const dirEntry = (p: string, count = 0, extra: Partial<RawEntry> = {}): RawEntry => ({ path: p, name: path.basename(p), dir: true, count, ...extra })

describe('junk', () => {
  it('lists empty folders that are really empty and skips system, app, dot and protected folders', async () => {
    const empty = path.join(root, 'empty')
    const nowFull = path.join(root, 'now-full')
    const git = path.join(root, 'repo', '.git', 'refs')
    const nm = path.join(root, 'node_modules', 'x')
    const keep = path.join(root, 'Documents')
    for (const d of [empty, nowFull, git, nm, keep]) fs.mkdirSync(d, { recursive: true })
    fs.writeFileSync(path.join(nowFull, 'f.txt'), 'x')
    const searches: Array<Record<string, unknown>> = []
    const res = await scanJunk({
      platform: process.platform,
      skipTree: [path.join(root, 'app')],
      allowTree: [os.tmpdir()],
      guard: async () => ({ platform: process.platform, home: os.homedir(), keep: [keep], roots: [root] }),
      search: async (q) => {
        searches.push(q)
        if (q.only === 'dir') return [dirEntry(empty), dirEntry(nowFull), dirEntry(git), dirEntry(nm), dirEntry(keep), dirEntry(path.join(root, 'app', 'x')), dirEntry(path.join(root, 'linked'), 0, { link: true }), dirEntry(path.join(root, 'has'), 2)]
        return []
      },
      readShortcut: () => null,
    })
    expect(res.indexed).toBe(true)
    expect(res.items.map((i) => i.path)).toEqual([empty])
    expect(res.items[0]).toMatchObject({ kind: 'empty', state: 'empty' })
    expect(searches[0]).toMatchObject({ only: 'dir', maxSize: 0 })
  })

  it('checks Windows shortcuts: missing target is broken, network or absent drive is unknown, live target is hidden', async () => {
    const live = path.join(root, 'live.txt')
    fs.writeFileSync(live, 'x')
    const lnk = (name: string): RawEntry => ({ path: path.join(root, name), name, dir: false })
    const targets: Record<string, string> = {
      'broken.lnk': path.join(root, 'gone.exe'),
      'live.lnk': live,
      'net.lnk': '\\\\server\\share\\a.exe',
      'usb.lnk': 'Q:\\tool.exe',
    }
    const res = await scanJunk({
      platform: 'win32',
      skipTree: [],
      allowTree: [os.tmpdir()],
      guard: async () => ({ platform: 'win32', home: 'C:\\Users\\nobody', keep: [], roots: [root] }),
      search: async (q) => (q.only === 'file' ? Object.keys(targets).map(lnk).concat([lnk('readme.txt')]) : []),
      readShortcut: (p) => targets[path.basename(p)] ?? null,
      volumes: async () => [{ mount: `${path.parse(root).root}`, network: false }],
      stat: async (p) => (p.startsWith('Q:') || p.startsWith('\\\\') ? Promise.reject(Object.assign(new Error('x'), { code: 'ENOENT' })) : fs.promises.stat(p)),
    })
    if (process.platform !== 'win32') return
    const byName = Object.fromEntries(res.items.map((i) => [i.name, i.state]))
    expect(byName).toEqual({ 'broken.lnk': 'broken', 'net.lnk': 'unknown', 'usb.lnk': 'unknown' })
  })

  it('returns not indexed without a scanned index', async () => {
    expect(await scanJunk({ platform: 'win32', skipTree: [], guard: async () => null, search: async () => [] })).toEqual({ indexed: false, items: [] })
    expect(rawItems({ items: [{ path: 'C:\\a', name: 'a', dir: true, count: 0 }, { bad: 1 }] })).toHaveLength(1)
    expect(rawItems(null)).toBeNull()
  })

  it('skips dot, build and app folders unless an allowed tree holds them', () => {
    expect(skippedPath('C:\\Users\\me\\AppData\\Local\\X\\e', 'win32', [])).toBe(true)
    expect(skippedPath('C:\\Users\\me\\AppData\\Local\\Temp\\e', 'win32', [], ['C:\\Users\\me\\AppData\\Local\\Temp'])).toBe(false)
    expect(skippedPath('D:\\work\\proj\\.git\\x', 'win32', [])).toBe(true)
    expect(skippedPath('D:\\work\\proj\\node_modules\\x', 'win32', [])).toBe(true)
    expect(skippedPath('/Users/me/Library/Caches/x', 'darwin', [])).toBe(true)
    expect(skippedPath('D:\\work\\empty', 'win32', [])).toBe(false)
  })
})
