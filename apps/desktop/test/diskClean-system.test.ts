import { existsSync, mkdirSync, mkdtempSync, readdirSync, realpathSync, renameSync, rmSync, symlinkSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { CLEAN_ONE_TAP, parseCleanDelete, type CleanItem, type DiskHistoryEntry } from '@ccm/shared'
import { cacheProvider, type CleanCtx } from '../src/diskClean/providers'
import { findJunk, restoreOf, type GitTracked } from '../src/diskClean/projects'
import { DiskCleanService, type DiskCleanDeps, type OneTapChoice } from '../src/diskClean/service'
import { binBoxOptions, oneTapBoxOptions, oneTapChoice } from '../src/diskClean/dialog'
import {
  appCacheLock,
  browserSpots,
  dirBin,
  installerName,
  isForbidden,
  parseBinQuery,
  parseLsappinfo,
  parseTasklistNames,
  sysRoots,
  SYSTEM_PROVIDERS,
  type SystemDeps,
} from '../src/diskClean/system'

const win = process.platform === 'win32'
const DAY = 86_400_000
let tmp: string
let base: string

beforeEach(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-sysclean-')))
  base = path.join(tmp, 'fake')
})
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true })
})

function put(abs: string, n = 10, ageMs = 0): string {
  mkdirSync(path.dirname(abs), { recursive: true })
  writeFileSync(abs, Buffer.alloc(n, 1))
  if (ageMs) {
    const t = new Date(Date.now() - ageMs)
    utimesSync(abs, t, t)
  }
  return abs
}

function old(abs: string, ageMs: number): void {
  const t = new Date(Date.now() - ageMs)
  utimesSync(abs, t, t)
}

interface Fx {
  sys: SystemDeps
  procs: string[]
  busy: Set<string>
  removed: string[]
  trashed: string[]
  history: DiskHistoryEntry[]
  deps: (over?: Partial<DiskCleanDeps>) => DiskCleanDeps
}

function fixture(): Fx {
  const roots = sysRoots({ platform: process.platform, home: '', env: {}, fake: base })
  mkdirSync(roots.bin!, { recursive: true })
  const fx: Fx = {
    procs: [],
    busy: new Set(),
    removed: [],
    trashed: [],
    history: [],
    sys: null as unknown as SystemDeps,
    deps: (over = {}) => ({
      platform: process.platform,
      home: roots.home,
      env: {},
      system: fx.sys,
      self: () => [],
      running: () => [],
      keep: () => [],
      emit: () => undefined,
      confirm: async () => true,
      askOneTap: async () => 'clean',
      askBin: async () => true,
      trash: async (p) => {
        fx.trashed.push(p)
        rmSync(p, { recursive: true, force: true })
      },
      remove: async (p) => {
        fx.removed.push(p)
        rmSync(p, { recursive: true, force: true })
      },
      record: async (e) => {
        fx.history.push(e)
      },
      ...over,
    }),
  }
  fx.sys = { roots, procs: async () => fx.procs, bin: dirBin(roots.bin!), busy: async (f) => fx.busy.has(f) }
  return fx
}

const SYS_IDS = ['temp', 'browser', 'crash', 'logs', 'thumbs', 'installers', 'bin', 'update']
const byCat = (items: CleanItem[], c: string): CleanItem[] => items.filter((i) => i.category === c)

