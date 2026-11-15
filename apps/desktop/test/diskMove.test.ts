import { existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readlinkSync, realpathSync, renameSync, rmSync, symlinkSync, writeFileSync, appendFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MOVE_SUFFIX, moveNeed, parseMoveId, parseMovePlan, parseMoveRecover, type DiskHistoryEntry, type MoveProgress } from '@ccm/shared'
import { DiskMoveService, holderOf, type DiskMoveDeps, type MoveAsk } from '../src/diskMove/service'
import { createDiskMove } from '../src/diskMove/ipc'
import { moveBoxOptions } from '../src/diskMove/dialog'
import { pickSamples } from '../src/diskMove/tree'
import { fakeDev } from '../src/diskMove/procs'

let tmp: string
beforeEach(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-move-')))
  mkdirSync(path.join(tmp, 'vols', 'A'), { recursive: true })
  mkdirSync(path.join(tmp, 'vols', 'netB'), { recursive: true })
  mkdirSync(path.join(tmp, 'trash'), { recursive: true })
})
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true })
})

const at = (...p: string[]): string => path.join(tmp, ...p)
function put(rel: string, text = 'x'): string {
  const file = at(rel)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
  return file
}
const linkType = process.platform === 'win32' ? 'junction' : 'dir'

function data(): string {
  put('home/.gradle/caches/a.bin', 'a'.repeat(5000))
  put('home/.gradle/caches/sub/b.txt', 'hello')
  put('home/.gradle/wrapper/dists/gradle-8/x.zip', 'z'.repeat(1234))
  put('outside/keep.txt', 'outside')
  symlinkSync(at('outside'), at('home/.gradle/out-link'), linkType)
  return at('home/.gradle')
}

type D = DiskMoveDeps & { asks: MoveAsk[]; history: DiskHistoryEntry[]; progress: MoveProgress[] }

function deps(over: Partial<DiskMoveDeps> = {}): D {
  const asks: MoveAsk[] = []
  const history: DiskHistoryEntry[] = []
  const progress: MoveProgress[] = []
  let n = 0
  return {
    platform: process.platform,
    home: at('home'),
    dataDir: at('data'),
    keep: () => [],
    self: () => [at('app')],
    volumes: async () => [
      { mount: at('vols', 'A'), label: 'A', network: false, total: 1e12, free: 1e12 },
      { mount: at('vols', 'netB'), label: 'B', network: true, total: 1e12, free: 1e12 },
      { mount: at('home'), label: 'C', network: false, total: 1e12, free: 1e12 },
    ],
    suggest: () => [
      { kind: 'gradle', path: at('home', '.gradle') },
      { kind: 'docker', path: at('home', 'Docker'), guide: true },
    ],
    holders: async () => [],
    confirm: async (a) => {
      asks.push(a)
      return true
    },
    trash: async (p) => renameSync(p, at('trash', String(++n))),
    record: async (e) => {
      history.push(e)
    },
    emit: (p) => progress.push(p),
    devOf: fakeDev(tmp),
    asks,
    history,
    progress,
    ...over,
  }
}

async function planFor(svc: DiskMoveService, kind = 'gradle', mount = at('vols', 'A')) {
  const list = await svc.list()
  const c = list.find((x) => x.kind === kind)!
  return { c, res: await svc.plan(c.id, mount) }
}

function journal(): Array<Record<string, unknown>> {
  return readFileSync(at('data', 'disk', 'moves.jsonl'), 'utf8')
    .trim()
    .split('\n')
    .map((l) => JSON.parse(l) as Record<string, unknown>)
}

