import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { parseCleanDelete, parseCleanScan, type CleanProgress, type DiskHistoryEntry } from '@ccm/shared'
import { cacheSpots, realDir } from '../src/diskClean/caches'
import { findJunk, junkKind, nodeManager, projectTouched, restoreOf } from '../src/diskClean/projects'
import { cacheProvider, type CleanCtx } from '../src/diskClean/providers'
import { DiskCleanService, lockOf, type DiskCleanDeps } from '../src/diskClean/service'
import { createDiskClean } from '../src/diskClean/ipc'

let tmp: string
beforeEach(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-diskclean-')))
})
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true })
})

function put(rel: string, text = 'x'): string {
  const file = path.join(tmp, rel)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
  return file
}

const DAY = 86_400_000

function age(rel: string, days: number): void {
  const t = new Date(Date.now() - days * DAY)
  utimesSync(path.join(tmp, rel), t, t)
}

function link(target: string, at: string): void {
  symlinkSync(target, at, process.platform === 'win32' ? 'junction' : 'dir')
}

const rel = (p: string): string => path.relative(tmp, p).split(path.sep).join('/')

function deps(over: Partial<DiskCleanDeps> = {}): DiskCleanDeps & { events: CleanProgress[]; history: DiskHistoryEntry[]; trashed: string[] } {
  const events: CleanProgress[] = []
  const history: DiskHistoryEntry[] = []
  const trashed: string[] = []
  return {
    platform: process.platform,
    home: path.join(tmp, 'home'),
    env: {},
    self: () => [],
    running: () => [],
    keep: () => [],
    emit: (p) => events.push(p),
    confirm: async () => true,
    trash: async (p) => {
      trashed.push(p)
      rmSync(p, { recursive: true, force: true })
    },
    remove: async (p) => rmSync(p, { recursive: true, force: true }),
    record: async (e) => {
      history.push(e)
    },
    events,
    history,
    trashed,
    ...over,
  }
}

describe('junkKind', () => {
  const s = (...n: string[]) => new Set(n)
  it('needs a project marker next to the folder', () => {
    expect(junkKind('node_modules', s('package.json'))).toBe('node')
    expect(junkKind('node_modules', s())).toBeNull()
    expect(junkKind('build', s('build.gradle.kts'))).toBe('gradle')
    expect(junkKind('build', s('pubspec.yaml'))).toBe('flutter')
    expect(junkKind('build', s('readme.md'))).toBeNull()
    expect(junkKind('target', s('cargo.toml'))).toBe('rust')
    expect(junkKind('.dart_tool', s('pubspec.yaml'))).toBe('flutter')
    expect(junkKind('.dart_tool', s('package.json'))).toBeNull()
    expect(junkKind('.next', s('package.json'))).toBe('web')
    expect(junkKind('.pnpm-store', s('pnpm-lock.yaml', 'package.json'))).toBe('pnpm')
    expect(junkKind('dist', s('cargo.toml'))).toBeNull()
  })
})

describe('findJunk', () => {
  it('finds project junk, skips build/ without a marker, never walks into node_modules or links', async () => {
    put('web/package.json', '{}')
    put('web/node_modules/a/package.json', '{}')
    put('web/node_modules/a/node_modules/b/index.js')
    put('web/.next/x.js')
    put('docs/build/page.html')
    put('android/app/build.gradle', '')
    put('android/app/build/out.apk')
    put('mono/package.json', '{}')
    put('mono/packages/one/package.json', '{}')
    put('mono/packages/one/node_modules/x/i.js')
    put('mono/packages/one/dist/i.js')
    put('outside/package.json', '{}')
    put('outside/node_modules/z.js')
    link(path.join(tmp, 'outside'), path.join(tmp, 'web', 'linked'))
    const hits = (await findJunk(tmp)).map((h) => rel(h.path)).sort()
    expect(hits).toEqual(['android/app/build', 'mono/packages/one/dist', 'mono/packages/one/node_modules', 'outside/node_modules', 'web/.next', 'web/node_modules'])
  })

  it('handles Unicode paths', async () => {
    put('dự án/package.json', '{}')
    put('dự án/node_modules/a.js')
    const hits = await findJunk(tmp)
    expect(hits.map((h) => path.basename(h.owner))).toEqual(['dự án'])
  })
})

