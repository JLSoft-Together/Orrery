import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { ORGANIZE_KINDS } from '@ccm/shared'
import { classify, cleanPatch, cleanRule, folderNames, monthOf, withSuffix } from '../src/organize/rules'
import { buildPlan } from '../src/organize/plan'
import { moveFile, realMoveIo, type MoveIo } from '../src/organize/move'
import { parseRun } from '../src/organize/log'
import { OrganizeService, type OrganizeDeps } from '../src/organize/service'
import { e2ePath, e2eStableMs } from '../src/organize/ipc'

const FUTURE = 10 * 60_000
const STABLE = 120_000
const errno = (code: string): NodeJS.ErrnoException => Object.assign(new Error(code), { code })

let base: string
let src: string
let data: string

const put = (rel: string, size = 10, mtime?: number): string => {
  const f = path.join(src, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(size, 1))
  if (mtime !== undefined) fs.utimesSync(f, new Date(mtime), new Date(mtime))
  return f
}

function service(over: Partial<OrganizeDeps> = {}): OrganizeService {
  return new OrganizeService({
    dataDir: data,
    platform: process.platform,
    home: os.homedir(),
    downloads: src,
    desktop: '',
    appDirs: [data],
    locale: () => 'vi',
    stableMs: STABLE,
    now: () => Date.now() + FUTURE,
    setTimer: () => null,
    clearTimer: () => undefined,
    watch: () => ({ close: () => undefined }),
    ...over,
  })
}

beforeEach(() => {
  base = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-organize-test-'))
  src = path.join(base, 'Downloads')
  data = path.join(base, 'data')
  fs.mkdirSync(src)
})

afterEach(() => {
  fs.rmSync(base, { recursive: true, force: true })
})

describe('rules', () => {
  it('classifies by type, month and custom rules in order', () => {
    const opts = { mode: 'type' as const, kinds: [...ORGANIZE_KINDS], rules: [], locale: 'vi' }
    expect(classify('a.JPG', 1, 0, opts)?.folder).toBe('Ảnh')
    expect(classify('setup.exe', 1, 0, opts)?.folder).toBe('Cài đặt')
    expect(classify('noext', 1, 0, opts)?.folder).toBe('Khác')
    expect(classify('a.pdf', 1, 0, { ...opts, locale: 'en' })?.folder).toBe('Documents')
    const t = new Date(2024, 2, 15).getTime()
    expect(classify('a.pdf', 1, t, { ...opts, mode: 'month' })?.folder).toBe('2024-03')
    expect(monthOf(new Date(2025, 11, 31).getTime())).toBe('2025-12')
    const rules = [cleanRule({ kind: 'name', value: 'Hoá đơn', folder: 'Hoá đơn' })!, cleanRule({ kind: 'ext', value: '.PDF', folder: 'PDF' })!, cleanRule({ kind: 'size', value: '1', folder: 'Lớn' })!]
    expect(rules[1]!.value).toBe('pdf')
    expect(classify('hoá đơn tháng 3.pdf', 1, 0, { ...opts, rules })?.folder).toBe('Hoá đơn')
    expect(classify('x.pdf', 1, 0, { ...opts, rules })).toMatchObject({ folder: 'PDF', rule: 1 })
    expect(classify('x.mp4', 2 * 1024 * 1024, 0, { ...opts, rules })?.folder).toBe('Lớn')
    expect(classify('x.mp4', 10, 0, { ...opts, kinds: ['image'] })).toBeNull()
    expect(folderNames('en').other).toBe('Other')
  })

  it('rejects folders that could leave the source', () => {
    for (const bad of ['..', '.', 'a/b', 'a\\b', 'C:', 'x.', 'con', 'NUL.txt', '', 'a'.repeat(65), 'a*b']) {
      expect(cleanRule({ kind: 'ext', value: 'pdf', folder: bad })).toBeNull()
    }
    expect(cleanRule({ kind: 'size', value: '-1', folder: 'Big' })).toBeNull()
    expect(cleanRule({ kind: 'ext', value: 'a b', folder: 'X' })).toBeNull()
    expect(cleanPatch({ mode: 'bogus', kinds: ['image', 'nope'], auto: 'yes', rules: [{ kind: 'ext', value: 'pdf', folder: '../x' }] })).toEqual({ kinds: ['image'], rules: [] })
    expect(withSuffix('a.tar.gz', 2)).toBe('a.tar (2).gz')
    expect(withSuffix('README', 3)).toBe('README (3)')
  })
})

