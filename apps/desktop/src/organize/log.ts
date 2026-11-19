import { promises as fs } from 'node:fs'
import path from 'node:path'
import { randomBytes } from 'node:crypto'
import type { OrganizeMode, OrganizeRunSummary } from '@ccm/shared'

export interface RunHeader {
  id: string
  at: number
  source: string
  mode: OrganizeMode
  auto: boolean
}

export interface RunMove {
  from: string
  to: string
  size?: number
  mtime?: number
  done: boolean
}

export interface RunRecord {
  header: RunHeader
  dirs: string[]
  moves: RunMove[]
  backs: Set<string>
  undo?: { at: number; restored: number; conflicts: number }
}

const ID = /^[a-z0-9]{4,16}-[a-f0-9]{8}$/
const KEEP_RUNS = 200

export const isRunId = (v: unknown): v is string => typeof v === 'string' && ID.test(v)
export const newRunId = (now: number): string => `${now.toString(36)}-${randomBytes(4).toString('hex')}`

const num = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : undefined)
const str = (v: unknown): string | null => (typeof v === 'string' && v && !v.includes('\0') ? v : null)

export function parseRun(text: string): RunRecord | null {
  let header: RunHeader | null = null
  const dirs: string[] = []
  const moves: RunMove[] = []
  const byTo = new Map<string, RunMove>()
  const backs = new Set<string>()
  let undo: RunRecord['undo']
  for (const line of text.split('\n')) {
    if (!line.trim()) continue
    let o: Record<string, unknown>
    try {
      o = JSON.parse(line) as Record<string, unknown>
    } catch {
      continue
    }
    if (!o || typeof o !== 'object') continue
    if (o.t === 'run' && !header) {
      const source = str(o.source)
      if (!isRunId(o.id) || !source || num(o.at) === undefined) return null
      header = { id: o.id, at: num(o.at)!, source, mode: o.mode === 'month' ? 'month' : 'type', auto: o.auto === true }
      continue
    }
    if (!header) return null
    const from = str(o.from)
    const to = str(o.to)
    if (o.t === 'dir') {
      const p = str(o.path)
      if (p && !dirs.includes(p)) dirs.push(p)
    } else if (o.t === 'intent' && from && to) {
      const m: RunMove = { from, to, done: false }
      moves.push(m)
      byTo.set(to, m)
    } else if (o.t === 'move' && from && to) {
      const m = byTo.get(to) ?? { from, to, done: false }
      if (!byTo.has(to)) {
        moves.push(m)
        byTo.set(to, m)
      }
      m.done = true
      m.size = num(o.size)
      m.mtime = num(o.mtime)
    } else if (o.t === 'fail' && to) {
      const m = byTo.get(to)
      if (m && !m.done) {
        moves.splice(moves.indexOf(m), 1)
        byTo.delete(to)
      }
    } else if (o.t === 'back' && to) {
      backs.add(to)
    } else if (o.t === 'undo') {
      undo = { at: num(o.at) ?? 0, restored: num(o.restored) ?? 0, conflicts: num(o.conflicts) ?? 0 }
    }
  }
  return header ? { header, dirs, moves, backs, undo } : null
}

export function summarize(r: RunRecord): OrganizeRunSummary {
  const done = r.moves.filter((m) => m.done)
  return {
    id: r.header.id,
    at: r.header.at,
    source: r.header.source,
    mode: r.header.mode,
    auto: r.header.auto,
    moved: done.length,
    bytes: done.reduce((s, m) => s + (m.size ?? 0), 0),
    undone: !!r.undo,
    ...(r.undo ? { undoneAt: r.undo.at, restored: r.undo.restored, conflicts: r.undo.conflicts } : {}),
  }
}

export class RunWriter {
  private handle: fs.FileHandle | null = null
  private opening: Promise<fs.FileHandle> | null = null

  constructor(
    readonly file: string,
    private readonly header: RunHeader | null,
  ) {}

  private async open(): Promise<fs.FileHandle> {
    if (this.handle) return this.handle
    this.opening ??= (async () => {
      await fs.mkdir(path.dirname(this.file), { recursive: true })
      const h = await fs.open(this.file, 'a')
      if (this.header) await h.appendFile(`${JSON.stringify({ t: 'run', ...this.header })}\n`, 'utf8')
      this.handle = h
      return h
    })()
    return this.opening
  }

  async write(entry: Record<string, unknown>, sync = false): Promise<void> {
    const h = await this.open()
    await h.appendFile(`${JSON.stringify(entry)}\n`, 'utf8')
    if (sync) await h.datasync().catch(() => undefined)
  }

  async close(): Promise<void> {
    const h = this.handle ?? (this.opening ? await this.opening.catch(() => null) : null)
    this.handle = null
    this.opening = null
    await h?.close().catch(() => undefined)
  }
}

export class RunStore {
  constructor(readonly dir: string) {}

  fileOf(id: string): string {
    return path.join(this.dir, `${id}.jsonl`)
  }

  writer(header: RunHeader): RunWriter {
    return new RunWriter(this.fileOf(header.id), header)
  }

  appender(id: string): RunWriter {
    return new RunWriter(this.fileOf(id), null)
  }

  async read(id: string): Promise<RunRecord | null> {
    if (!isRunId(id)) return null
    try {
      return parseRun(await fs.readFile(this.fileOf(id), 'utf8'))
    } catch {
      return null
    }
  }

  async list(limit = 50): Promise<OrganizeRunSummary[]> {
    let names: string[]
    try {
      names = (await fs.readdir(this.dir)).filter((n) => n.endsWith('.jsonl') && isRunId(n.slice(0, -6)))
    } catch {
      return []
    }
    const out: OrganizeRunSummary[] = []
    for (const n of names) {
      const r = await this.read(n.slice(0, -6))
      if (r && r.moves.length) out.push(summarize(r))
    }
    out.sort((a, b) => b.at - a.at)
    for (const old of out.slice(KEEP_RUNS)) await fs.unlink(this.fileOf(old.id)).catch(() => undefined)
    return out.slice(0, limit)
  }
}
