import { desktop } from '../../../core/lib/desktop'

export const INSTALL_TTL_MS = 10 * 60_000

export interface InstallFile {
  path: string
  name: string
  size: number
}

export interface InstallState {
  apk: InstallFile | null
  missing: boolean
  suggested: InstallFile | null
  url: string
  running: boolean
}

export interface InstallTicket {
  path: string
  exp: number
  port: number
  addresses: string[]
}

export type InstallError = 'off' | 'apk' | 'url' | 'rng' | 'invalid' | 'unavailable'

export type InstallReply<T> = { ok: true; value: T } | { ok: false; error: InstallError }

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const ERRORS: readonly InstallError[] = ['off', 'apk', 'url', 'rng', 'invalid', 'unavailable']

export const hasInstall = typeof desktop?.remoteInstall === 'function'

function parseFile(v: unknown): InstallFile | null {
  if (!isObj(v) || typeof v.path !== 'string' || typeof v.name !== 'string') return null
  return { path: v.path, name: v.name, size: typeof v.size === 'number' && v.size >= 0 ? v.size : 0 }
}

function parseError(v: unknown): InstallError {
  const e = isObj(v) ? v.error : undefined
  return ERRORS.includes(e as InstallError) ? (e as InstallError) : 'invalid'
}

export function parseState(v: unknown): InstallReply<InstallState> {
  if (!isObj(v) || v.ok !== true) return { ok: false, error: parseError(v) }
  return {
    ok: true,
    value: {
      apk: parseFile(v.apk),
      missing: v.missing === true,
      suggested: parseFile(v.suggested),
      url: typeof v.url === 'string' ? v.url : '',
      running: v.running === true,
    },
  }
}

const IPV4 = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/

export function parseTicket(v: unknown): InstallReply<InstallTicket> {
  if (!isObj(v) || v.ok !== true) return { ok: false, error: parseError(v) }
  const path = typeof v.path === 'string' && /^\/install\/[A-Za-z0-9_-]{1,64}\/orrery\.apk$/.test(v.path) ? v.path : ''
  const port = typeof v.port === 'number' && Number.isInteger(v.port) && v.port > 0 && v.port < 65536 ? v.port : 0
  const exp = typeof v.exp === 'number' && v.exp > 0 ? v.exp : 0
  const addresses = Array.isArray(v.addresses) ? v.addresses.filter((a): a is string => typeof a === 'string' && IPV4.test(a)) : []
  if (!path || !port || !exp || !addresses.length) return { ok: false, error: 'invalid' }
  return { ok: true, value: { path, port, exp, addresses } }
}

export const installUrl = (ip: string, port: number, path: string): string => `http://${ip}:${port}${path}`

export const ticketLeftMs = (exp: number, nowMs: number): number => Math.max(0, exp * 1000 - nowMs)

export function isHttpsUrl(text: string): boolean {
  const s = text.trim()
  if (!s || s.length > 2048 || !/^https:\/\//i.test(s) || /[^\x21-\x7e]/.test(s)) return false
  try {
    const u = new URL(s)
    return u.protocol === 'https:' && !!u.hostname && !u.username && !u.password
  } catch {
    return false
  }
}

export const relayAppUrl = (relayUrl: string | null | undefined): string => {
  const base = (relayUrl ?? '').trim().replace(/\/+$/, '')
  return isHttpsUrl(base) ? `${base}/app/` : ''
}

export function formatSize(bytes: number): string {
  if (bytes >= 1 << 20) return `${(bytes / (1 << 20)).toFixed(1)} MB`
  if (bytes >= 1 << 10) return `${Math.round(bytes / (1 << 10))} KB`
  return `${bytes} B`
}

async function call(op: { op: string; url?: string }): Promise<unknown> {
  if (!desktop?.remoteInstall) return { ok: false, error: 'unavailable' }
  return desktop.remoteInstall(op).catch(() => ({ ok: false, error: 'unavailable' }))
}

export const installGet = async (): Promise<InstallReply<InstallState>> => parseState(await call({ op: 'get' }))
export const installPick = async (): Promise<InstallReply<InstallState>> => parseState(await call({ op: 'pick' }))
export const installUseSuggested = async (): Promise<InstallReply<InstallState>> => parseState(await call({ op: 'suggested' }))
export const installClear = async (): Promise<InstallReply<InstallState>> => parseState(await call({ op: 'clear' }))
export const installSetUrl = async (url: string): Promise<InstallReply<InstallState>> => parseState(await call({ op: 'url', url }))
export const installStart = async (): Promise<InstallReply<InstallTicket>> => parseTicket(await call({ op: 'start' }))
export const installStop = async (): Promise<void> => {
  await call({ op: 'stop' })
}
