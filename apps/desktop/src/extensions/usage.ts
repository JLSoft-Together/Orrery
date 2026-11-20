import { closeSync, openSync, readdirSync, readSync, statSync } from 'node:fs'
import path from 'node:path'
import type { ExtSessionUsage } from '@ccm/shared'

const SESSION_RE = /^[0-9A-Za-z][0-9A-Za-z-]{7,63}$/
const KEY_RE = /^[A-Za-z0-9_.-]{1,200}$/
const NS_RE = /^[A-Za-z0-9_.-]{1,100}$/
const CACHE_MAX = 64
const CHUNK = 1 << 20

interface Tally {
  offset: number
  rest: Buffer
  ids: Set<string>
  total: number
  servers: Map<string, number>
  namespaces: Map<string, number>
}

const fresh = (): Tally => ({ offset: 0, rest: Buffer.alloc(0), ids: new Set(), total: 0, servers: new Map(), namespaces: new Map() })

const bump = (m: Map<string, number>, k: string): void => void m.set(k, (m.get(k) ?? 0) + 1)

export const validSession = (v: unknown): v is string => typeof v === 'string' && SESSION_RE.test(v)

export function serverOf(tool: string): string | null {
  if (!tool.startsWith('mcp__')) return null
  const end = tool.indexOf('__', 5)
  const key = end < 0 ? '' : tool.slice(5, end)
  return KEY_RE.test(key) ? key : null
}

function namespaceOf(tool: string, input: unknown): string | null {
  if (!input || typeof input !== 'object') return null
  const o = input as Record<string, unknown>
  const raw = tool === 'Skill' ? o.skill : tool === 'Agent' || tool === 'Task' ? o.subagent_type : undefined
  if (typeof raw !== 'string') return null
  const at = raw.indexOf(':')
  if (at <= 0) return null
  const ns = raw.slice(0, at)
  return NS_RE.test(ns) ? ns : null
}

function scanLine(t: Tally, line: string): void {
  if (!line.includes('"tool_use"')) return
  let entry: unknown
  try {
    entry = JSON.parse(line)
  } catch {
    return
  }
  const content = (entry as { message?: { content?: unknown } } | null)?.message?.content
  if (!Array.isArray(content)) return
  for (const block of content) {
    if (!block || typeof block !== 'object') continue
    const b = block as Record<string, unknown>
    if (b.type !== 'tool_use' || typeof b.id !== 'string' || typeof b.name !== 'string' || t.ids.has(b.id)) continue
    t.ids.add(b.id)
    t.total++
    const server = serverOf(b.name)
    if (server) bump(t.servers, server)
    const ns = namespaceOf(b.name, b.input)
    if (ns) bump(t.namespaces, ns)
  }
}

export class SessionUsage {
  private files = new Map<string, Tally>()
  private where = new Map<string, string>()

  constructor(private readonly root: () => string) {}

  private locate(sessionId: string): string | null {
    const known = this.where.get(sessionId)
    if (known) {
      try {
        if (statSync(known).isFile()) return known
      } catch {
        this.where.delete(sessionId)
      }
    }
    const projects = path.join(this.root(), 'projects')
    let dirs: string[]
    try {
      dirs = readdirSync(projects)
    } catch {
      return null
    }
    for (const d of dirs) {
      const file = path.join(projects, d, `${sessionId}.jsonl`)
      try {
        if (statSync(file).isFile()) {
          this.where.set(sessionId, file)
          return file
        }
      } catch {
        void 0
      }
    }
    return null
  }

  private update(file: string): Tally | null {
    let size: number
    try {
      size = statSync(file).size
    } catch {
      this.files.delete(file)
      return null
    }
    let t = this.files.get(file)
    if (!t || size < t.offset) t = fresh()
    this.files.delete(file)
    this.files.set(file, t)
    if (this.files.size > CACHE_MAX) this.files.delete(this.files.keys().next().value!)
    if (size === t.offset) return t
    let fd: number
    try {
      fd = openSync(file, 'r')
    } catch {
      return t
    }
    try {
      const buf = Buffer.alloc(CHUNK)
      while (t.offset < size) {
        const n = readSync(fd, buf, 0, Math.min(CHUNK, size - t.offset), t.offset)
        if (n <= 0) break
        t.offset += n
        const data = t.rest.length ? Buffer.concat([t.rest, buf.subarray(0, n)]) : buf.subarray(0, n)
        const last = data.lastIndexOf(10)
        if (last < 0) {
          t.rest = Buffer.from(data)
          continue
        }
        for (const line of data.subarray(0, last).toString('utf8').split('\n')) scanLine(t, line)
        t.rest = Buffer.from(data.subarray(last + 1))
      }
    } finally {
      closeSync(fd)
    }
    return t
  }

  read(sessionId: unknown): ExtSessionUsage {
    if (!validSession(sessionId)) return { ok: false, code: 'badArg' }
    const main = this.locate(sessionId)
    if (!main) return { ok: false, code: 'missing' }
    const files = [main]
    const subDir = path.join(path.dirname(main), sessionId, 'subagents')
    try {
      for (const f of readdirSync(subDir).sort()) if (f.endsWith('.jsonl')) files.push(path.join(subDir, f))
    } catch {
      void 0
    }
    let total = 0
    const servers: Record<string, number> = {}
    const namespaces: Record<string, number> = {}
    for (const f of files) {
      const t = this.update(f)
      if (!t) continue
      total += t.total
      for (const [k, v] of t.servers) servers[k] = (servers[k] ?? 0) + v
      for (const [k, v] of t.namespaces) namespaces[k] = (namespaces[k] ?? 0) + v
    }
    return { ok: true, total, servers, namespaces }
  }
}