describe.runIf(win)('system providers (Windows layout, fake roots)', () => {
  it('Temp offers only entries older than 24 hours and skips locked files', async () => {
    const fx = fixture()
    const T = fx.sys.roots.temp
    const oldFile = put(path.join(T, 'old.log'), 100, 2 * DAY)
    const fresh = put(path.join(T, 'new.log'), 100)
    const locked = put(path.join(T, 'locked.tmp'), 100, 3 * DAY)
    const uni = put(path.join(T, 'tệp cũ ✓.txt'), 50, 5 * DAY)
    put(path.join(T, 'olddir', 'a.bin'), 200, 3 * DAY)
    old(path.join(T, 'olddir'), 3 * DAY)
    put(path.join(T, 'livedir', 'old.bin'), 200, 3 * DAY)
    put(path.join(T, 'livedir', 'new.bin'), 200)
    fx.busy.add(locked)
    const svc = new DiskCleanService(fx.deps())
    const res = await svc.scan({ ids: ['temp'], roots: [], open: [] })
    const item = byCat(res.items, 'temp').find((i) => i.path === T)!
    expect(item.action).toBe('files')
    const names = svc.membersOf(T).map((m) => path.basename(m.path)).sort()
    expect(names).toEqual(['old.log', 'olddir', 'tệp cũ ✓.txt'].sort())
    expect(item.size).toBe(100 + 50 + 200)
    expect(item.files).toBe(3)

    const del = await svc.remove({ paths: [T], permanent: true, open: [] })
    expect(fx.removed.slice().sort()).toEqual([oldFile, uni, path.join(T, 'olddir')].sort())
    expect(existsSync(fresh) && existsSync(locked) && existsSync(path.join(T, 'livedir'))).toBe(true)
    expect(existsSync(T)).toBe(true)
    expect(del.done).toEqual([{ path: T, size: 350 }])
    expect(fx.history[0]?.action).toBe('delete')
  })

  it('a Temp file changed or locked after the scan is skipped, not deleted', async () => {
    const fx = fixture()
    const T = fx.sys.roots.temp
    const a = put(path.join(T, 'a.log'), 10, 2 * DAY)
    const b = put(path.join(T, 'b.log'), 10, 2 * DAY)
    const c = put(path.join(T, 'c.log'), 10, 2 * DAY)
    const svc = new DiskCleanService(fx.deps())
    await svc.scan({ ids: ['temp'], roots: [], open: [] })
    writeFileSync(a, 'touched')
    fx.busy.add(b)
    const res = await svc.remove({ paths: [T], permanent: true, open: [] })
    expect(existsSync(a) && existsSync(b)).toBe(true)
    expect(fx.removed).toEqual([c])
    expect(res.failed).toEqual([{ path: T, reason: 'skipped:2' }])
  })

  it('a Temp entry under the running app is never offered', async () => {
    const fx = fixture()
    const T = fx.sys.roots.temp
    put(path.join(T, 'portable-orrery', 'app.asar'), 10, 3 * DAY)
    old(path.join(T, 'portable-orrery'), 3 * DAY)
    put(path.join(T, 'other.log'), 10, 3 * DAY)
    const svc = new DiskCleanService(fx.deps({ self: () => [path.join(T, 'portable-orrery')] }))
    await svc.scan({ ids: ['temp'], roots: [], open: [] })
    expect(svc.membersOf(T).map((m) => path.basename(m.path))).toEqual(['other.log'])
  })

  it('browser caches: only Cache / Code Cache / GPUCache, locked while the browser runs, rechecked at delete', async () => {
    const fx = fixture()
    const ud = path.join(fx.sys.roots.local, 'Google', 'Chrome', 'User Data')
    for (const p of ['Default', 'Profile 1']) {
      put(path.join(ud, p, 'Cache', 'Cache_Data', 'f_0001'), 500)
      put(path.join(ud, p, 'Code Cache', 'js', 'x'), 100)
      put(path.join(ud, p, 'Cookies'), 50)
      put(path.join(ud, p, 'History'), 50)
    }
    put(path.join(ud, 'Default', 'GPUCache', 'data_0'), 70)
    put(path.join(fx.sys.roots.local, 'Mozilla', 'Firefox', 'Profiles', 'abc.default', 'cache2', 'entries', 'e1'), 300)
    fx.procs = ['chrome.exe']
    const svc = new DiskCleanService(fx.deps())
    const items = byCat((await svc.scan({ ids: ['browser'], roots: [], open: [] })).items, 'browser')
    const base = (i: CleanItem) => path.basename(i.path)
    expect(items.map(base).sort()).toEqual(['Cache', 'Cache', 'Code Cache', 'Code Cache', 'GPUCache', 'cache2'])
    expect(items.filter((i) => i.kind === 'chrome').every((i) => i.locked === 'browser')).toBe(true)
    expect(items.find((i) => i.kind === 'firefox')?.locked).toBeUndefined()
    const refused = await svc.remove({ paths: [path.join(ud, 'Default', 'Cache')], permanent: true, open: [] })
    expect(refused.failed[0]?.reason).toBe('locked:browser')

    fx.procs = []
    await svc.scan({ ids: ['browser'], roots: [], open: [] })
    fx.procs = ['firefox.exe', 'chrome.exe']
    const ff = path.join(fx.sys.roots.local, 'Mozilla', 'Firefox', 'Profiles', 'abc.default', 'cache2')
    const late = await svc.remove({ paths: [ff, path.join(ud, 'Default', 'Cache')], permanent: true, open: [] })
    expect(late.failed.map((f) => f.reason)).toEqual(['locked:browser', 'locked:browser'])
    expect(existsSync(ff)).toBe(true)
    expect(existsSync(path.join(ud, 'Default', 'Cookies'))).toBe(true)
  })

  it('admin places show size with the admin lock and can never be deleted', async () => {
    const fx = fixture()
    const W = fx.sys.roots.windir
    put(path.join(W, 'Temp', 'x.tmp'), 40, 3 * DAY)
    put(path.join(W, 'Minidump', 'a.dmp'), 60)
    put(path.join(W, 'SoftwareDistribution', 'Download', 'kb', 'u.cab'), 80)
    put(path.join(fx.sys.roots.programData, 'Microsoft', 'Windows', 'WER', 'ReportArchive', 'r', 'Report.wer'), 30)
    const svc = new DiskCleanService(fx.deps())
    const items = (await svc.scan({ ids: ['temp', 'crash', 'update', 'logs'], roots: [], open: [] })).items
    const admin = items.filter((i) => i.locked === 'admin')
    expect(admin.map((i) => i.kind).sort()).toEqual(['minidump', 'winTemp', 'winUpdate', 'werArchiveAll'].sort())
    expect(admin.find((i) => i.kind === 'winUpdate')?.size).toBe(80)
    const res = await svc.remove({ paths: admin.map((i) => i.path), permanent: true, open: [] })
    expect(res.failed.every((f) => f.reason === 'locked:admin')).toBe(true)
    expect(existsSync(path.join(W, 'Minidump', 'a.dmp'))).toBe(true)
    expect(fx.removed).toEqual([])
  })

  it('forbidden places never leak, even when an env root points at them', async () => {
    const fx = fixture()
    const r = fx.sys.roots
    const sys32 = path.join(r.windir, 'System32')
    put(path.join(sys32, 'drivers', 'etc.sys'), 10, 9 * DAY)
    const docs = path.join(r.home, 'Documents')
    put(path.join(docs, 'thesis.docx'), 10, 9 * DAY)
    put(path.join(r.home, '.ssh', 'id_ed25519'), 10, 9 * DAY)
    for (const temp of [sys32, docs, r.home, path.join(r.home, '.ssh')]) {
      const svc = new DiskCleanService(fx.deps({ system: { ...fx.sys, roots: { ...r, temp, downloads: temp, local: temp } } }))
      const items = (await svc.scan({ ids: SYS_IDS, roots: [], open: [] })).items
      const leaked = items.filter((i) => !i.locked && i.action !== 'bin').map((i) => i.path)
      expect(leaked).toEqual([])
      expect(svc.membersOf(temp)).toEqual([])
    }
    expect(isForbidden('C:\\Windows\\System32\\x.dll', r)).toBe(true)
    expect(isForbidden('C:\\Program Files\\App\\cache', r)).toBe(true)
    expect(isForbidden(path.join(r.home, 'Pictures', 'a.png'), r)).toBe(true)
    expect(isForbidden(path.dirname(r.home), r)).toBe(true)
    expect(isForbidden(path.join(r.temp, 'x'), r)).toBe(false)
  })

  it('installers in Downloads: older than 30 days, never ticked, never in one tap', async () => {
    const fx = fixture()
    const D = fx.sys.roots.downloads
    const setup = put(path.join(D, 'Setup-2.1.exe'), 900, 40 * DAY)
    put(path.join(D, 'fresh.msi'), 900, 2 * DAY)
    put(path.join(D, 'photos.zip'), 900, 60 * DAY)
    put(path.join(D, 'tool-installer.zip'), 900, 60 * DAY)
    put(path.join(D, 'notes.txt'), 9, 60 * DAY)
    const svc = new DiskCleanService(fx.deps())
    expect(svc.categories().find((c) => c.id === 'installers')?.preselect).toBe(false)
    const items = byCat((await svc.scan({ ids: ['installers'], roots: [], open: [] })).items, 'installers')
    expect(items.map((i) => path.basename(i.path)).sort()).toEqual(['Setup-2.1.exe', 'tool-installer.zip'])
    expect(items.every((i) => i.file)).toBe(true)
    const tap = await svc.remove({ paths: [setup], permanent: false, open: [], oneTap: true })
    expect(tap.failed[0]?.reason).toBe('invalid')
    expect(existsSync(setup)).toBe(true)
    const res = await svc.remove({ paths: [setup], permanent: false, open: [] })
    expect(res.done.length).toBe(1)
    expect(fx.trashed).toEqual([setup])
  })

  it('crash dumps, WER logs and thumbnail cache (locked thumbcache skipped)', async () => {
    const fx = fixture()
    const L = fx.sys.roots.local
    put(path.join(L, 'CrashDumps', 'app.exe.123.dmp'), 300)
    put(path.join(L, 'Microsoft', 'Windows', 'WER', 'ReportArchive', 'AppCrash_x', 'Report.wer'), 20)
    const ex = path.join(L, 'Microsoft', 'Windows', 'Explorer')
    put(path.join(ex, 'thumbcache_256.db'), 400)
    const held = put(path.join(ex, 'thumbcache_idx.db'), 40)
    put(path.join(ex, 'iconcache_16.db'), 40)
    fx.busy.add(held)
    const svc = new DiskCleanService(fx.deps())
    const items = (await svc.scan({ ids: ['crash', 'logs', 'thumbs'], roots: [], open: [] })).items
    expect(byCat(items, 'crash')[0]?.size).toBe(300)
    expect(byCat(items, 'logs').find((i) => i.kind === 'werArchive')?.size).toBe(20)
    expect(svc.membersOf(ex).map((m) => path.basename(m.path))).toEqual(['thumbcache_256.db'])
  })

  it('Recycle Bin: size only, not selectable, emptied after confirm', async () => {
    const fx = fixture()
    const bin = fx.sys.roots.bin!
    put(path.join(bin, '$R1.txt'), 120)
    put(path.join(bin, '$R2', 'x'), 30)
    let asked = 0
    const svc = new DiskCleanService(fx.deps({ askBin: async () => (++asked === 1 ? false : true) }))
    const item = byCat((await svc.scan({ ids: ['bin'], roots: [], open: [] })).items, 'bin')[0]!
    expect(item).toMatchObject({ action: 'bin', size: 150, files: 2 })
    const viaDelete = await svc.remove({ paths: [item.path], permanent: true, open: [] })
    expect(viaDelete.failed[0]?.reason).toBe('invalid')
    expect((await svc.emptyBin()).cancelled).toBe(true)
    expect(readdirSync(bin).length).toBe(2)
    expect(await svc.emptyBin()).toEqual({ done: true, freed: 150 })
    expect(readdirSync(bin)).toEqual([])
    expect(fx.history.at(-1)).toMatchObject({ action: 'delete', freed: 150 })
  })

  it('empty or unreadable Recycle Bin is shown as unknown and locked', async () => {
    const fx = fixture()
    const svc = new DiskCleanService(fx.deps({ system: { ...fx.sys, bin: { size: async () => null, empty: async () => undefined } } }))
    const item = byCat((await svc.scan({ ids: ['bin'], roots: [], open: [] })).items, 'bin')[0]!
    expect(item).toMatchObject({ locked: 'denied', unknown: true })
  })

  it('one tap: Temp and browser cache deleted for good, dev junk to the bin; "trash" sends all to the bin; cancel keeps all', async () => {
    const fx = fixture()
    const T = fx.sys.roots.temp
    const ud = path.join(fx.sys.roots.local, 'Microsoft', 'Edge', 'User Data')
    const run = async (choice: OneTapChoice) => {
      put(path.join(T, 'a.log'), 10, 2 * DAY)
      put(path.join(ud, 'Default', 'Cache', 'd'), 20)
      const proj = path.join(tmp, 'proj')
      put(path.join(proj, 'package.json'), 2, 90 * DAY)
      put(path.join(proj, 'node_modules', 'x.js'), 30)
      fx.removed.length = 0
      fx.trashed.length = 0
      let asked: unknown = null
      const svc = new DiskCleanService(fx.deps({ askOneTap: async (a) => ((asked = a), choice) }))
      const items = (await svc.scan({ ids: ['temp', 'browser', 'node_modules'], roots: [tmp], open: [] })).items
      const res = await svc.remove({ paths: items.map((i) => i.path), permanent: false, open: [], oneTap: true })
      return { res, asked, proj }
    }
    const a = await run('clean')
    expect(a.asked).toMatchObject({ permCount: 2, trashCount: 1, permBytes: 30, trashBytes: 30 })
    expect(fx.removed.map((p) => path.basename(p)).sort()).toEqual(['Cache', 'a.log'])
    expect(fx.trashed.map((p) => path.basename(p))).toEqual(['node_modules'])
    expect(a.res.freed).toBe(60)
    expect(a.res.permanent).toBe(true)
    await run('trash')
    expect(fx.removed).toEqual([])
    expect(fx.trashed.length).toBe(3)
    const c = await run('cancel')
    expect(c.res.cancelled).toBe(true)
    expect(existsSync(path.join(c.proj, 'node_modules'))).toBe(true)
  })
})