describe('move to another drive', () => {
  it('copies, verifies, links, sends the old copy to the bin, then moves back', async () => {
    const src = data()
    const d = deps()
    const svc = new DiskMoveService(d)
    const { res } = await planFor(svc)
    expect(res.error).toBeUndefined()
    const plan = res.plan!
    expect(plan.target).toBe(at('vols', 'A', 'OrreryMoved', '.gradle'))
    expect(plan.files).toBe(3)
    expect(plan.bytes).toBe(5000 + 5 + 1234)
    expect(plan.need).toBe(moveNeed(plan.bytes))
    const r = await svc.run(plan.id)
    expect(r.ok).toBe(true)
    expect(lstatSync(src).isSymbolicLink()).toBe(true)
    expect(realpathSync(src)).toBe(realpathSync(plan.target))
    expect(readFileSync(path.join(src, 'caches', 'sub', 'b.txt'), 'utf8')).toBe('hello')
    expect(lstatSync(path.join(plan.target, 'out-link')).isSymbolicLink()).toBe(true)
    expect(path.resolve(readlinkSync(path.join(plan.target, 'out-link')))).toBe(path.resolve(at('outside')))
    expect(existsSync(src + MOVE_SUFFIX)).toBe(false)
    expect(existsSync(at('trash', '1', 'caches', 'a.bin'))).toBe(true)
    expect(existsSync(at('outside', 'keep.txt'))).toBe(true)
    expect(d.asks.map((a) => a.kind)).toEqual(['move'])
    expect(d.history[0]).toMatchObject({ action: 'trash', paths: [src + MOVE_SUFFIX] })
    expect(journal().map((e) => e.stage)).toEqual(['copying', 'renamed', 'linked', 'done'])
    expect(d.progress.some((p) => p.phase === 'verify') && d.progress.at(-1)?.phase).toBe('done')
    const st = await svc.state()
    expect(st.moves).toHaveLength(1)
    expect(st.pending).toEqual([])
    expect((await svc.list()).find((c) => c.kind === 'gradle')?.block).toBe('moved')

    const back = await svc.back(st.moves[0]!.id)
    expect(back.ok).toBe(true)
    expect(lstatSync(src).isSymbolicLink()).toBe(false)
    expect(lstatSync(src).isDirectory()).toBe(true)
    expect(readFileSync(path.join(src, 'caches', 'a.bin'), 'utf8')).toBe('a'.repeat(5000))
    expect(existsSync(plan.target)).toBe(false)
    expect((await svc.state()).moves).toEqual([])
    expect(d.asks.map((a) => a.kind)).toEqual(['move', 'back'])
  })

  for (const step of ['mkdir', 'copyFile', 'verify', 'rename', 'symlink', 'linkCheck', 'cleanup']) {
    it(`rolls back when ${step} fails`, async () => {
      const src = data()
      const fault = vi.fn(async (s: string) => {
        if (s === step) throw Object.assign(new Error('boom'), { code: step === 'copyFile' ? 'ENOSPC' : 'EFAULT' })
      })
      const svc = new DiskMoveService(deps({ fault }))
      const { res } = await planFor(svc)
      const r = await svc.run(res.plan!.id)
      expect(r.ok).toBe(false)
      expect(r.rolledBack).toBe(true)
      expect(lstatSync(src).isSymbolicLink()).toBe(false)
      expect(readFileSync(path.join(src, 'caches', 'sub', 'b.txt'), 'utf8')).toBe('hello')
      expect(lstatSync(path.join(src, 'out-link')).isSymbolicLink()).toBe(true)
      expect(existsSync(src + MOVE_SUFFIX)).toBe(false)
      expect(existsSync(res.plan!.target)).toBe(false)
      expect(existsSync(at('outside', 'keep.txt'))).toBe(true)
      expect(journal().at(-1)?.stage).toBe('rolledBack')
      expect((await svc.state()).pending).toEqual([])
    })
  }

  it('rolls back when the copy is cancelled', async () => {
    data()
    let svc!: DiskMoveService
    const fault = async (s: string) => {
      if (s === 'copyFile') svc.cancel()
    }
    svc = new DiskMoveService(deps({ fault }))
    const { res } = await planFor(svc)
    const r = await svc.run(res.plan!.id)
    expect(r).toMatchObject({ ok: false, cancelled: true, rolledBack: true })
    expect(existsSync(res.plan!.target)).toBe(false)
  })

  it('asks a second time before deleting the old copy for good when the bin fails', async () => {
    const src = data()
    const d = deps({ trash: async () => Promise.reject(new Error('too big')) })
    const svc = new DiskMoveService(d)
    const { res } = await planFor(svc)
    expect((await svc.run(res.plan!.id)).ok).toBe(true)
    expect(d.asks.map((a) => a.kind)).toEqual(['move', 'permanent'])
    expect(existsSync(src + MOVE_SUFFIX)).toBe(false)
    expect(existsSync(at('outside', 'keep.txt'))).toBe(true)
    expect(d.history[0]).toMatchObject({ action: 'delete' })
  })

  it('keeps the old copy as leftover when the second confirm is declined, then finishes later', async () => {
    const src = data()
    const d = deps({ trash: async () => Promise.reject(new Error('too big')) })
    d.confirm = async (a) => {
      d.asks.push(a)
      return a.kind !== 'permanent'
    }
    const svc = new DiskMoveService(d)
    const { res } = await planFor(svc)
    const r = await svc.run(res.plan!.id)
    expect(r).toMatchObject({ ok: true, leftover: src + MOVE_SUFFIX })
    const st = await svc.state()
    expect(st.pending).toMatchObject([{ stage: 'linked', kind: 'move' }])
    expect(await svc.back(st.moves[0]!.id)).toMatchObject({ ok: false, error: 'leftover' })
    d.confirm = async () => true
    expect((await svc.recover(st.pending[0]!.id, 'finish')).ok).toBe(true)
    expect(existsSync(src + MOVE_SUFFIX)).toBe(false)
    expect((await svc.state()).pending).toEqual([])
    expect((await svc.state()).moves).toHaveLength(1)
  })

  it('locks when a tool holds the folder', async () => {
    const src = data()
    const svc = new DiskMoveService(deps({ holders: async () => [{ name: 'java.exe', cmd: `java -Dgradle.user.home=${src} -cp x GradleDaemon` }] }))
    const { res } = await planFor(svc)
    expect(res).toEqual({ error: 'locked', holder: 'java.exe' })
    expect(holderOf(src, [{ name: 'node', cmd: `node ${src}-other/x.js` }], process.platform)).toBeNull()
    expect(holderOf(src, [{ name: 'notepad.exe', cmd: `notepad ${src}/a` }], process.platform)).toBeNull()
    expect(holderOf(src, [{ name: 'adb', cmd: `adb -s x push ${src.toUpperCase()}/a` }], 'win32')).toBe('adb')
  })

  it('blocks dangerous sources and odd volumes', async () => {
    data()
    put('app/res/a.txt')
    put('data/x.txt')
    mkdirSync(at('home', 'Docker'))
    symlinkSync(at('outside'), at('home', 'linked'), linkType)
    const svc = new DiskMoveService(
      deps({
        suggest: () => [
          { kind: 'picked', path: at('home') },
          { kind: 'picked', path: at('app', 'res') },
          { kind: 'picked', path: at('data') },
          { kind: 'picked', path: at('home', 'linked') },
          { kind: 'picked', path: at('home', 'nope') },
          { kind: 'picked', path: path.parse(tmp).root },
          { kind: 'gradle', path: at('home', '.gradle') },
          { kind: 'docker', path: at('home', 'Docker'), guide: true },
        ],
      }),
    )
    const list = await svc.list()
    const by = (p: string) => list.find((c) => c.path === path.normalize(p) || c.path === p)
    expect(by(at('home'))?.block).toBe('guard')
    expect(by(at('app', 'res'))?.block).toBe('self')
    expect(by(at('data'))?.block).toMatch(/self|guard/)
    expect(by(at('home', 'linked'))?.block).toBe('link')
    expect(by(at('home', 'nope'))?.block).toBe('missing')
    expect(by(path.parse(tmp).root)?.block).toBe('guard')
    const docker = by(at('home', 'Docker'))!
    expect(docker.guide).toBe(true)
    expect(await svc.plan(docker.id, at('vols', 'A'))).toEqual({ error: 'invalid' })
    expect(await svc.plan(by(at('home'))!.id, at('vols', 'A'))).toEqual({ error: 'guard' })
    const g = by(at('home', '.gradle'))!
    const vols = await svc.volumes(g.id)
    expect(vols.map((v) => [v.label, v.ok, v.reason ?? ''])).toEqual([
      ['A', true, ''],
      ['B', false, 'network'],
      ['C', false, 'same'],
    ])
    expect(await svc.plan(g.id, at('vols', 'netB'))).toEqual({ error: 'network' })
    expect(await svc.plan(g.id, at('nowhere'))).toEqual({ error: 'invalid' })
  })

  it('refuses when the target drive lacks room plus 10%', async () => {
    data()
    const svc = new DiskMoveService(deps({ freeOf: async () => 6000 }))
    const { res } = await planFor(svc)
    expect(res).toEqual({ error: 'space' })
  })

  it('refuses to move back when the target drive is gone', async () => {
    const src = data()
    const svc = new DiskMoveService(deps())
    const { res } = await planFor(svc)
    await svc.run(res.plan!.id)
    const id = (await svc.state()).moves[0]!.id
    renameSync(res.plan!.target, at('vols', 'A', 'away'))
    expect(await svc.back(id)).toMatchObject({ ok: false })
    expect(lstatSync(src).isSymbolicLink()).toBe(true)
    renameSync(at('vols', 'A', 'away'), res.plan!.target)
    rmSync(at('vols', 'A'), { recursive: true, force: true })
    expect((await svc.back(id)).error).toMatch(/targetMissing|notLink/)
    expect(lstatSync(src).isSymbolicLink()).toBe(true)
  })

  it('finds an interrupted move on the next start and restores it', async () => {
    const src = data()
    const target = at('vols', 'A', 'OrreryMoved', '.gradle')
    mkdirSync(path.dirname(target), { recursive: true })
    mkdirSync(target)
    put('vols/A/OrreryMoved/.gradle/caches/a.bin', 'partial')
    renameSync(src, src + MOVE_SUFFIX)
    mkdirSync(at('data', 'disk'), { recursive: true })
    appendFileSync(at('data', 'disk', 'moves.jsonl'), JSON.stringify({ id: 'abc-1', stage: 'copying', at: 1, kind: 'move', source: src, target, bytes: 10, files: 1 }) + '\n' + JSON.stringify({ id: 'abc-1', stage: 'renamed', at: 2 }) + '\n')
    const svc = new DiskMoveService(deps())
    const st = await svc.state()
    expect(st.pending).toEqual([{ id: 'abc-1', kind: 'move', source: src, target, stage: 'renamed' }])
    expect((await svc.list()).find((c) => c.kind === 'gradle')?.block).toBe('missing')
    expect(await svc.recover('abc-1', 'finish')).toMatchObject({ ok: false, error: 'invalid' })
    const r = await svc.recover('abc-1', 'restore')
    expect(r.ok).toBe(true)
    expect(readFileSync(path.join(src, 'caches', 'a.bin'), 'utf8')).toBe('a'.repeat(5000))
    expect(existsSync(target)).toBe(false)
    expect((await svc.state()).pending).toEqual([])
  })

  it('restores an interrupted move that already left a link', async () => {
    const src = data()
    const target = at('vols', 'A', 'OrreryMoved', '.gradle')
    const svc0 = new DiskMoveService(deps({ trash: async () => Promise.reject(new Error('x')), confirm: async (a) => a.kind === 'move' }))
    const { res } = await planFor(svc0)
    await svc0.run(res.plan!.id)
    const svc = new DiskMoveService(deps())
    const p = (await svc.state()).pending[0]!
    expect(p.stage).toBe('linked')
    expect((await svc.recover(p.id, 'restore')).ok).toBe(true)
    expect(lstatSync(src).isSymbolicLink()).toBe(false)
    expect(existsSync(target)).toBe(false)
    expect(readFileSync(path.join(src, 'caches', 'sub', 'b.txt'), 'utf8')).toBe('hello')
  })

  it('copies paths longer than 260 characters', async () => {
    const deep = Array.from({ length: 14 }, (_, i) => `folder-name-${i}-abcdefghij`).join(path.sep)
    data()
    put(path.join('home', '.gradle', deep, 'deep.txt'), 'deep')
    expect(path.join(at('home', '.gradle'), deep, 'deep.txt').length).toBeGreaterThan(260)
    const svc = new DiskMoveService(deps())
    const { res } = await planFor(svc)
    expect((await svc.run(res.plan!.id)).ok).toBe(true)
    expect(readFileSync(path.join(res.plan!.target, deep, 'deep.txt'), 'utf8')).toBe('deep')
  })

  it('picks hash samples from the biggest and spread files', () => {
    const files = Array.from({ length: 20 }, (_, i) => ({ rel: `f${i}`, size: i }))
    const s = pickSamples(files)
    expect(s.length).toBe(6)
    expect(s.slice(0, 3)).toEqual(['f19', 'f18', 'f17'])
  })
})