describe('project age and restore', () => {
  it('takes the newest marker or git file', async () => {
    put('p/package.json', '{}')
    put('p/.git/HEAD', 'ref')
    put('p/.git/index', 'i')
    age('p/package.json', 90)
    age('p/.git/HEAD', 60)
    age('p/.git/index', 45)
    const t = await projectTouched(path.join(tmp, 'p'), path.join(tmp, 'p'))
    expect(Math.round((Date.now() - t!) / DAY)).toBe(45)
  })

  it('reads git from the monorepo root above the package', async () => {
    put('m/.git/HEAD', 'ref')
    put('m/pkg/package.json', '{}')
    age('m/pkg/package.json', 100)
    age('m/.git/HEAD', 3)
    const t = await projectTouched(path.join(tmp, 'm', 'pkg'), path.join(tmp, 'm'))
    expect(Math.round((Date.now() - t!) / DAY)).toBe(3)
  })

  it('picks the restore command from the lockfile', async () => {
    put('a/package.json', '{}')
    put('a/pnpm-lock.yaml', '')
    put('b/package.json', '{}')
    put('b/yarn.lock', '')
    put('c/package.json', '{}')
    put('c/package-lock.json', '{}')
    put('d/package.json', '{}')
    put('m/pnpm-lock.yaml', '')
    put('m/pkg/package.json', '{}')
    expect(await nodeManager(path.join(tmp, 'a'), path.join(tmp, 'a'))).toBe('pnpm')
    expect(await restoreOf({ kind: 'node', owner: path.join(tmp, 'b'), root: path.join(tmp, 'b') })).toBe('yarn')
    expect(await restoreOf({ kind: 'node', owner: path.join(tmp, 'c'), root: path.join(tmp, 'c') })).toBe('npm install')
    expect(await restoreOf({ kind: 'node', owner: path.join(tmp, 'd'), root: path.join(tmp, 'd') })).toBe('npm install')
    expect(await restoreOf({ kind: 'node', owner: path.join(tmp, 'm', 'pkg'), root: path.join(tmp, 'm') })).toBe('pnpm install')
    expect(await restoreOf({ kind: 'flutter', owner: tmp, root: tmp })).toBe('flutter pub get')
    expect(await restoreOf({ kind: 'gradle', owner: tmp, root: tmp })).toBe('./gradlew build')
    expect(await restoreOf({ kind: 'rust', owner: tmp, root: tmp })).toBe('cargo build')
  })
})

describe('global caches', () => {
  it('lists the known spots per platform, honouring env overrides', () => {
    const win = cacheSpots({ home: 'C:\\Users\\u', platform: 'win32', env: { LOCALAPPDATA: 'C:\\Users\\u\\AppData\\Local', GRADLE_USER_HOME: 'K:\\dev\\.gradle' } })
    expect(win.find((s) => s.kind === 'gradle')?.path).toBe('K:\\dev\\.gradle\\caches')
    expect(win.find((s) => s.kind === 'npm')?.path).toBe('C:\\Users\\u\\AppData\\Local\\npm-cache\\_cacache')
    expect(win.some((s) => s.kind === 'cocoapods')).toBe(false)
    const mac = cacheSpots({ home: '/Users/u', platform: 'darwin', env: {} })
    expect(mac.find((s) => s.kind === 'cocoapods')?.path).toBe('/Users/u/Library/Caches/CocoaPods')
    expect(mac.find((s) => s.kind === 'pub')?.path).toBe('/Users/u/.pub-cache/hosted')
  })

  it('a Gradle home that is a link is measured and offered at its real folder', async () => {
    put('real-gradle/caches/modules-2/a.jar', '12345678')
    mkdirSync(path.join(tmp, 'home'))
    link(path.join(tmp, 'real-gradle'), path.join(tmp, 'home', '.gradle'))
    expect(await realDir(path.join(tmp, 'home', '.gradle', 'caches'))).toBe(path.join(tmp, 'real-gradle', 'caches'))
    const items: unknown[] = []
    const ctx: CleanCtx = {
      roots: [],
      home: path.join(tmp, 'home'),
      platform: process.platform,
      env: { LOCALAPPDATA: path.join(tmp, 'home', 'local') },
      signal: new AbortController().signal,
      junk: async () => [],
      size: async () => 8,
      emit: (b) => items.push(...b),
      keep: [],
      now: Date.now(),
      attach: () => undefined,
      keepMember: () => true,
    }
    const out = await cacheProvider.scan(ctx)
    expect(out.map((i) => i.path)).toEqual([path.join(tmp, 'real-gradle', 'caches')])
    expect(out[0]!.kind).toBe('gradle')
    expect(items).toHaveLength(1)
  })
})