describe.runIf(win)('delete path hardening', () => {
  function project(name: string): string {
    const proj = path.join(tmp, name)
    put(path.join(proj, 'package.json'), 2, 90 * DAY)
    put(path.join(proj, 'node_modules', 'x.js'), 30)
    return proj
  }

  it('main builds its own open list: a pane open in the project locks it even if the page sends none', async () => {
    const fx = fixture()
    const proj = project('p1')
    const svc = new DiskCleanService(fx.deps({ system: undefined, openNow: async () => [proj] }))
    const items = (await svc.scan({ ids: ['node_modules'], roots: [tmp], open: [] })).items
    expect(items[0]?.locked).toBe('open')
    let open: string[] = []
    const svc2 = new DiskCleanService(fx.deps({ system: undefined, openNow: async () => open }))
    await svc2.scan({ ids: ['node_modules'], roots: [tmp], open: [] })
    const res = await svc2.remove({ paths: [path.join(proj, 'node_modules')], permanent: true, open: [] })
    expect(res.done.length).toBe(1)
    project('p2')
    await svc2.scan({ ids: ['node_modules'], roots: [tmp], open: [] })
    const target = path.join(tmp, 'p2', 'node_modules')
    const late = await new DiskCleanService(
      fx.deps({
        system: undefined,
        openNow: async () => open,
        confirm: async () => {
          open = [path.join(tmp, 'p2')]
          return true
        },
      }),
    )
    await late.scan({ ids: ['node_modules'], roots: [tmp], open: [] })
    open = []
    const r2 = await late.remove({ paths: [target], permanent: true, open: [] })
    expect(r2.failed[0]?.reason).toBe('locked:open')
    expect(existsSync(target)).toBe(true)
  })

  it('a parent swapped for a junction while the dialog is open is refused', async () => {
    const fx = fixture()
    const proj = project('p3')
    const elsewhere = path.join(tmp, 'elsewhere')
    put(path.join(elsewhere, 'node_modules', 'precious.txt'), 5)
    const svc = new DiskCleanService(
      fx.deps({
        system: undefined,
        confirm: async (_a, second) => {
          if (second) return true
          renameSync(proj, path.join(tmp, 'p3-moved'))
          symlinkSync(elsewhere, proj, 'junction')
          return true
        },
      }),
    )
    await svc.scan({ ids: ['node_modules'], roots: [tmp], open: [] })
    const res = await svc.remove({ paths: [path.join(proj, 'node_modules')], permanent: true, open: [] })
    expect(res.done).toEqual([])
    expect(res.failed[0]?.reason).toBe('changed')
    expect(existsSync(path.join(elsewhere, 'node_modules', 'precious.txt'))).toBe(true)
  })

  it('a cache env var that resolves to home or a keep folder is not offered', async () => {
    const home = path.join(tmp, 'home')
    const keep = path.join(home, 'Documents')
    put(path.join(keep, 'caches', 'f'), 5)
    mkdirSync(path.join(tmp, 'npm'), { recursive: true })
    symlinkSync(home, path.join(tmp, 'npm', '_cacache'), 'junction')
    const items: CleanItem[] = []
    const ctx = {
      roots: [],
      home,
      platform: process.platform,
      env: { GRADLE_USER_HOME: keep, npm_config_cache: path.join(tmp, 'npm') },
      signal: new AbortController().signal,
      junk: async () => [],
      size: async () => 1,
      emit: (b: CleanItem[]) => items.push(...b),
      keep: [keep],
      now: Date.now(),
      attach: () => undefined,
      keepMember: () => true,
    } satisfies CleanCtx
    await cacheProvider.scan(ctx)
    expect(items.filter((i) => i.kind === 'gradle' || i.kind === 'npm')).toEqual([])
  })
})