describe('plan', () => {
  it('skips partial, hidden, recent and nested files, suffixes clashes and keeps Unicode names', async () => {
    const old = Date.now() - 3600_000
    put('a.jpg', 10, old)
    put('Ảnh/a.jpg', 5, old)
    put('Ảnh/a (2).jpg', 5, old)
    put('ảnh chụp màn hình.png', 7, old)
    put('b.pdf', 10, old)
    put('c.crdownload', 10, old)
    put('movie.mkv', 10, old)
    put('movie.mkv.part', 10, old)
    put('.hidden', 1, old)
    put('desktop.ini', 1, old)
    put('~$report.docx', 1, old)
    put('sub/deep.jpg', 1, old)
    put('new.png', 1, Date.now() + FUTURE)
    const r = await buildPlan(src, { mode: 'type', kinds: [...ORGANIZE_KINDS], rules: [], locale: 'vi', platform: process.platform, now: Date.now() + FUTURE, stableMs: STABLE })
    const moves = Object.fromEntries(r.moves.map((m) => [m.name, `${m.folder}/${m.target}`]))
    expect(moves).toEqual({ 'a.jpg': 'Ảnh/a (3).jpg', 'ảnh chụp màn hình.png': 'Ảnh/ảnh chụp màn hình.png', 'b.pdf': 'Tài liệu/b.pdf' })
    const skipped = Object.fromEntries(r.skipped.map((s) => [s.name, s.reason]))
    expect(skipped['c.crdownload']).toBe('downloading')
    expect(skipped['movie.mkv']).toBe('downloading')
    expect(skipped['movie.mkv.part']).toBe('downloading')
    expect(skipped['.hidden']).toBe('hidden')
    expect(skipped['desktop.ini']).toBe('hidden')
    expect(skipped['~$report.docx']).toBe('hidden')
    expect(skipped['new.png']).toBe('recent')
    expect(r.waitUntil).not.toBeNull()
    expect(skipped['deep.jpg']).toBeUndefined()
  })

  it('suffixes two files of the plan that land on the same name and blocks a file standing in for a folder', async () => {
    const old = Date.now() - 3600_000
    put('x.pdf', 1, old)
    put('y.PDF', 1, old)
    put('Khác', 1, old)
    const rules = [cleanRule({ kind: 'name', value: 'x', folder: 'Same' })!, cleanRule({ kind: 'name', value: 'y', folder: 'Same' })!]
    const r = await buildPlan(src, { mode: 'type', kinds: [...ORGANIZE_KINDS], rules, locale: 'vi', platform: process.platform, now: Date.now() + FUTURE, stableMs: STABLE })
    expect(r.moves.map((m) => m.target).sort()).toEqual(['x.pdf', 'y.PDF'])
    expect(r.skipped.find((s) => s.name === 'Khác')?.reason).toBe('blocked')
  })
})

describe('move', () => {
  it('copies, verifies and removes the source across volumes', async () => {
    const from = put('a.bin', 100, Date.now() - 3600_000)
    const to = path.join(src, 'b.bin')
    const io: MoveIo = { ...realMoveIo, rename: async () => Promise.reject(errno('EXDEV')) }
    expect(await moveFile(from, to, io)).toBe('copy')
    expect(fs.existsSync(from)).toBe(false)
    expect(fs.statSync(to).size).toBe(100)
  })

  it('drops the copy when the size does not verify and never overwrites', async () => {
    const from = put('a.bin', 100)
    const to = path.join(src, 'b.bin')
    const io: MoveIo = {
      ...realMoveIo,
      rename: async () => Promise.reject(errno('EXDEV')),
      copyFile: async (a, b) => fs.promises.writeFile(b, Buffer.alloc(3)),
    }
    await expect(moveFile(from, to, io)).rejects.toMatchObject({ code: 'verify' })
    expect(fs.existsSync(from)).toBe(true)
    expect(fs.existsSync(to)).toBe(false)
    put('b.bin', 1)
    await expect(moveFile(from, to)).rejects.toMatchObject({ code: 'EEXIST' })
    expect(fs.statSync(to).size).toBe(1)
  })
})

