import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import zlib from 'node:zlib'
import { afterEach, describe, expect, it } from 'vitest'
import { buildCompared, cleanRoots, compareAt, freeDrops, isSnapshotId, SnapshotStore, toSnapshot, topGrowth, type Snapshot } from '../src/diskMap/snapshots'

const dirs: string[] = []
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'disksnap-store-'))
  dirs.push(d)
  return d
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

const GB = 1024 ** 3
const snap = (at: number, dirsList: [string, number][] = [['C:\\Users', 10]], free = 100 * GB): Snapshot => ({
  v: 1,
  at,
  roots: [{ path: 'C:\\', size: 50, dirs: dirsList }],
  volumes: [{ mount: 'C:\\', label: 'OS', free, total: 500 * GB }],
})
const META = { id: '00000001-1', at: 1, bytes: 0, roots: [] as string[] }

describe('SnapshotStore', () => {
  it('writes, reads back and lists newest first', async () => {
    const store = new SnapshotStore(path.join(tmp(), 'snapshots'))
    const a = await store.write(snap(1_700_000_000_000))
    const b = await store.write(snap(1_700_000_100_000))
    expect((await store.list()).map((m) => m.id)).toEqual([b.id, a.id])
    expect(await store.read(a.id)).toEqual(snap(1_700_000_000_000))
    expect(a.bytes).toBe(50)
    expect(a.roots).toEqual(['C:\\'])
    expect((await store.latest())?.meta.id).toBe(b.id)
  })

  it('keeps 30 and drops the oldest', async () => {
    const dir = path.join(tmp(), 'snapshots')
    const store = new SnapshotStore(dir)
    const ids: string[] = []
    for (let i = 0; i < 33; i++) ids.push((await store.write(snap(1_700_000_000_000 + i))).id)
    const files = readdirSync(dir).filter((f) => f.endsWith('.json.gz'))
    expect(files).toHaveLength(30)
    expect(files).not.toContain(`${ids[0]}.json.gz`)
    expect(files).not.toContain(`${ids[2]}.json.gz`)
    expect(files).toContain(`${ids[3]}.json.gz`)
    expect(await store.list()).toHaveLength(30)
  })

  it('rotates by write order when the clock goes backwards', async () => {
    const store = new SnapshotStore(path.join(tmp(), 's'), 3)
    await store.write(snap(5_000_000_000))
    await store.write(snap(4_000_000_000))
    await store.write(snap(3_000_000_000))
    const d = await store.write(snap(2_000_000_000))
    const list = await store.list()
    expect(list).toHaveLength(3)
    expect(list[0]!.id).toBe(d.id)
    expect(list.map((m) => m.at)).toEqual([2_000_000_000, 3_000_000_000, 4_000_000_000])
    expect((await store.latest())?.meta.at).toBe(2_000_000_000)
  })

  it('skips corrupt files and keeps numbering past them', async () => {
    const dir = path.join(tmp(), 's')
    const store = new SnapshotStore(dir)
    const a = await store.write(snap(1_700_000_000_000))
    mkdirSync(dir, { recursive: true })
    writeFileSync(path.join(dir, '00000002-1700000000001.json.gz'), 'not gzip')
    writeFileSync(path.join(dir, '00000003-1700000000002.json.gz'), zlib.gzipSync('{"v":1,"at":1,"roots":[]}'))
    const c = await store.write(snap(1_700_000_000_003))
    expect(c.id.startsWith('00000004-')).toBe(true)
    expect((await store.list()).map((m) => m.id)).toEqual([c.id, a.id])
    expect(await store.read('00000002-1700000000001')).toBeNull()
    expect(await store.read('00000003-1700000000002')).toBeNull()
  })

  it('refuses ids that are not file stems', async () => {
    const store = new SnapshotStore(path.join(tmp(), 's'))
    await store.write(snap(1_700_000_000_000))
    for (const bad of ['../x', '..\\x', '00000001-1700000000000.json.gz', '', '1', '00000001-17000']) {
      expect(isSnapshotId(bad)).toBe(false)
      expect(await store.read(bad)).toBeNull()
    }
  })

  it('stores folders only, never file names', async () => {
    const store = new SnapshotStore(path.join(tmp(), 's'))
    const summary = { roots: [{ path: 'C:\\', size: 9, dirs: [['C:\\Users', 5, 'secret.txt'], ['C:\\x'.repeat(1000), 1]], files: [{ name: 'passwords.txt', size: 1 }] }], leak: 'passwords.txt' }
    const s = toSnapshot(summary, [{ mount: 'C:\\', label: 'OS', free: 1, total: 2, name: 'passwords.txt' }], 1_700_000_000_000)!
    const meta = await store.write(s)
    const raw = zlib.gunzipSync(readFileSync(path.join(store.dir, `${meta.id}.json.gz`))).toString('utf8')
    expect(raw).not.toContain('passwords')
    expect(raw).not.toContain('secret.txt')
    expect(JSON.parse(raw).roots[0].dirs).toEqual([['C:\\Users', 5]])
  })

  it('serialises overlapping writes', async () => {
    const store = new SnapshotStore(path.join(tmp(), 's'))
    const metas = await Promise.all([1, 2, 3, 4, 5].map((i) => store.write(snap(1_700_000_000_000 + i))))
    expect(new Set(metas.map((m) => m.id)).size).toBe(5)
  })

  it('returns empty on a missing directory', async () => {
    const store = new SnapshotStore(path.join(tmp(), 'none'))
    expect(await store.list()).toEqual([])
    expect(await store.latest()).toBeNull()
  })
})

