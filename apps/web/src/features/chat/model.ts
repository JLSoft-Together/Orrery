import { REMOTE_CHAT_SYS_KINDS, REMOTE_CHAT_TOOL_CATS, type RemoteChatAgent, type RemoteChatFile, type RemoteChatItem, type RemoteChatSysKind, type RemoteChatTool, type RemoteChatToolCat } from '@ccm/shared'

export type ChatItem = RemoteChatItem
export type ToolCat = RemoteChatToolCat

export const CHAT_WINDOW = 60
export const CHAT_STEP = 30
export const CHAT_POLL_MS = 1500
export const CHAT_NEAR_TOP = 120
export const CHAT_NEAR_BOTTOM = 48

export interface ChatLog {
  gen: string | null
  agent: RemoteChatAgent | null
  items: ReadonlyMap<string, ChatItem>
  older: number | null
}

export interface ChatPageData {
  gen: string
  agent: RemoteChatAgent | null
  items: ChatItem[]
  older: number | null
}

export type ChatPoll = { reset: true } | { reset: false; gen: string; items: ChatItem[] }

type Obj = Record<string, unknown>

const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v)
const text = (v: unknown, max: number): string | null => (typeof v === 'string' ? v.slice(0, max) : null)
const count = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? Math.floor(v) : undefined)

export function emptyLog(): ChatLog {
  return { gen: null, agent: null, items: new Map(), older: null }
}

export function toolCat(v: unknown): ToolCat {
  return typeof v === 'string' && (REMOTE_CHAT_TOOL_CATS as readonly string[]).includes(v) ? (v as ToolCat) : 'other'
}

function parseTool(raw: unknown): RemoteChatTool | null {
  if (!isObj(raw)) return null
  const tool = text(raw.tool, 40)
  if (!tool) return null
  const target = text(raw.target, 120)
  return { tool, cat: toolCat(raw.cat), ...(target ? { target } : {}), ...(raw.err === true ? { err: true as const } : {}) }
}

function parseFile(raw: unknown): RemoteChatFile | null {
  if (!isObj(raw)) return null
  const path = text(raw.path, 300)
  if (!path) return null
  const add = count(raw.add)
  const del = count(raw.del)
  return { path, ...(add !== undefined ? { add } : {}), ...(del !== undefined ? { del } : {}), ...(raw.out === true ? { out: true as const } : {}) }
}

function parseSum(raw: unknown): Partial<Record<ToolCat, number>> {
  const out: Partial<Record<ToolCat, number>> = {}
  if (!isObj(raw)) return out
  for (const [key, value] of Object.entries(raw)) {
    const n = count(value)
    if (!n) continue
    const cat = toolCat(key)
    out[cat] = (out[cat] ?? 0) + n
  }
  return out
}

export function parseChatItem(raw: unknown): ChatItem | null {
  if (!isObj(raw)) return null
  const id = text(raw.id, 64)
  const seq = count(raw.seq)
  if (!id || seq === undefined) return null
  const at = count(raw.at)
  const base = { id, seq, rev: count(raw.rev) ?? 0, ...(at !== undefined ? { at } : {}) }
  switch (raw.k) {
    case 'me':
    case 'ai': {
      const body = text(raw.text, raw.k === 'me' ? 4000 : 8000)
      if (body === null) return null
      return { ...base, k: raw.k, text: body, ...(raw.cut === true ? { cut: true as const } : {}) }
    }
    case 'tools': {
      const items = Array.isArray(raw.items) ? raw.items.flatMap((x) => parseTool(x) ?? []) : []
      const more = count(raw.more)
      return { ...base, k: 'tools', sum: parseSum(raw.sum), items, ...(more ? { more } : {}) }
    }
    case 'files': {
      const files = Array.isArray(raw.files) ? raw.files.flatMap((x) => parseFile(x) ?? []) : []
      const more = count(raw.more)
      return { ...base, k: 'files', root: text(raw.root, 1000) ?? '', files, ...(more ? { more } : {}) }
    }
    case 'sys': {
      const kind = typeof raw.kind === 'string' && (REMOTE_CHAT_SYS_KINDS as readonly string[]).includes(raw.kind) ? (raw.kind as RemoteChatSysKind) : 'error'
      const body = text(raw.text, 300)
      return { ...base, k: 'sys', kind, ...(body ? { text: body } : {}) }
    }
    default:
      return null
  }
}

