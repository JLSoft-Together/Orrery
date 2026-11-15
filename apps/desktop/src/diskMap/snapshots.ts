import fs from 'node:fs/promises'
import path from 'node:path'
import zlib from 'node:zlib'
import { promisify } from 'node:util'
import { DISK_SNAPSHOT_DEPTH, DISK_SNAPSHOT_KEEP, DISK_SNAPSHOT_TOP, type DiskCompare, type DiskDiffRow, type DiskSnapshotMeta } from '@ccm/shared'

const gzip = promisify(zlib.gzip)
const gunzip = promisify(zlib.gunzip)

export const SNAPSHOT_DIRS_MAX = 60_000
const ROOTS_MAX = 64
const PATH_MAX = 2048
const VOLUMES_MAX = 64
const FILE_MAX = 64 * 1024 * 1024
const ROWS_MAX = 2000
const FILE_RE = /^(\d{8})-(\d{10,15})\.json\.gz$/
const ID_RE = /^\d{8}-\d{10,15}$/

export type SnapshotDir = [string, number]

export interface SnapshotRoot {
  path: string
  size: number
  dirs: SnapshotDir[]
}

export interface SnapshotVolume {
  mount: string
  label: string
  free: number
  total: number
}

export interface Snapshot {
  v: 1
  at: number
  roots: SnapshotRoot[]
  volumes: SnapshotVolume[]
}

const num = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? Math.floor(v) : 0)
const text = (v: unknown): string | null => (typeof v === 'string' && v.length > 0 && v.length <= PATH_MAX && !v.includes('\0') ? v : null)

export function cleanRoots(raw: unknown): SnapshotRoot[] {
  const list = Array.isArray(raw) ? raw.slice(0, ROOTS_MAX) : []
  const out: SnapshotRoot[] = []
  let left = SNAPSHOT_DIRS_MAX
  for (const r of list) {
    const o = r && typeof r === 'object' ? (r as { path?: unknown; size?: unknown; dirs?: unknown }) : {}
    const p = text(o.path)
    if (!p) continue
    const dirs: SnapshotDir[] = []
    for (const d of Array.isArray(o.dirs) ? o.dirs : []) {
      if (left <= 0) break
      if (!Array.isArray(d)) continue
      const dp = text(d[0])
      if (!dp) continue
      dirs.push([dp, num(d[1])])
      left--
    }
    out.push({ path: p, size: num(o.size), dirs })
  }
  return out
}

export function cleanVolumes(raw: unknown): SnapshotVolume[] {
  const out: SnapshotVolume[] = []
  for (const v of Array.isArray(raw) ? raw.slice(0, VOLUMES_MAX) : []) {
    const o = v && typeof v === 'object' ? (v as { mount?: unknown; label?: unknown; free?: unknown; total?: unknown }) : {}
    const mount = text(o.mount)
    if (!mount) continue
    out.push({ mount, label: typeof o.label === 'string' ? o.label.slice(0, 120) : '', free: num(o.free), total: num(o.total) })
  }
  return out
}

export function toSnapshot(summary: unknown, volumes: unknown, at: number): Snapshot | null {
  const o = summary && typeof summary === 'object' ? (summary as { roots?: unknown }) : {}
  const roots = cleanRoots(o.roots)
  if (!roots.length) return null
  return { v: 1, at: Math.floor(at), roots, volumes: cleanVolumes(volumes) }
}

export function parseSnapshot(raw: unknown): Snapshot | null {
  const o = raw && typeof raw === 'object' ? (raw as { v?: unknown; at?: unknown; roots?: unknown; volumes?: unknown }) : null
  if (!o || o.v !== 1 || typeof o.at !== 'number' || !Number.isFinite(o.at) || o.at <= 0) return null
  const roots = cleanRoots(o.roots)
  if (!roots.length) return null
  return { v: 1, at: Math.floor(o.at), roots, volumes: cleanVolumes(o.volumes) }
}

export const snapshotBytes = (s: Snapshot): number => s.roots.reduce((n, r) => n + r.size, 0)

export function isSnapshotId(v: unknown): v is string {
  return typeof v === 'string' && ID_RE.test(v)
}

export class SnapshotStore {
  private chain: Promise<unknown> = Promise.resolve()
  private readonly metas = new Map<string, DiskSnapshotMeta | null>()

