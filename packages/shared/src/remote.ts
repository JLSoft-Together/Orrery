import { isAgentSessionId } from './pty'
import type { LayoutNodeLike } from './remoteLayout'

export const REMOTE_PROTO = 1
export const REMOTE_MIN_PROTO = 1
export const REMOTE_PORT = 4320
export const REMOTE_TEXT_MAX = 2000
export const REMOTE_JSON_MAX = 1024 * 1024
export const REMOTE_ATTACH_MAX = 4
export const REMOTE_CONFIRM_MS = 60_000
export const REMOTE_SUMMARY_DEBOUNCE_MS = 200

export type RemoteErrorCode =
  | 'denied'
  | 'busy'
  | 'limit'
  | 'notFound'
  | 'invalid'
  | 'timeout'
  | 'offline'
  | 'rate'
  | 'proto'
  | 'internal'

export type RemotePreset = 'claude' | 'codex' | 'shell'

export interface RemotePane {
  key: string
  kind: string
  preset?: string
  agent?: string
  title: string
  cwd?: string
  branch?: string
  state: string
  rail: string
  waitText?: string
  attention?: 'waiting' | 'done'
  subagents: number
  sessionId?: string
  terminalId?: string
  dirty: boolean
  busy: boolean
}

export interface RemoteTab {
  key: string
  name?: string
  project?: string
  tree: LayoutNodeLike
  zoom?: string
  folded?: boolean
  panes: RemotePane[]
}

export interface RemoteWorkspaceSnapshot {
  rev: number
  tabs: RemoteTab[]
  closedCount: number
}

export interface RemotePatchOp {
  op: 'add' | 'remove' | 'replace'
  path: string
  value?: unknown
}

export interface RemoteWorkspacePatch {
  rev: number
  ops: RemotePatchOp[]
}

export interface RemoteConfirmAsk {
  cid: string
  title: string
  detail?: string
  ok: string
  cancel: string
}

export type RemoteCommand =
  | { t: 'ws.get' }
  | { t: 'ws.open'; cwd: string; preset: RemotePreset; prompt?: string; submit?: boolean; resume?: string }
  | { t: 'ws.close'; paneKey: string }
  | { t: 'ws.rename'; tabKey: string; name: string | null }
  | { t: 'ws.move'; paneKey: string; tabKey: string }
  | { t: 'ws.reopen' }
  | { t: 'ws.compact'; paneKey: string }
  | { t: 'ws.send'; paneKey: string; text: string }
  | { t: 'ws.keys'; paneKey: string; keys: string; since: number }

export type RemoteCommandName = RemoteCommand['t']

export type RemoteKeyPress = { key: 'approve' | 'deny' | 'stop' | 'esc' } | { key: 'option'; option: number }

export interface RemotePhoneCall<T extends RemoteCommandName = RemoteCommandName> {
  device: string
  cmd: Extract<RemoteCommand, { t: T }>
  confirmed?: boolean
}

export interface RemoteNeedsConfirm {
  title: string
  detail?: string
  ok: string
  cancel: string
}

export interface RemotePairStart {
  qr: string
  exp: number
}

const DEVICE_MAX = 100
const KEY_ID_MAX = 200
const PATH_MAX = 4096
const NAME_MAX = 100

const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)
const str = (v: unknown, max: number): v is string => typeof v === 'string' && v.length <= max && !v.includes('\0')
const filled = (v: unknown, max: number): v is string => str(v, max) && v.length > 0
const ident = (v: unknown, max: number): v is string => filled(v, max) && /^[A-Za-z0-9_.:-]+$/.test(v)

export function parseRemoteKeys(v: unknown): RemoteKeyPress | null {
  if (v === 'approve' || v === 'deny' || v === 'stop' || v === 'esc') return { key: v }
  if (typeof v === 'string' && /^[1-9]$/.test(v)) return { key: 'option', option: Number(v) }
  return null
}

export function parseRemoteCommand(raw: unknown, name: RemoteCommandName): RemoteCommand | null {
  if (!isObj(raw) || raw.t !== name) return null
  switch (name) {
    case 'ws.get':
      return { t: 'ws.get' }
    case 'ws.open': {
      if (!filled(raw.cwd, PATH_MAX) || /[\r\n]/.test(raw.cwd)) return null
      if (raw.preset !== 'claude' && raw.preset !== 'codex' && raw.preset !== 'shell') return null
      if (raw.prompt !== undefined && !str(raw.prompt, REMOTE_TEXT_MAX)) return null
      if (raw.submit !== undefined && typeof raw.submit !== 'boolean') return null
      if (raw.resume !== undefined && !isAgentSessionId(raw.resume)) return null
      return {
        t: 'ws.open',
        cwd: raw.cwd,
        preset: raw.preset,
        ...(raw.prompt === undefined ? {} : { prompt: raw.prompt }),
        ...(raw.submit === undefined ? {} : { submit: raw.submit }),
        ...(raw.resume === undefined ? {} : { resume: raw.resume }),
      }
    }
    case 'ws.close':
    case 'ws.compact':
      return ident(raw.paneKey, KEY_ID_MAX) ? { t: name, paneKey: raw.paneKey } : null
    case 'ws.rename': {
      if (!ident(raw.tabKey, KEY_ID_MAX)) return null
      if (raw.name === null) return { t: 'ws.rename', tabKey: raw.tabKey, name: null }
      return str(raw.name, NAME_MAX) ? { t: 'ws.rename', tabKey: raw.tabKey, name: raw.name } : null
    }
    case 'ws.move':
      return ident(raw.paneKey, KEY_ID_MAX) && ident(raw.tabKey, KEY_ID_MAX) ? { t: 'ws.move', paneKey: raw.paneKey, tabKey: raw.tabKey } : null
    case 'ws.reopen':
      return { t: 'ws.reopen' }
    case 'ws.send':
      return ident(raw.paneKey, KEY_ID_MAX) && filled(raw.text, REMOTE_TEXT_MAX) ? { t: 'ws.send', paneKey: raw.paneKey, text: raw.text } : null
    case 'ws.keys':
      return ident(raw.paneKey, KEY_ID_MAX) && parseRemoteKeys(raw.keys) && typeof raw.since === 'number' && Number.isFinite(raw.since)
        ? { t: 'ws.keys', paneKey: raw.paneKey, keys: raw.keys as string, since: raw.since }
        : null
    default:
      return null
  }
}

export function parseRemotePhoneCall<T extends RemoteCommandName>(raw: unknown, name: T): RemotePhoneCall<T> | null {
  if (!isObj(raw) || !ident(raw.device, DEVICE_MAX)) return null
  const cmd = parseRemoteCommand(raw.cmd, name)
  if (!cmd) return null
  const confirmed = raw.confirmed === true
  return { device: raw.device, cmd: cmd as Extract<RemoteCommand, { t: T }>, ...(confirmed ? { confirmed: true as const } : {}) }
}

export type RemoteDeviceState = 'connected' | 'idle' | 'revoked'

export interface RemoteDevice {
  deviceId: string
  name: string
  platform: 'android' | 'ios'
  pairedAt: number
  lastSeen: number
  via?: 'lan' | 'relay'
  state: RemoteDeviceState
}

export interface RemoteStatus {
  enabled: boolean
  listening: boolean
  addresses: string[]
  port: number
  relay: 'off' | 'connecting' | 'online' | 'error'
  keepAwake: boolean
  devices: RemoteDevice[]
}
