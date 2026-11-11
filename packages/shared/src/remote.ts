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
