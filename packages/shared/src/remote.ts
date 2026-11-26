import { isAgentSessionId } from './pty'
import type { RemoteInbox } from './inbox'
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
  | 'stale'
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
  | 'notWaiting'
  | 'useMenu'

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
  inbox?: RemoteInbox
  menu?: RemoteMenu
  question?: string
  snoozed?: boolean
}

export const REMOTE_QUESTION_MAX = 300
export const REMOTE_SNOOZE_MINUTES = [15, 60] as const

export interface RemotePaneGroup {
  project: string
  name?: string
  tone?: number
  folded?: boolean
  panes: string[]
}

export interface RemoteTab {
  key: string
  name?: string
  project?: string
  tree: LayoutNodeLike
  zoom?: string
  folded?: boolean
  groups?: RemotePaneGroup[]
  panes: RemotePane[]
}

export type RemoteQuotaAgent = 'claude' | 'codex'
export type RemoteQuotaWindow = '5h' | 'week'

export interface RemoteQuota {
  agent: RemoteQuotaAgent
  window: RemoteQuotaWindow
  pct: number
  resetAt: number | null
  stale: boolean
}

export interface RemoteWorkspaceSnapshot {
  rev: number
  tabs: RemoteTab[]
  closedCount: number
  inbox?: true
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
  | { t: 'ws.menu'; paneKey: string; sig: string; index: number }
  | { t: 'ws.snooze'; paneKey: string; minutes: 15 | 60 | null }
  | { t: 'ws.reply'; paneKey: string; text: string }
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
  via?: 'usb' | 'qr' | 'lan'
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
    case 'ws.menu':
      return ident(raw.paneKey, KEY_ID_MAX) && ident(raw.sig, REMOTE_MENU_SIG_MAX) && Number.isInteger(raw.index) && (raw.index as number) >= 1 && (raw.index as number) <= REMOTE_MENU_ITEMS_MAX
        ? { t: 'ws.menu', paneKey: raw.paneKey, sig: raw.sig, index: raw.index as number }
        : null
    case 'ws.snooze':
      if (!ident(raw.paneKey, KEY_ID_MAX)) return null
      return raw.minutes === null || raw.minutes === 15 || raw.minutes === 60 ? { t: 'ws.snooze', paneKey: raw.paneKey, minutes: raw.minutes } : null
    case 'ws.reply':
      return ident(raw.paneKey, KEY_ID_MAX) && filled(raw.text, REMOTE_TEXT_MAX) ? { t: 'ws.reply', paneKey: raw.paneKey, text: raw.text } : null
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
  platform: 'android' | 'web' | 'unknown'
  pairedAt: number
  lastSeen: number
  via?: 'lan' | 'relay'
  state: RemoteDeviceState
  push?: 'web' | 'fcm' | 'none'
  pushOn?: boolean
  pushContent?: boolean
  appVersion?: string
  appBuild?: string
  model?: string
  os?: string
  pairedVia?: 'usb' | 'qr' | 'lan'
  adbSerialHash?: string
  revoking?: boolean
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
  pairEnded?: 'tries' | 'denied' | null
  guardRev?: number
  relayUrl?: string
  installFetched?: { ip: string; at: number } | null
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
  | 'no_node'
  | 'login_cancelled'
  | 'login_timeout'
  | 'deploy_failed'
  | 'subdomain_missing'
  | 'wrangler_failed'
  | 'timeout'

export type RelayDeployStep = 'verify' | 'subdomain' | 'upload' | 'enable' | 'login' | 'deploy' | 'health' | 'done'

export interface RelaySetupStatus {
  state: RelaySetupState
  url?: string
  account?: string
  updateAvailable: boolean
  hasToken: boolean
  via?: 'token' | 'wrangler'
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

export const REMOTE_CHAT_PROTO = 1
export const REMOTE_CHAT_PAGE = 30
export const REMOTE_CHAT_PAGE_MAX = 50
export const REMOTE_CHAT_REPLY_BYTES = 256 * 1024
export const REMOTE_CHAT_EVENT_BYTES = 64 * 1024
export const REMOTE_CHAT_SCAN_BYTES = 4 * 1024 * 1024
export const REMOTE_CHAT_BATCH_MS = 300
export const REMOTE_CHAT_WATCH_MAX = 2
export const REMOTE_CHAT_ME_MAX = 4000
export const REMOTE_CHAT_AI_MAX = 8000
export const REMOTE_CHAT_SYS_MAX = 300
export const REMOTE_CHAT_TOOLS_MAX = 30
export const REMOTE_CHAT_FILES_MAX = 20
export const REMOTE_CHAT_TARGET_MAX = 120
export const REMOTE_CHAT_TOOL_NAME_MAX = 40
export const REMOTE_CHAT_PATH_MAX = 300
export const REMOTE_CHAT_LAST_MAX = 160
export const REMOTE_CHAT_ID_MAX = 64
export const REMOTE_MENU_ITEMS_MAX = 12
export const REMOTE_MENU_LABEL_MAX = 120
export const REMOTE_MENU_QUESTION_MAX = 300
export const REMOTE_MENU_SIG_MAX = 16

export const REMOTE_CHAT_TOOL_CATS = ['edit', 'run', 'read', 'search', 'web', 'agent', 'other'] as const
export type RemoteChatToolCat = (typeof REMOTE_CHAT_TOOL_CATS)[number]

export const REMOTE_CHAT_SYS_KINDS = ['stop', 'error', 'limit', 'compact'] as const
export type RemoteChatSysKind = (typeof REMOTE_CHAT_SYS_KINDS)[number]

export type RemoteChatAgent = 'claude' | 'codex'

export interface RemoteChatTool {
  tool: string
  cat: RemoteChatToolCat
  target?: string
  err?: true
}

export interface RemoteChatFile {
  path: string
  add?: number
  del?: number
  out?: true
}

interface RemoteChatBase {
  id: string
  seq: number
  rev: number
  at?: number
}

export interface RemoteChatMe extends RemoteChatBase {
  k: 'me'
  text: string
  cut?: true
}

export interface RemoteChatAi extends RemoteChatBase {
  k: 'ai'
  text: string
  cut?: true
}

export interface RemoteChatTools extends RemoteChatBase {
  k: 'tools'
  sum: Partial<Record<RemoteChatToolCat, number>>
  items: RemoteChatTool[]
  more?: number
}

export interface RemoteChatFiles extends RemoteChatBase {
  k: 'files'
  root: string
  files: RemoteChatFile[]
  more?: number
}

export interface RemoteChatSys extends RemoteChatBase {
  k: 'sys'
  kind: RemoteChatSysKind
  text?: string
}

export type RemoteChatItem = RemoteChatMe | RemoteChatAi | RemoteChatTools | RemoteChatFiles | RemoteChatSys
export type RemoteChatItemKind = RemoteChatItem['k']

export type RemoteChatCommand =
  | { t: 'chat.page'; paneKey: string; gen?: string; before?: number; limit?: number }
  | { t: 'chat.watch'; paneKey: string; limit?: number }
  | { t: 'chat.unwatch'; paneKey: string }

export type RemoteChatCommandName = RemoteChatCommand['t']

export interface RemoteChatPage {
  paneKey: string
  agent: RemoteChatAgent
  gen: string
  items: RemoteChatItem[]
  older: number | null
}

export interface RemoteChatLast {
  paneKey: string
  text: string | null
  at?: number
}

export type RemoteChatEvent =
  | { t: 'chat.items'; paneKey: string; gen: string; items: RemoteChatItem[] }
  | { t: 'chat.reset'; paneKey: string; gen: string }
  | { t: 'chat.last'; full?: true; items: RemoteChatLast[] }

export type RemoteMenuKind = 'ask' | 'ask-multi' | 'permission' | 'plan' | 'picker' | 'codex-approval'

export interface RemoteMenuItem {
  index: number
  label: string
  selected?: true
  checked?: boolean
  other?: true
  submit?: true
}

export interface RemoteMenu {
  sig: string
  kind: RemoteMenuKind
  agent: 'claude' | 'codex' | 'unknown'
  question: string
  items: RemoteMenuItem[]
  numbered: boolean
  confident: boolean
  since: number
}