  constructor(readonly dir: string, private readonly keep = DISK_SNAPSHOT_KEEP) {}

  private serial<T>(fn: () => Promise<T>): Promise<T> {
    const run = this.chain.then(fn, fn)
    this.chain = run.catch(() => undefined)
    return run
  }

  private async names(): Promise<{ name: string; seq: number }[]> {
    const all = await fs.readdir(this.dir).catch(() => [] as string[])
    return all.flatMap((name) => {
      const m = FILE_RE.exec(name)
      return m ? [{ name, seq: Number(m[1]) }] : []
    }).sort((a, b) => a.seq - b.seq)
  }

  write(snap: Snapshot): Promise<DiskSnapshotMeta> {
    return this.serial(async () => {
      await fs.mkdir(this.dir, { recursive: true })
      const names = await this.names()
      const seq = (names.at(-1)?.seq ?? 0) + 1
      const id = `${String(seq).padStart(8, '0')}-${snap.at}`
      const file = path.join(this.dir, `${id}.json.gz`)
      const tmp = `${file}.tmp`
      const body = await gzip(Buffer.from(JSON.stringify(snap), 'utf8'), { level: 6 })
      try {
        await fs.writeFile(tmp, body)
        await fs.rename(tmp, file)
      } catch (err) {
        await fs.rm(tmp, { force: true }).catch(() => undefined)
        throw err
      }
      const meta: DiskSnapshotMeta = { id, at: snap.at, bytes: snapshotBytes(snap), roots: snap.roots.map((r) => r.path) }
      this.metas.set(`${id}.json.gz`, meta)
      const all = [...names, { name: `${id}.json.gz`, seq }]
      for (const old of all.slice(0, Math.max(0, all.length - this.keep))) {
        await fs.rm(path.join(this.dir, old.name), { force: true }).catch(() => undefined)
        this.metas.delete(old.name)
      }
      return meta
    })
  }

  async read(id: string): Promise<Snapshot | null> {
    if (!isSnapshotId(id)) return null
    try {
      const file = path.join(this.dir, `${id}.json.gz`)
      const st = await fs.stat(file)
      if (!st.isFile() || st.size > FILE_MAX) return null
      const raw = await gunzip(await fs.readFile(file), { maxOutputLength: FILE_MAX })
      return parseSnapshot(JSON.parse(raw.toString('utf8')))
    } catch {
      return null
    }
  }

  async list(): Promise<DiskSnapshotMeta[]> {
    const names = await this.names()
    const live = new Set(names.map((n) => n.name))
    for (const k of [...this.metas.keys()]) if (!live.has(k)) this.metas.delete(k)
    const out: DiskSnapshotMeta[] = []
    for (const { name } of [...names].reverse()) {
      if (!this.metas.has(name)) {
        const snap = await this.read(name.replace(/\.json\.gz$/, ''))
        this.metas.set(name, snap ? { id: name.replace(/\.json\.gz$/, ''), at: snap.at, bytes: snapshotBytes(snap), roots: snap.roots.map((r) => r.path) } : null)
      }
      const meta = this.metas.get(name)
      if (meta) out.push(meta)
    }
    return out
  }

  async latest(): Promise<{ meta: DiskSnapshotMeta; snap: Snapshot } | null> {
    for (const meta of await this.list()) {
      const snap = await this.read(meta.id)
      if (snap) return { meta, snap }
    }
    return null
  }
}

const SEP = /[\\/]+$/

export function pathKey(p: string, platform: NodeJS.Platform): string {
  const t = p.replace(SEP, '')
  return platform === 'win32' || platform === 'darwin' ? t.toLowerCase() : t
}

function parentOf(p: string): string {
  const t = p.replace(SEP, '')
  const i = Math.max(t.lastIndexOf('\\'), t.lastIndexOf('/'))
  return i < 0 ? '' : t.slice(0, i)
}

export function dirName(p: string): string {
  const t = p.replace(SEP, '')
  const i = Math.max(t.lastIndexOf('\\'), t.lastIndexOf('/'))
  return (i < 0 ? t : t.slice(i + 1)) || p
}

interface Node {
  path: string
  before: number | null
  after: number | null
  depth: number
  parent: string
  root: boolean
}

export interface Compared {
  nodes: Map<string, Node>
  children: Map<string, string[]>
  roots: string[]
}