describe('lockOf', () => {
  const p = process.platform
  it('locks items of a project with an open pane, Run or itself', () => {
    const it2 = { path: path.join(tmp, 'proj', 'node_modules'), project: path.join(tmp, 'proj') }
    expect(lockOf(it2, [path.join(tmp, 'proj', 'sub')], [], [], p)).toBe('open')
    expect(lockOf(it2, [path.join(tmp, 'other')], [], [], p)).toBeUndefined()
    expect(lockOf(it2, [tmp], [], [], p)).toBeUndefined()
    expect(lockOf(it2, [], [{ cwd: path.join(tmp, 'proj'), lock: 'run' }], [], p)).toBe('run')
    expect(lockOf(it2, [], [], [path.join(tmp, 'proj', 'node_modules', 'electron', 'dist')], p)).toBe('self')
    expect(lockOf(it2, [], [], [path.join(tmp, 'proj')], p)).toBe('self')
  })
})

describe('DiskCleanService', () => {
  function fixture(): void {
    put('old/package.json', '{}')
    put('old/package-lock.json', '{}')
    put('old/node_modules/a/index.js', 'aaaa')
    put('old/dist/out.js', 'bb')
    age('old/package.json', 60)
    age('old/package-lock.json', 60)
    put('busy/pubspec.yaml', '')
    put('busy/.dart_tool/x', 'ccc')
    put('busy/build/app.bin', 'dddd')
  }

  it('scans in category order, streams progress, locks open projects', async () => {
    fixture()
    const d = deps()
    const svc = new DiskCleanService(d)
    expect(svc.categories().map((c) => c.id)).toEqual(process.platform === 'darwin' ? ['node_modules', 'build', 'xcode', 'cache'] : ['node_modules', 'build', 'cache'])
    const r = await svc.scan({ ids: ['node_modules', 'build'], roots: [path.join(tmp, 'old'), path.join(tmp, 'busy'), path.join(tmp, 'busy', 'build'), 'relative'], open: [path.join(tmp, 'busy')] })
    const byRel = Object.fromEntries(r.items.map((i) => [rel(i.path), i]))
    expect(Object.keys(byRel).sort()).toEqual(['busy/.dart_tool', 'busy/build', 'old/dist', 'old/node_modules'])
    expect(byRel['old/node_modules']).toMatchObject({ category: 'node_modules', size: 4, restore: 'npm install', group: 'dev' })
    expect(byRel['old/node_modules']!.locked).toBeUndefined()
    expect(byRel['busy/build']!.locked).toBe('open')
    expect(Math.round((Date.now() - byRel['old/node_modules']!.lastTouched!) / DAY)).toBe(60)
    expect(d.events.map((e) => `${e.category}:${e.state}`)[0]).toBe('node_modules:start')
    expect(d.events.at(-1)).toMatchObject({ category: 'build', state: 'done', count: 3 })
  })

  it('a parent folder of many projects: the project is the git repo or the marker folder, so one open project locks only itself', async () => {
    fixture()
    put('mono/.git/HEAD', 'ref')
    put('mono/packages/a/package.json', '{}')
    put('mono/packages/a/node_modules/x.js')
    const d = deps()
    const svc = new DiskCleanService(d)
    const r = await svc.scan({ ids: ['node_modules', 'build'], roots: [tmp], open: [path.join(tmp, 'busy')] })
    const by = Object.fromEntries(r.items.map((i) => [rel(i.path), i]))
    expect(rel(by['mono/packages/a/node_modules']!.project!)).toBe('mono')
    expect(rel(by['old/node_modules']!.project!)).toBe('old')
    expect(by['old/node_modules']!.locked).toBeUndefined()
    expect(by['busy/build']!.locked).toBe('open')
    const r2 = await svc.scan({ ids: ['node_modules'], roots: [tmp], open: [path.join(tmp, 'mono', 'packages', 'b')] })
    expect(r2.items.find((i) => rel(i.path) === 'mono/packages/a/node_modules')!.locked).toBe('open')
  })

  it('deletes only scanned, unlocked folders and rechecks locks at delete time', async () => {
    fixture()
    const d = deps()
    const svc = new DiskCleanService(d)
    await svc.scan({ ids: ['node_modules', 'build'], roots: [path.join(tmp, 'old'), path.join(tmp, 'busy')], open: [] })
    const r = await svc.remove({
      paths: [path.join(tmp, 'old', 'node_modules'), path.join(tmp, 'busy', 'build'), path.join(tmp, 'old', 'package.json'), path.join(tmp, 'old')],
      permanent: false,
      open: [path.join(tmp, 'busy', 'lib')],
    })
    expect(r.done.map((x) => rel(x.path))).toEqual(['old/node_modules'])
    expect(r.freed).toBe(4)
    expect(r.failed.map((f) => `${rel(f.path)}:${f.reason}`).sort()).toEqual(['busy/build:locked:open', 'old/package.json:invalid', 'old:invalid'])
    expect(existsSync(path.join(tmp, 'old', 'node_modules'))).toBe(false)
    expect(existsSync(path.join(tmp, 'busy', 'build'))).toBe(true)
    expect(d.history).toHaveLength(1)
    expect(d.history[0]).toMatchObject({ action: 'trash', count: 1, freed: 4 })
    const again = await svc.remove({ paths: [path.join(tmp, 'old', 'node_modules')], permanent: false, open: [] })
    expect(again.failed[0]!.reason).toBe('invalid')
  })

  it('refuses a folder that became a link after the scan and never follows it', async () => {
    fixture()
    put('keep/precious.txt', 'p')
    const d = deps()
    const svc = new DiskCleanService(d)
    await svc.scan({ ids: ['build'], roots: [path.join(tmp, 'old')], open: [] })
    rmSync(path.join(tmp, 'old', 'dist'), { recursive: true })
    link(path.join(tmp, 'keep'), path.join(tmp, 'old', 'dist'))
    const r = await svc.remove({ paths: [path.join(tmp, 'old', 'dist')], permanent: true, open: [] })
    expect(r.failed[0]!.reason).toBe('changed')
    expect(existsSync(path.join(tmp, 'keep', 'precious.txt'))).toBe(true)
  })

  it('a cancelled confirm deletes nothing, a Run session found at delete time locks', async () => {
    fixture()
    const holders = [] as Array<{ cwd: string; lock: 'run' }>
    const d = deps({ confirm: async () => false, running: () => holders })
    const svc = new DiskCleanService(d)
    await svc.scan({ ids: ['node_modules', 'build'], roots: [path.join(tmp, 'old')], open: [] })
    const r = await svc.remove({ paths: [path.join(tmp, 'old', 'node_modules')], permanent: false, open: [] })
    expect(r.cancelled).toBe(true)
    expect(existsSync(path.join(tmp, 'old', 'node_modules'))).toBe(true)
    holders.push({ cwd: path.join(tmp, 'old'), lock: 'run' })
    const r2 = await svc.remove({ paths: [path.join(tmp, 'old', 'dist')], permanent: false, open: [] })
    expect(r2.failed[0]!.reason).toBe('locked:run')
  })

  it('cancel stops a scan', async () => {
    fixture()
    const d = deps()
    const svc = new DiskCleanService(d)
    const p = svc.scan({ ids: ['node_modules', 'build'], roots: [path.join(tmp, 'old')], open: [] })
    expect(svc.cancel()).toBe(true)
    expect((await p).cancelled).toBe(true)
    expect(svc.cancel()).toBe(false)
  })
})

