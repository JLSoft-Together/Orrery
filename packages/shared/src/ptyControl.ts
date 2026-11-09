import { isPtyPreset, type PtyClientMessage, type PtyServerMessage, type PtySessionInfo } from './pty'

export type PtySpawnRequest = Extract<PtyClientMessage, { t: 'spawn' }>
export type PtyWorktreeRequest = Extract<PtyClientMessage, { t: 'wt.list' | 'wt.add' | 'wt.remove' }>
export type PtyWorktreeReply = Extract<PtyServerMessage, { t: 'wt.list' | 'wt.added' | 'wt.removed' | 'wt.error' }>
export type PtySpawnError = 'preset' | 'cwd' | 'limit' | 'spawn'

export const PTY_SPAWN_ERRORS: readonly PtySpawnError[] = ['preset', 'cwd', 'limit', 'spawn']
export const PTY_CONTROL_LINE_MAX = 1024 * 1024
export const PTY_RESOLVE_TIMEOUT_MS = 15_000

export type PtyResolved =
  | { t: 'resolved'; rid: string; ok: true; id: string; file: string; args: string[]; env: Record<string, string>; cwd: string; agentSession?: string; si?: string }
  | { t: 'resolved'; rid: string; ok: false; message: PtySpawnError }

export type PtyMainToCore =
  | { t: 'init'; token: string; origins: string[]; appVersion: string }
  | PtyResolved
  | { t: 'cwd-ok'; rid: string; cwd: string | null }
  | { t: 'wt-reply'; rid: string; reply: PtyWorktreeReply }
  | { t: 'branch'; id: string; branch: string | null }
  | { t: 'census' }
  | { t: 'kill-all' }
  | { t: 'shutdown' }

export type PtyCoreToMain =
  | { t: 'listening'; port: number }
  | { t: 'resolve'; rid: string; msg: PtySpawnRequest }
  | { t: 'spawned'; id: string; pid: number }
  | { t: 'pid'; id: string; pid: number }
  | { t: 'exited'; id: string; code: number }
  | { t: 'removed'; id: string }
  | { t: 'cwd'; rid: string; id: string; cwd: string }
  | { t: 'wt'; rid: string; msg: PtyWorktreeRequest }
  | { t: 'census-reply'; sessions: PtySessionInfo[] }

const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)
const isStr = (v: unknown): v is string => typeof v === 'string'
const isId = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && v.length <= 128
const isInt = (v: unknown): v is number => typeof v === 'number' && Number.isInteger(v)
const isStrArr = (v: unknown): v is string[] => Array.isArray(v) && v.every(isStr)
const isEnv = (v: unknown): v is Record<string, string> => isObj(v) && Object.values(v).every(isStr)

export function parseCoreToMain(raw: unknown): PtyCoreToMain | null {
  if (!isObj(raw)) return null
  switch (raw.t) {
    case 'listening':
      return isInt(raw.port) && raw.port > 0 && raw.port < 65536 ? { t: 'listening', port: raw.port } : null
    case 'resolve':
      return isId(raw.rid) && isObj(raw.msg) && raw.msg.t === 'spawn' ? { t: 'resolve', rid: raw.rid, msg: raw.msg as PtySpawnRequest } : null
    case 'spawned':
    case 'pid':
      return isId(raw.id) && isInt(raw.pid) && raw.pid >= 0 ? { t: raw.t, id: raw.id, pid: raw.pid } : null
    case 'exited':
      return isId(raw.id) && isInt(raw.code) ? { t: 'exited', id: raw.id, code: raw.code } : null
    case 'removed':
      return isId(raw.id) ? { t: 'removed', id: raw.id } : null
    case 'cwd':
      return isId(raw.rid) && isId(raw.id) && isStr(raw.cwd) ? { t: 'cwd', rid: raw.rid, id: raw.id, cwd: raw.cwd } : null
    case 'wt':
      return isId(raw.rid) && isObj(raw.msg) && (raw.msg.t === 'wt.list' || raw.msg.t === 'wt.add' || raw.msg.t === 'wt.remove') ? { t: 'wt', rid: raw.rid, msg: raw.msg as PtyWorktreeRequest } : null
    case 'census-reply':
      return Array.isArray(raw.sessions) && raw.sessions.every(isSessionInfo) ? { t: 'census-reply', sessions: raw.sessions } : null
    default:
      return null
  }
}

export function invalidCensusRows(raw: unknown): number | null {
  if (!isObj(raw) || raw.t !== 'census-reply') return null
  return Array.isArray(raw.sessions) ? raw.sessions.filter((s) => !isSessionInfo(s)).length : -1
}

function isSessionInfo(v: unknown): v is PtySessionInfo {
  return isObj(v) && isId(v.id) && isPtyPreset(v.preset) && isStr(v.cwd) && (v.liveCwd === undefined || isStr(v.liveCwd)) && isInt(v.pid) && typeof v.alive === 'boolean'
}