describe('toSnapshot / cleanRoots', () => {
  it('rejects empty or hostile input', () => {
    expect(toSnapshot(null, [], 1)).toBeNull()
    expect(toSnapshot({ roots: [] }, [], 1)).toBeNull()
    expect(toSnapshot({ roots: [{ path: '', size: 1, dirs: [] }] }, [], 1)).toBeNull()
    expect(cleanRoots([{ path: 'C:\\', size: -5, dirs: [['x', NaN], 'bad', ['a\0b', 1]] }])).toEqual([{ path: 'C:\\', size: 0, dirs: [['x', 0]] }])
  })
})

describe('compare', () => {
  const base = snap(1, [['C:\\Users', 100], ['C:\\Users\\a', 60], ['C:\\Users\\b', 40], ['C:\\Users\\a\\x', 30], ['C:\\Old', 20], ['C:\\Users\\a\\x\\y', 10]])
  const cur = snap(2, [['C:\\Users', 180], ['C:\\Users\\a', 60], ['C:\\Users\\b', 120], ['C:\\Users\\a\\x', 30], ['C:\\New', 15], ['C:\\Users\\b\\cache', 70], ['C:\\Users\\a\\x\\y', 10]])

  it('lists the roots and the children of a folder with deltas', () => {
    const c = buildCompared(base, cur, 'win32')
    const top = compareAt(c, '', 'win32', META)
    expect(top.rows.map((r) => r.path)).toEqual(['C:\\'])
    const users = compareAt(c, 'C:\\Users', 'win32', META)
    expect(users.covered).toBe(true)
    expect(users.rows.map((r) => [r.name, r.delta, r.state])).toEqual([['b', 80, 'changed'], ['a', 0, 'same']])
    const rootRows = compareAt(c, 'C:\\', 'win32', META).rows
    expect(rootRows.map((r) => [r.name, r.state])).toEqual([['Users', 'changed'], ['New', 'new'], ['Old', 'gone']])
    expect(rootRows.find((r) => r.name === 'Old')).toMatchObject({ before: 20, after: 0, delta: -20 })
    expect(rootRows.find((r) => r.name === 'New')).toMatchObject({ before: 0, after: 15, delta: 15 })
  })

  it('does not cover folders at the snapshot depth, or unknown ones', () => {
    const c = buildCompared(base, cur, 'win32')
    expect(compareAt(c, 'C:\\Users\\a\\x\\y', 'win32', META)).toMatchObject({ covered: false, rows: [] })
    expect(compareAt(c, 'C:\\Nope', 'win32', META)).toMatchObject({ covered: false, rows: [] })
  })

  it('matches paths without regard to case on Windows only', () => {
    expect(compareAt(buildCompared(base, cur, 'win32'), 'c:\\users', 'win32', META).rows).toHaveLength(2)
    const mixed = snap(2, [['C:\\USERS', 999]])
    expect(buildCompared(base, mixed, 'win32').nodes.get('c:\\users')).toMatchObject({ before: 100, after: 999 })
    expect(buildCompared(base, mixed, 'linux').nodes.get('C:\\USERS')).toMatchObject({ before: null, after: 999 })
  })

  it('top growth skips parents explained by a child and the roots', () => {
    const top = topGrowth(buildCompared(base, cur, 'win32'))
    expect(top.map((r) => r.path)).toEqual(['C:\\Users\\b\\cache', 'C:\\New'])
    expect(top[0]).toMatchObject({ delta: 70, state: 'new' })
    expect(top.every((r) => r.delta > 0)).toBe(true)
  })

  it('ignores roots present in only one snapshot', () => {
    const other: Snapshot = { ...cur, roots: [{ path: 'D:\\', size: 5, dirs: [['D:\\z', 5]] }] }
    const c = buildCompared(base, other, 'win32')
    expect(c.roots).toEqual([])
    expect(c.nodes.size).toBe(0)
  })

  it('treats a removed root as no data', () => {
    const c = buildCompared(base, { ...cur, roots: [] }, 'win32')
    expect(compareAt(c, '', 'win32', META).rows).toEqual([])
  })
})

describe('freeDrops', () => {
  it('reports drives whose free space fell by more than the threshold', () => {
    const a = snap(1, [], 100 * GB)
    expect(freeDrops(a, snap(2, [], 94 * GB), 'win32', 5 * GB)).toEqual([{ mount: 'C:\\', label: 'C:', free: 94 * GB, total: 500 * GB, drop: 6 * GB }])
    expect(freeDrops(a, snap(2, [], 96 * GB), 'win32', 5 * GB)).toEqual([])
    expect(freeDrops(a, snap(2, [], 150 * GB), 'win32', 5 * GB)).toEqual([])
    expect(freeDrops({ ...a, volumes: [] }, snap(2, [], 1), 'win32', 5 * GB)).toEqual([])
  })
})