export function parseChatItems(raw: unknown): ChatItem[] {
  return Array.isArray(raw) ? raw.flatMap((x) => parseChatItem(x) ?? []) : []
}

export function parseChatPage(raw: unknown): ChatPageData | null {
  if (!isObj(raw) || raw.ok !== true || typeof raw.gen !== 'string') return null
  const agent = raw.agent === 'claude' || raw.agent === 'codex' ? raw.agent : null
  return { gen: raw.gen, agent, items: parseChatItems(raw.items), older: count(raw.older) ?? null }
}

export function parseChatPoll(raw: unknown): ChatPoll | null {
  if (!isObj(raw) || raw.ok !== true) return null
  if (raw.reset === true) return { reset: true }
  if (typeof raw.gen !== 'string') return null
  return { reset: false, gen: raw.gen, items: parseChatItems(raw.items) }
}

function upsertAll(into: Map<string, ChatItem>, items: readonly ChatItem[]): void {
  for (const item of items) {
    const prev = into.get(item.id)
    if (!prev || item.rev >= prev.rev) into.set(item.id, item)
  }
}

export function applyPage(log: ChatLog, page: ChatPageData, older: boolean): ChatLog {
  if (page.gen !== log.gen) {
    const items = new Map<string, ChatItem>()
    upsertAll(items, page.items)
    return { gen: page.gen, agent: page.agent ?? log.agent, items, older: page.older }
  }
  const items = new Map(log.items)
  upsertAll(items, page.items)
  const keepOlder = !older && log.items.size > 0
  return { gen: log.gen, agent: page.agent ?? log.agent, items, older: keepOlder ? log.older : page.older }
}

export function applyItems(log: ChatLog, gen: string, items: readonly ChatItem[]): { log: ChatLog; stale: boolean } {
  if (log.gen === null || gen !== log.gen) return { log, stale: true }
  if (items.length === 0) return { log, stale: false }
  const next = new Map(log.items)
  upsertAll(next, items)
  return { log: { ...log, items: next }, stale: false }
}

export function chatView(log: ChatLog): ChatItem[] {
  const sorted = [...log.items.values()].sort((a, b) => a.seq - b.seq || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
  const out: ChatItem[] = []
  let held: ChatItem[] = []
  for (const item of sorted) {
    if (item.k === 'files') {
      held.push(item)
      continue
    }
    if (item.k === 'me') {
      out.push(...held)
      held = []
    }
    out.push(item)
  }
  out.push(...held)
  return out
}

export function toolParts(sum: Partial<Record<ToolCat, number>>): Array<{ cat: ToolCat; n: number }> {
  return REMOTE_CHAT_TOOL_CATS.flatMap((cat) => {
    const n = sum[cat] ?? 0
    return n > 0 ? [{ cat, n }] : []
  })
}

export function fileTotals(files: readonly RemoteChatFile[]): { add: number; del: number } {
  return files.reduce((acc, f) => ({ add: acc.add + (f.add ?? 0), del: acc.del + (f.del ?? 0) }), { add: 0, del: 0 })
}

export function splitPath(path: string): { dir: string; name: string } {
  const at = path.lastIndexOf('/')
  return at < 0 ? { dir: '', name: path } : { dir: path.slice(0, at + 1), name: path.slice(at + 1) }
}

export function joinRoot(root: string, path: string): string {
  if (!root) return path
  const sep = root.includes('\\') && !root.includes('/') ? '\\' : '/'
  const clean = root.replace(/[\\/]+$/, '')
  return `${clean}${sep}${sep === '\\' ? path.replace(/\//g, '\\') : path}`
}

export function windowOf<T>(items: readonly T[], shown: number): T[] {
  const n = Math.max(0, Math.min(items.length, shown))
  return items.slice(items.length - n)
}

export type TopAction = 'none' | 'grow' | 'fetch'

export function topAction(total: number, shown: number, older: number | null, loading: boolean): TopAction {
  if (shown < total) return 'grow'
  return older !== null && !loading ? 'fetch' : 'none'
}

export function anchorTop(prevTop: number, prevHeight: number, nextHeight: number): number {
  return Math.max(0, prevTop + (nextHeight - prevHeight))
}

export function nearBottom(top: number, height: number, client: number): boolean {
  return height - top - client <= CHAT_NEAR_BOTTOM
}
