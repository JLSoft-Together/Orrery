import { isAgentSessionId } from './pty'
import type { LayoutNodeLike } from './remoteLayout'

export const REMOTE_PROTO = 1
export const REMOTE_MIN_PROTO = 1
export const REMOTE_PORT = 4320
export const REMOTE_TEXT_MAX = 2000
export const REMOTE_JSON_MAX = 1024 * 1024
export const REMOTE_ATTACH_MAX = 4
export const REMOTE_CONFIRM_MS = 60_000
export const REMOTE_RID_MAX = 128
export const REMOTE_RID_KEEP = 50
export const REMOTE_RID_TTL_MS = 60_000
export const REMOTE_SUMMARY_DEBOUNCE_MS = 200

export type RemoteErrorCode =
  | 'denied'
  | 'busy'
  | 'cancelled'
  | 'migration'
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
  | { t: 'push.register'; token: string; platform: RemotePushPlatform; locale: RemotePushLocale; mute?: RemotePushGroup[] }
  | { t: 'push.unregister' }

export type RemotePushPlatform = 'android' | 'ios'
export type RemotePushLocale = 'vi' | 'en'
export type RemotePushGroup = 'waiting' | 'done' | 'error'
export const REMOTE_PUSH_GROUPS: readonly RemotePushGroup[] = ['waiting', 'done', 'error']
export const REMOTE_PUSH_TOKEN_RE = /^[A-Za-z0-9:_.-]{16,512}$/
export const REMOTE_PUSH_KINDS = ['done', 'question', 'permission', 'attention'] as const
export type RemotePushKind = (typeof REMOTE_PUSH_KINDS)[number]

export interface RemotePushConfig {
  apiKey: string
  appId: string
  senderId: string
  projectId: string
}

export type RemotePushConfigEvent = { t: 'push.config' } | ({ t: 'push.config' } & RemotePushConfig)

export interface RemotePushData {
  m: string
  p: string
  k: RemotePushKind
}

export type RemoteCommandName = RemoteCommand['t']

export type RemoteKeyPress = { key: 'approve' | 'deny' | 'stop' | 'esc' } | { key: 'option'; option: number }

export interface RemotePhoneCall<T extends RemoteCommandName = RemoteCommandName> {
  device: string
  cmd: Extract<RemoteCommand, { t: T }>
  confirmed?: boolean
  rid?: string
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
  relay?: boolean
  lan?: number
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
    case 'push.register': {
      if (typeof raw.token !== 'string' || !REMOTE_PUSH_TOKEN_RE.test(raw.token)) return null
      if (raw.platform !== 'android' && raw.platform !== 'ios') return null
      if (raw.locale !== 'vi' && raw.locale !== 'en') return null
      const mute = Array.isArray(raw.mute) ? REMOTE_PUSH_GROUPS.filter((g) => (raw.mute as unknown[]).includes(g)) : []
      return { t: 'push.register', token: raw.token, platform: raw.platform, locale: raw.locale, ...(mute.length ? { mute } : {}) }
    }
    case 'push.unregister':
      return { t: 'push.unregister' }
    default:
      return null
  }
}

export function parseRemotePhoneCall<T extends RemoteCommandName>(raw: unknown, name: T): RemotePhoneCall<T> | null {
  if (!isObj(raw) || !ident(raw.device, DEVICE_MAX)) return null
  const cmd = parseRemoteCommand(raw.cmd, name)
  if (!cmd) return null
  const confirmed = raw.confirmed === true
  const rid = ident(raw.rid, REMOTE_RID_MAX) ? raw.rid : undefined
  return { device: raw.device, cmd: cmd as Extract<RemoteCommand, { t: T }>, ...(confirmed ? { confirmed: true as const } : {}), ...(rid ? { rid } : {}) }
}

export type RemoteDeviceState = 'connected' | 'idle' | 'revoked'

export const REMOTE_DEVICE_ALIAS_MAX = 40