export function parseMainToCore(raw: unknown): PtyMainToCore | null {
  if (!isObj(raw)) return null
  switch (raw.t) {
    case 'init':
      return isStr(raw.token) && isStrArr(raw.origins) && isStr(raw.appVersion) ? { t: 'init', token: raw.token, origins: raw.origins, appVersion: raw.appVersion } : null
    case 'resolved': {
      if (!isId(raw.rid)) return null
      if (raw.ok === false) return (PTY_SPAWN_ERRORS as readonly unknown[]).includes(raw.message) ? { t: 'resolved', rid: raw.rid, ok: false, message: raw.message as PtySpawnError } : null
      if (raw.ok !== true || !isId(raw.id) || !isStr(raw.file) || !isStrArr(raw.args) || !isEnv(raw.env) || !isStr(raw.cwd)) return null
      if ((raw.agentSession !== undefined && !isStr(raw.agentSession)) || (raw.si !== undefined && !isStr(raw.si))) return null
      return {
        t: 'resolved',
        rid: raw.rid,
        ok: true,
        id: raw.id,
        file: raw.file,
        args: raw.args,
        env: raw.env,
        cwd: raw.cwd,
        ...(raw.agentSession !== undefined ? { agentSession: raw.agentSession as string } : {}),
        ...(raw.si !== undefined ? { si: raw.si as string } : {}),
      }
    }
    case 'cwd-ok':
      return isId(raw.rid) && (raw.cwd === null || isStr(raw.cwd)) ? { t: 'cwd-ok', rid: raw.rid, cwd: raw.cwd } : null
    case 'wt-reply':
      return isId(raw.rid) && isObj(raw.reply) && isStr(raw.reply.t) && raw.reply.t.startsWith('wt.') ? { t: 'wt-reply', rid: raw.rid, reply: raw.reply as PtyWorktreeReply } : null
    case 'branch':
      return isId(raw.id) && (raw.branch === null || isStr(raw.branch)) ? { t: 'branch', id: raw.id, branch: raw.branch } : null
    case 'census':
    case 'kill-all':
    case 'shutdown':
      return { t: raw.t }
    default:
      return null
  }
}

export const encodeControl = (msg: PtyMainToCore | PtyCoreToMain | { t: string }): string => `${JSON.stringify(msg)}\n`

export function utf8Bytes(text: string): number {
  let n = 0
  for (let i = 0; i < text.length; i++) {
    const c = text.charCodeAt(i)
    if (c < 0x80) n += 1
    else if (c < 0x800) n += 2
    else if (c >= 0xd800 && c <= 0xdbff && i + 1 < text.length && (text.charCodeAt(i + 1) & 0xfc00) === 0xdc00) {
      n += 4
      i++
    } else n += 3
  }
  return n
}

export const PTY_QUEUE_MAX = 4 * 1024 * 1024

export class PtyFrameQueue<T> {
  private items: T[] = []
  private bytes = 0
  dropped = 0

  constructor(private readonly max = PTY_QUEUE_MAX) {}

  push(item: T, size: number, droppable: boolean): boolean {
    if (droppable && this.bytes + size > this.max) {
      this.dropped++
      return false
    }
    this.items.push(item)
    this.bytes += size
    return true
  }

  take(): T[] {
    const out = this.items
    this.items = []
    this.bytes = 0
    return out
  }
}

export class NdjsonReader {
  private buf = ''
  private bufBytes = 0
  private skipping = false

  constructor(
    private readonly onLine: (value: unknown) => void,
    private readonly max = PTY_CONTROL_LINE_MAX,
  ) {}

  push(chunk: string): void {
    let start = 0
    for (let nl = chunk.indexOf('\n'); nl >= 0; nl = chunk.indexOf('\n', start)) {
      const part = chunk.slice(start, nl)
      start = nl + 1
      const partBytes = utf8Bytes(part)
      if (this.skipping || this.bufBytes + partBytes > this.max) {
        this.buf = ''
        this.bufBytes = 0
        this.skipping = false
        continue
      }
      const line = (this.buf + part).replace(/\r$/, '')
      this.buf = ''
      this.bufBytes = 0
      if (!line.trim()) continue
      let value: unknown
      try {
        value = JSON.parse(line)
      } catch {
        continue
      }
      this.onLine(value)
    }
    const rest = chunk.slice(start)
    if (this.skipping) return
    const restBytes = utf8Bytes(rest)
    if (this.bufBytes + restBytes > this.max) {
      this.buf = ''
      this.bufBytes = 0
      this.skipping = true
    } else {
      this.buf += rest
      this.bufBytes += restBytes
    }
  }
}