describe('ipc', () => {
  it('only takes ids made by main and answers the app frame', async () => {
    expect(parseMovePlan({ candidate: 'abc', volume: 'D:\\' })).toEqual({ candidate: 'abc', volume: 'D:\\' })
    expect(parseMovePlan({ candidate: '../x', volume: 'D:\\' })).toBeNull()
    expect(parseMoveId({ plan: 'C:\\Users' }, 'plan')).toBeNull()
    expect(parseMoveRecover({ pending: 'a', action: 'rm' })).toBeNull()
    const handlers = new Map<string, (e: unknown, a?: unknown) => Promise<unknown>>()
    const ipcMain = { handle: vi.fn((c: string, h: (e: unknown, a?: unknown) => Promise<unknown>) => handlers.set(c, h)), removeHandler: vi.fn() }
    data()
    const dm = createDiskMove({ ...deps(), ipcMain, isAppFrame: (e) => e === 'app', broadcast: () => undefined, ask: async () => 1 })
    expect(await handlers.get('ccm:diskmove-list')!('evil')).toEqual([])
    const list = (await handlers.get('ccm:diskmove-list')!('app')) as Array<{ id: string; kind: string }>
    const g = list.find((c) => c.kind === 'gradle')!
    expect(await handlers.get('ccm:diskmove-run')!('app', { plan: 'nope' })).toEqual({ ok: false, error: 'invalid' })
    expect(await handlers.get('ccm:diskmove-from-clean')!('app', { path: at('home', '.gradle') })).toBeNull()
    const plan = (await handlers.get('ccm:diskmove-plan')!('app', { candidate: g.id, volume: at('vols', 'A') })) as { plan: { id: string } }
    expect(await handlers.get('ccm:diskmove-run')!('app', { plan: plan.plan.id })).toEqual({ ok: false, cancelled: true })
    expect(lstatSync(at('home', '.gradle')).isSymbolicLink()).toBe(false)
    dm.dispose()
    expect(ipcMain.removeHandler).toHaveBeenCalledTimes(10)
  })

  it('builds native dialogs', () => {
    expect(moveBoxOptions({ kind: 'move', source: 'C:\\a', target: 'D:\\b', bytes: 2048 }, 'vi').buttons).toEqual(['Chuyển', 'Huỷ'])
    expect(moveBoxOptions({ kind: 'permanent', source: 'C:\\a', target: 'C:\\a.orrery-moving', bytes: 1 }, 'en').type).toBe('warning')
  })
})