describe('build folders that belong to the source tree', () => {
  const p = (...s: string[]) => path.join(tmp, ...s)

  it('skips build/ of an electron-builder project and a tracked dist/', async () => {
    put(p('app', 'package.json'), 0)
    writeFileSync(p('app', 'package.json'), JSON.stringify({ devDependencies: { 'electron-builder': '^25' } }))
    put(p('app', 'build', 'icon.icns'), 5)
    put(p('app', 'dist', 'main.js'), 5)
    put(p('res', 'package.json'), 0)
    writeFileSync(p('res', 'package.json'), JSON.stringify({ build: { directories: { buildResources: 'out' } } }))
    put(p('res', 'out', 'entitlements.plist'), 5)
    put(p('res', 'build', 'bundle.js'), 5)
    put(p('repo', '.git', 'HEAD'), 3)
    put(p('repo', 'package.json'), 2)
    put(p('repo', 'dist', 'lib.js'), 5)
    put(p('repo', 'build', 'out.js'), 5)
    const tracked: GitTracked = async (dir, name) => dir.endsWith('repo') && name === 'dist'
    const hits = (await findJunk(tmp, undefined, 6, tracked)).map((h) => path.relative(tmp, h.path).split(path.sep).join('/')).sort()
    expect(hits).toEqual(['app/dist', 'repo/build', 'res/build'])
  })

  it('unknown git answer inside a repo keeps the folder out', async () => {
    put(p('r', '.git', 'HEAD'), 3)
    put(p('r', 'package.json'), 2)
    put(p('r', 'dist', 'a.js'), 5)
    put(p('r', 'node_modules', 'a.js'), 5)
    const hits = (await findJunk(tmp, undefined, 6, async () => null)).map((h) => path.basename(h.path))
    expect(hits).toEqual(['node_modules'])
  })

  it('.venv restore hint installs the requirements', async () => {
    put(p('py', 'requirements.txt'), 2)
    expect(await restoreOf({ kind: 'python', owner: p('py'), root: tmp })).toBe('python -m venv .venv && pip install -r requirements.txt')
  })
})

