export type DiffKind = 'add' | 'del' | 'ctx' | 'hunk'

export interface DiffLine {
  kind: DiffKind
  text: string
  oldNo: number | null
  newNo: number | null
}

type Item = Record<string, unknown>

const isItem = (value: unknown): value is Item => !!value && typeof value === 'object' && !Array.isArray(value)
const str = (value: unknown): string | null => (typeof value === 'string' && value !== '' ? value : null)
const num = (value: unknown): number => (typeof value === 'number' && Number.isFinite(value) ? value : 0)
const items = (value: unknown): Item[] => (Array.isArray(value) ? value.filter(isItem) : [])

const HUNK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/

export function parseDiff(text: string): DiffLine[] {
  const out: DiffLine[] = []
  let oldNo = 0
  let newNo = 0
  let inHunk = false
  const body = text.endsWith('\n') ? text.slice(0, -1) : text
  for (const raw of body.split('\n')) {
    const line = raw.endsWith('\r') ? raw.slice(0, -1) : raw
    const match = HUNK.exec(line)
    if (match) {
      oldNo = Number(match[1])
      newNo = Number(match[2])
      inHunk = true
      out.push({ kind: 'hunk', text: line, oldNo: null, newNo: null })
      continue
    }
    if (!inHunk || line.startsWith('\\')) continue
    if (line.startsWith('+')) out.push({ kind: 'add', text: line.slice(1), oldNo: null, newNo: newNo++ })
    else if (line.startsWith('-')) out.push({ kind: 'del', text: line.slice(1), oldNo: oldNo++, newNo: null })
    else out.push({ kind: 'ctx', text: line === '' ? '' : line.slice(1), oldNo: oldNo++, newNo: newNo++ })
  }
  return out
}

export function diffCounts(lines: DiffLine[]): { added: number; removed: number } {
  let added = 0
  let removed = 0
  for (const line of lines) {
    if (line.kind === 'add') added++
    else if (line.kind === 'del') removed++
  }
  return { added, removed }
}

export interface DirEntry {
  name: string
  path: string
  directory: boolean
}

export function parseDir(raw: unknown): DirEntry[] {
  const rows = items(raw).flatMap((m) => {
    const name = str(m.name)
    const path = str(m.path)
    return name && path && m.outOfRoot !== true ? [{ name, path, directory: m.directory === true }] : []
  })
  return rows.sort((a, b) => (a.directory !== b.directory ? (a.directory ? -1 : 1) : a.name.toLowerCase().localeCompare(b.name.toLowerCase())))
}

export interface FileData {
  kind: string
  path: string
  size: number
  text: string | null
}

export function parseFile(raw: unknown): FileData {
  const m = isItem(raw) ? raw : {}
  return { kind: str(m.kind) ?? 'missing', path: str(m.path) ?? '', size: num(m.size), text: typeof m.text === 'string' ? m.text : null }
}

export interface ChangeEntry {
  path: string
  kind: string
  mark: string
  added: number | null
  removed: number | null
  binary: boolean
}

const MARKS: Record<string, string> = { added: 'A', deleted: 'D', renamed: 'R', untracked: '?', conflict: '!' }

export interface ChangesData {
  branch: string | null
  entries: ChangeEntry[]
}

export function parseChanges(raw: unknown): ChangesData {
  const m = isItem(raw) ? raw : {}
  const entries = items(m.entries).flatMap((e) => {
    const path = str(e.path)
    if (!path) return []
    const kind = str(e.kind) ?? 'modified'
    return [{ path, kind, mark: MARKS[kind] ?? 'M', added: typeof e.added === 'number' ? e.added : null, removed: typeof e.removed === 'number' ? e.removed : null, binary: e.binary === true }]
  })
  return { branch: str(m.branch), entries }
}

export interface LogCommit {
  hash: string
  short: string
  author: string
  time: number
  subject: string
  refs: string[]
}

export interface LogData {
  commits: LogCommit[]
  more: boolean
}

export function parseLog(raw: unknown): LogData {
  const m = isItem(raw) ? raw : {}
  const commits = items(m.commits).flatMap((c) => {
    const hash = str(c.hash)
    if (!hash) return []
    const refs = (str(c.refs) ?? '')
      .split(',')
      .map((r) => r.trim())
      .filter(Boolean)
    return [{ hash, short: hash.slice(0, 7), author: str(c.author) ?? '', time: num(c.time), subject: str(c.subject) ?? '', refs }]
  })
  return { commits, more: m.more === true }
}

export interface RunSession {
  id: string
  name: string
  state: string
  cwd: string
  progress: string | null
  live: boolean
}

const LIVE = ['before-launch', 'building', 'installing', 'launching', 'starting', 'running', 'reloading', 'stopping']

export function parseRunSession(raw: unknown): RunSession | null {
  if (!isItem(raw)) return null
  const id = str(raw.id)
  if (!id) return null
  const state = str(raw.state) ?? 'stopped'
  return { id, name: str(raw.configName) ?? '', state, cwd: str(raw.cwd) ?? '', progress: str(raw.progress), live: LIVE.includes(state) }
}

export function parseRunSessions(raw: unknown): RunSession[] {
  return Array.isArray(raw) ? raw.flatMap((r) => parseRunSession(r) ?? []) : []
}

export interface RunLine {
  text: string
  level: string | null
}

export function parseRunLines(raw: unknown): RunLine[] {
  return items(raw).map((m) => ({ text: str(m.text) ?? '', level: str(m.level) }))
}

export function sizeText(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}

export function joinPath(dir: string, name: string): string {
  const windows = dir.includes('\\') && !dir.includes('/')
  const trimmed = dir.replace(/[\\/]+$/, '')
  return `${trimmed}${windows ? '\\' : '/'}${windows ? name.replace(/\//g, '\\') : name}`
}

export function samePath(a: string, b: string): boolean {
  const norm = (p: string): string => p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
  return norm(a) === norm(b)
}

export function relativePath(root: string, path: string): string {
  const r = root.replace(/\\/g, '/').replace(/\/+$/, '')
  const p = path.replace(/\\/g, '/')
  return p.toLowerCase().startsWith(`${r.toLowerCase()}/`) ? p.slice(r.length + 1) : p
}