describe('service', () => {
  const old = (): number => Date.now() - 3600_000

  it('plans, moves inside the source, logs and undoes', async () => {
    put('a.jpg', 10, old())
    put('b.pdf', 20, old())
    put('c.zip', 30, old())
    const svc = service()
    const plan = await svc.plan({})
    expect(plan.error).toBeUndefined()
    expect(plan.items.map((i) => i.name).sort()).toEqual(['a.jpg', 'b.pdf', 'c.zip'])
    const skip = plan.items.find((i) => i.name === 'c.zip')!.id
    const res = await svc.run({ plan: plan.id, exclude: [skip] })
    expect(res.moved).toBe(2)
    expect(res.bytes).toBe(30)
    expect(fs.existsSync(path.join(src, 'Ảnh', 'a.jpg'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'Tài liệu', 'b.pdf'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'c.zip'))).toBe(true)
    const rec = parseRun(fs.readFileSync(path.join(data, 'organize', `${res.run}.jsonl`), 'utf8'))!
    expect(rec.moves.filter((m) => m.done)).toHaveLength(2)
    expect(rec.dirs).toHaveLength(2)
    expect((await svc.run({ plan: plan.id })).error).toBe('expired')
    const st = await svc.state()
    expect(st.runs[0]).toMatchObject({ id: res.run, moved: 2, bytes: 30, undone: false })
    const u = await svc.undo({ run: res.run })
    expect(u).toMatchObject({ restored: 2, conflicts: [], dirs: 2 })
    expect(fs.existsSync(path.join(src, 'a.jpg'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'Ảnh'))).toBe(false)
    expect((await svc.undo({ run: res.run })).error).toBe('done')
    expect((await svc.state()).runs[0]?.undone).toBe(true)
  })

  it('undo skips files renamed, changed or replaced since and keeps folders that still hold files', async () => {
    for (const n of ['a.jpg', 'b.jpg', 'c.jpg', 'd.jpg']) put(n, 10, old())
    put('Ảnh/keep.jpg', 1, old())
    const svc = service()
    const plan = await svc.plan({})
    const res = await svc.run({ plan: plan.id })
    expect(res.moved).toBe(4)
    fs.renameSync(path.join(src, 'Ảnh', 'a.jpg'), path.join(src, 'Ảnh', 'renamed.jpg'))
    fs.appendFileSync(path.join(src, 'Ảnh', 'b.jpg'), 'more')
    put('c.jpg', 1)
    const u = await svc.undo({ run: res.run })
    expect(u.restored).toBe(1)
    expect(Object.fromEntries(u.conflicts.map((c) => [c.name, c.reason]))).toEqual({ 'a.jpg': 'missing', 'b.jpg': 'changed', 'c.jpg': 'occupied' })
    expect(fs.existsSync(path.join(src, 'd.jpg'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'Ảnh'))).toBe(true)
    expect(fs.statSync(path.join(src, 'c.jpg')).size).toBe(1)
  })

  it('undo recovers a move whose log line was cut by a power loss and never a move that did not happen', async () => {
    put('a.jpg', 10, old())
    put('b.jpg', 10, old())
    fs.mkdirSync(path.join(src, 'Ảnh'))
    fs.renameSync(path.join(src, 'a.jpg'), path.join(src, 'Ảnh', 'a.jpg'))
    const id = 'abc123-0011aabb'
    const lines = [
      { t: 'run', id, at: Date.now(), source: fs.realpathSync(src), mode: 'type', auto: false },
      { t: 'dir', path: path.join(fs.realpathSync(src), 'Ảnh') },
      { t: 'intent', from: path.join(fs.realpathSync(src), 'a.jpg'), to: path.join(fs.realpathSync(src), 'Ảnh', 'a.jpg') },
      { t: 'intent', from: path.join(fs.realpathSync(src), 'b.jpg'), to: path.join(fs.realpathSync(src), 'Ảnh', 'b.jpg') },
    ]
    fs.mkdirSync(path.join(data, 'organize'), { recursive: true })
    fs.writeFileSync(path.join(data, 'organize', `${id}.jsonl`), lines.map((l) => JSON.stringify(l)).join('\n') + '\n')
    const svc = service()
    expect((await svc.state()).runs.map((r) => r.id)).toContain(id)
    const u = await svc.undo({ run: id })
    expect(u).toMatchObject({ restored: 1, conflicts: [], dirs: 1 })
    expect(fs.existsSync(path.join(src, 'a.jpg'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'b.jpg'))).toBe(true)
  })

  it('reports locked files and keeps them in place', async () => {
    put('a.jpg', 10, old())
    const svc = service({ moveIo: { ...realMoveIo, rename: async () => Promise.reject(errno('EBUSY')) } })
    const plan = await svc.plan({})
    const res = await svc.run({ plan: plan.id })
    expect(res.moved).toBe(0)
    expect(res.failed).toEqual([{ name: 'a.jpg', reason: 'locked' }])
    expect(fs.existsSync(path.join(src, 'a.jpg'))).toBe(true)
    expect((await svc.state()).runs).toHaveLength(0)
  })

  it('skips a file that changed between preview and run', async () => {
    put('a.jpg', 10, old())
    const svc = service()
    const plan = await svc.plan({})
    put('a.jpg', 11, old())
    const res = await svc.run({ plan: plan.id })
    expect(res.failed).toEqual([{ name: 'a.jpg', reason: 'changed' }])
  })

  it('refuses a folder that is a link out of the source', async () => {
    put('a.jpg', 10, old())
    const outside = path.join(base, 'outside')
    fs.mkdirSync(outside)
    fs.symlinkSync(outside, path.join(src, 'Ảnh'), 'junction')
    const svc = service()
    const plan = await svc.plan({})
    expect(plan.items).toHaveLength(0)
    expect(plan.skipped.find((s) => s.name === 'a.jpg')?.reason).toBe('blocked')
    expect(fs.readdirSync(outside)).toHaveLength(0)
  })

  it('takes no path from the renderer: unknown plans, sources and dangerous folders are refused', async () => {
    put('a.jpg', 10, old())
    const svc = service()
    expect((await svc.run({ plan: 'bogus' })).error).toBe('expired')
    expect((await svc.run({ plan: 1, from: 'C:\\Windows\\x', to: 'D:\\y' })).error).toBe('unknown')
    expect((await svc.plan({ source: os.homedir() })).error).toBe('unknown')
    const before = (await svc.state()).settings.source
    expect((await svc.patch({ source: os.homedir() })).settings.source).toBe(before)
    expect((await svc.addSource(os.homedir())).reason).toBe('protected')
    expect((await svc.addSource(path.parse(os.homedir()).root)).reason).toBe('root')
    expect((await svc.addSource(path.join(os.homedir(), process.platform === 'win32' ? 'AppData' : '.config'))).reason).toBe('protected')
    expect((await svc.addSource(data)).reason).toBe('protected')
    expect((await svc.addSource('relative/dir')).reason).toBe('relative')
    if (process.platform === 'win32') expect((await svc.addSource('C:\\Windows')).reason).toBe('system')
    const other = path.join(base, 'Other')
    fs.mkdirSync(other)
    expect((await svc.addSource(other)).state.settings.sources).toHaveLength(2)
    expect((await svc.removeSource(other)).settings.sources).toHaveLength(1)
  })
})