describe('helpers', () => {
  it('parses shell answers', () => {
    expect(parseBinQuery('0 1048576 12\r\n')).toEqual({ bytes: 1048576, count: 12 })
    expect(parseBinQuery('-2147418113 0 0')).toBeNull()
    expect(parseBinQuery(null)).toBeNull()
    expect(parseTasklistNames('"chrome.exe","123","Console","1","100 K"\r\n"msedge.exe","9"')).toEqual(['chrome.exe', 'msedge.exe'])
    expect(parseLsappinfo('2) "Google Chrome" ASN:0x0\n    bundleID="com.google.Chrome"')).toEqual(['com.google.Chrome', 'Google Chrome'])
  })

  it('installer names per OS', () => {
    expect(installerName('Setup.EXE', 'win32')).toBe(true)
    expect(installerName('app.msi', 'win32')).toBe(true)
    expect(installerName('pics.zip', 'win32')).toBe(false)
    expect(installerName('app.dmg', 'darwin')).toBe(true)
    expect(installerName('app.exe', 'darwin')).toBe(false)
  })

  it('mac browser and app cache rules', () => {
    const r = sysRoots({ platform: 'darwin', home: '/Users/me', env: { TMPDIR: '/var/folders/ab/xyz/T/' } })
    const chrome = browserSpots(r).find((b) => b.id === 'chrome')!
    expect(chrome.bases).toEqual(['/Users/me/Library/Caches/Google/Chrome', '/Users/me/Library/Application Support/Google/Chrome'])
    expect(appCacheLock('com.spotify.client', new Set(['com.spotify.client']))).toBe(true)
    expect(appCacheLock('com.tinyspeck.slackmacgap', new Set(['finder']))).toBe(false)
    expect(isForbidden('/Users/me/Library/Keychains/login.keychain-db', r)).toBe(true)
    expect(isForbidden('/Library/Caches/x', r)).toBe(true)
    expect(isForbidden('/var/folders/ab/xyz/T/old', r)).toBe(false)
  })

  it('one tap list and dialogs', () => {
    expect(CLEAN_ONE_TAP).not.toContain('installers')
    expect(CLEAN_ONE_TAP).not.toContain('cache')
    expect(CLEAN_ONE_TAP).not.toContain('bin')
    expect(parseCleanDelete({ paths: ['C:\\a'], oneTap: true })?.oneTap).toBe(true)
    const ask = { bytes: 3e9, count: 3, permBytes: 2e9, permCount: 2, trashBytes: 1e9, trashCount: 1 }
    const box = oneTapBoxOptions(ask, 'vi', false)
    expect(box.buttons).toHaveLength(3)
    expect(String(box.detail)).toContain('xoá hẳn')
    expect(oneTapChoice(0, ask)).toBe('clean')
    expect(oneTapChoice(1, ask)).toBe('trash')
    expect(oneTapChoice(2, ask)).toBe('cancel')
    expect(oneTapChoice(1, { ...ask, permCount: 0 })).toBe('cancel')
    expect(binBoxOptions({ bytes: 10, count: 1 }, 'en', true).message).toContain('Trash')
    for (const t of [box.message, box.detail, ...(box.buttons ?? [])]) expect(String(t)).not.toMatch(/[—·]|trần/)
  })

  it('system providers are only added when the system deps exist', () => {
    const plain = new DiskCleanService({ platform: 'win32', home: 'C:\\Users\\x', env: {}, self: () => [], running: () => [], keep: () => [], emit: () => undefined, confirm: async () => true, trash: async () => undefined, remove: async () => undefined, record: async () => undefined })
    expect(plain.categories().some((c) => c.group === 'system')).toBe(false)
    expect(SYSTEM_PROVIDERS.every((p) => p.group === 'system')).toBe(true)
  })
})