export function buildCompared(base: Snapshot, cur: Snapshot, platform: NodeJS.Platform): Compared {
  const nodes = new Map<string, Node>()
  const children = new Map<string, string[]>()
  const roots: string[] = []
  const baseRoots = new Map(base.roots.map((r) => [pathKey(r.path, platform), r]))
  const put = (key: string, p: string, side: 'before' | 'after', size: number, depth: number, parent: string, root: boolean): void => {
    const n = nodes.get(key)
    if (n) n[side] = size
    else {
      nodes.set(key, { path: p, before: null, after: null, depth, parent, root, [side]: size })
      if (!root) {
        const list = children.get(parent)
        if (list) list.push(key)
        else children.set(parent, [key])
      }
    }
  }
  for (const root of cur.roots) {
    const rk = pathKey(root.path, platform)
    const b = baseRoots.get(rk)
    if (!b) continue
    roots.push(rk)
    put(rk, root.path, 'after', root.size, 0, '', true)
    put(rk, b.path, 'before', b.size, 0, '', true)
    const depthOf = new Map<string, number>([[rk, 0]])
    const add = (side: 'before' | 'after', dirs: SnapshotDir[]): void => {
      for (const [p, size] of dirs) {
        const key = pathKey(p, platform)
        const parent = pathKey(parentOf(p), platform)
        const depth = (depthOf.get(parent) ?? 0) + 1
        depthOf.set(key, depth)
        put(key, p, side, size, depth, parent, false)
      }
    }
    add('after', root.dirs)
    add('before', b.dirs)
  }
  return { nodes, children, roots }
}

function rowOf(n: Node): DiskDiffRow {
  const before = n.before ?? 0
  const after = n.after ?? 0
  const state: DiskDiffRow['state'] = n.before === null ? 'new' : n.after === null ? 'gone' : after === before ? 'same' : 'changed'
  return { path: n.path, name: dirName(n.path), before, after, delta: after - before, state }
}

export function topGrowth(c: Compared, limit = DISK_SNAPSHOT_TOP): DiskDiffRow[] {
  const delta = (n: Node): number => (n.after ?? 0) - (n.before ?? 0)
  const out: DiskDiffRow[] = []
  for (const [key, n] of c.nodes) {
    const d = delta(n)
    if (n.root || d <= 0) continue
    const explained = (c.children.get(key) ?? []).some((k) => {
      const child = c.nodes.get(k)
      return child !== undefined && delta(child) >= d * 0.8
    })
    if (!explained) out.push(rowOf(n))
  }
  return out.sort((a, b) => b.delta - a.delta || a.path.localeCompare(b.path)).slice(0, limit)
}

export function compareAt(c: Compared, at: string, platform: NodeJS.Platform, base: DiskSnapshotMeta): DiskCompare {
  const top = topGrowth(c)
  if (!at) {
    const rows = c.roots.map((k) => rowOf(c.nodes.get(k)!)).sort((a, b) => b.delta - a.delta || a.path.localeCompare(b.path))
    return { base, path: '', covered: true, rows: rows.slice(0, ROWS_MAX), top }
  }
  const key = pathKey(at, platform)
  const node = c.nodes.get(key)
  const covered = !!node && node.depth < DISK_SNAPSHOT_DEPTH
  const rows = covered ? (c.children.get(key) ?? []).map((k) => rowOf(c.nodes.get(k)!)).sort((a, b) => b.delta - a.delta || a.path.localeCompare(b.path)) : []
  return { base, path: at, covered, rows: rows.slice(0, ROWS_MAX), top }
}

export function freeDrops(base: Snapshot, cur: Snapshot, platform: NodeJS.Platform, minDrop: number): { mount: string; label: string; free: number; total: number; drop: number }[] {
  const was = new Map(base.volumes.map((v) => [pathKey(v.mount, platform), v]))
  const out: { mount: string; label: string; free: number; total: number; drop: number }[] = []
  for (const v of cur.volumes) {
    const b = was.get(pathKey(v.mount, platform))
    if (!b || b.total === 0 || v.total === 0) continue
    const drop = b.free - v.free
    if (drop > minDrop) out.push({ mount: v.mount, label: v.mount.replace(SEP, '') || v.mount, free: v.free, total: v.total, drop })
  }
  return out.sort((a, b) => b.drop - a.drop)
}