export interface RemoteDevice {
  deviceId: string
  name: string
  originalName?: string
  alias?: string
  platform: 'android' | 'ios' | 'web' | 'unknown'
  pairedAt: number
  lastSeen: number
  via?: 'lan' | 'relay'
  state: RemoteDeviceState
  push?: 'web' | 'fcm' | 'none'
  pushOn?: boolean
  pushContent?: boolean
}

export interface RemoteStatus {
  enabled: boolean
  listening: boolean
  addresses: string[]
  port: number
  relay: 'off' | 'connecting' | 'online' | 'error'
  keepAwake: boolean
  devices: RemoteDevice[]
  pairing?: { exp: number; relay: boolean } | null
  pairEnded?: 'tries' | null
  guardRev?: number
}

export type RelaySetupState = 'none' | 'ready' | 'deploying' | 'error'

export type RelaySetupError =
  | 'token_invalid'
  | 'token_scope'
  | 'rate_limited'
  | 'no_account'
  | 'no_token'
  | 'keyring'
  | 'offline'
  | 'subdomain'
  | 'upload'
  | 'enable'
  | 'health'
  | 'bundle'
  | 'busy'
  | 'rejected'
  | 'cancelled'
  | 'migration'

export type RelayDeployStep = 'verify' | 'subdomain' | 'upload' | 'enable' | 'health' | 'done'

export interface RelaySetupStatus {
  state: RelaySetupState
  url?: string
  account?: string
  updateAvailable: boolean
  hasToken: boolean
  error?: RelaySetupError
  warning?: 'health'
}

export interface RelayAccount {
  id: string
  name: string
}

export type RelayTokenResult = { ok: true; accounts: RelayAccount[] } | { ok: false; error: RelaySetupError }
export type RelayDeployResult = { ok: true; url: string; warning?: 'health' } | { ok: false; error: RelaySetupError }
export type RelayRemoveResult = { ok: true; workerDeleted: boolean } | { ok: false; error: RelaySetupError; workerDeleted?: boolean }

export type PushSetupError = 'bad_json' | 'bad_service_account' | 'mismatch_project' | 'no_android_client' | 'keyring'
export type PushTestError = 'not_configured' | 'no_device' | 'unregistered' | 'auth' | 'denied' | 'rejected' | 'quota' | 'unavailable' | 'offline' | 'keyring'

export interface PushStatus {
  configured: boolean
  projectId?: string
  tokens: number
  error?: PushTestError
}

export type PushSetupResult = { ok: true; projectId: string } | { ok: false; error: PushSetupError }
export type PushTestResult = { ok: true; sent: number } | { ok: false; error: PushTestError }

export interface RelayDeployProgress {
  step: RelayDeployStep
}

export type RemoteGuardSource = 'lan' | 'relay' | 'manual'
export type RemoteGuardResult = 'ok' | 'fail' | 'blocked' | 'denied'

export interface RemoteGuardBlock {
  key: string
  source: RemoteGuardSource
  why: string
  count: number
  at: number
  until: number | null
  level: number
}

export interface RemoteGuardLogLine {
  at: number
  ip: string
  via: 'lan' | 'relay'
  country?: string
  device?: string
  result: RemoteGuardResult
  why: string
}

export interface RemoteGuardState {
  blocked: RemoteGuardBlock[]
  trusted: string[]
  log: RemoteGuardLogLine[]
  rev: number
}

export type RemoteGuardOp =
  | { op: 'get' }
  | { op: 'unblock'; key: string }
  | { op: 'unblock-all' }
  | { op: 'block'; cidr: string; ttl: number | null }
  | { op: 'trust'; list: string[] }

export type RemoteGuardError = 'invalid' | 'live' | 'full' | 'keyring'

export type RemoteGuardReply = { ok: true; state: RemoteGuardState } | { ok: false; error: RemoteGuardError; state?: RemoteGuardState }

export const REMOTE_GUARD_RULES_MAX = 32
export const REMOTE_GUARD_TEXT_MAX = 64