describe('auto', () => {
  it('waits for a file to settle, then sorts only files newer than the mark and logs for undo', async () => {
    const timers: Array<{ fn: () => void; ms: number }> = []
    let clock = Date.now() + FUTURE
    let onChange: () => void = () => undefined
    const svc = service({ now: () => clock, setTimer: (fn, ms) => timers.push({ fn, ms }), watch: (_d, cb) => ((onChange = cb), { close: () => undefined }) })
    put('before.jpg', 10, Date.now() - 3600_000)
    await svc.start()
    await svc.patch({ auto: true })
    expect((await svc.state()).watching).toHaveLength(1)
    clock += 1000
    const f = put('fresh.jpg', 10, clock)
    onChange()
    expect(await svc.pass(src)).toBe(0)
    expect(fs.existsSync(f)).toBe(true)
    expect(timers.some((t) => t.ms >= STABLE - 2000)).toBe(true)
    clock += STABLE + 1000
    expect(await svc.pass(src)).toBe(1)
    expect(fs.existsSync(path.join(src, 'Ảnh', 'fresh.jpg'))).toBe(true)
    expect(fs.existsSync(path.join(src, 'before.jpg'))).toBe(true)
    const run = (await svc.state()).runs[0]!
    expect(run.auto).toBe(true)
    expect((await svc.undo({ run: run.id })).restored).toBe(1)
    svc.dispose()
  })

  it('catches up on files that arrived while the app was closed and stops watching a deleted source', async () => {
    let clock = Date.now() + FUTURE
    const first = service({ now: () => clock })
    await first.start()
    await first.patch({ auto: true })
    first.dispose()
    clock += 60_000
    put('while-closed.pdf', 10, clock)
    clock += STABLE + 1000
    let failWatch: () => void = () => undefined
    const second = service({ now: () => clock, watch: (_d, _c, err) => ((failWatch = err), { close: () => undefined }) })
    await second.start()
    expect(await second.pass(src)).toBe(1)
    expect(fs.existsSync(path.join(src, 'Tài liệu', 'while-closed.pdf'))).toBe(true)
    fs.rmSync(src, { recursive: true, force: true })
    failWatch()
    expect((await second.state()).watching).toHaveLength(0)
    expect(await second.pass(src)).toBe(0)
    second.dispose()
  })
})

describe('e2e env', () => {
  it('only applies to a dev build', () => {
    expect(e2eStableMs({ CCM_E2E_ORGANIZE_STABLE_MS: '3000' }, false)).toBe(3000)
    expect(e2eStableMs({ CCM_E2E_ORGANIZE_STABLE_MS: '3000' }, true)).toBeUndefined()
    expect(e2ePath({ CCM_E2E_ORGANIZE_DOWNLOADS: 'C:\\tmp' }, true, 'DOWNLOADS')).toBeNull()
    expect(e2ePath({ CCM_E2E_ORGANIZE_DOWNLOADS: 'C:\\tmp' }, false, 'DOWNLOADS')).toBe('C:\\tmp')
  })
})