describe('ipc', () => {
  it('validates requests and only answers the app frame', async () => {
    expect(parseCleanScan({ ids: ['build'], roots: ['C:\\a'], open: [] })).toEqual({ ids: ['build'], roots: ['C:\\a'], open: [] })
    expect(parseCleanScan({ ids: 'x' })).toBeNull()
    expect(parseCleanDelete({ paths: [], permanent: false })).toBeNull()
    expect(parseCleanDelete({ paths: ['a\u0000b'] })).toBeNull()
    const handlers = new Map<string, (e: unknown, a?: unknown) => Promise<unknown>>()
    const ipcMain = { handle: vi.fn((c: string, h: (e: unknown, a?: unknown) => Promise<unknown>) => handlers.set(c, h)), removeHandler: vi.fn() }
    const dc = createDiskClean({ ...deps(), ipcMain, isAppFrame: (e) => e === 'app', broadcast: () => undefined, confirm: async () => true })
    expect(await handlers.get('ccm:diskclean-categories')!('evil')).toEqual([])
    expect(((await handlers.get('ccm:diskclean-categories')!('app')) as unknown[]).length).toBeGreaterThan(1)
    expect(await handlers.get('ccm:diskclean-delete')!('app', { paths: 'x' })).toMatchObject({ cancelled: true })
    dc.dispose()
    expect(ipcMain.removeHandler).toHaveBeenCalledTimes(6)
  })
})
