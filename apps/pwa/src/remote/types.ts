export type JsonMap = Record<string, unknown>

export type LinkRoute = 'lan' | 'relay'
export type RoutePolicy = 'auto' | 'lan' | 'relay'

export type LinkStatus = 'idle' | 'discovering' | 'handshaking' | 'ready' | 'reconnecting' | 'offline' | 'protoTooOld' | 'revoked'

export type LinkReason =
  | 'none'
  | 'background'
  | 'networkLost'
  | 'desktopAbsent'
  | 'remoteOff'
  | 'restarting'
  | 'tokenExpired'
  | 'rateLimited'
  | 'handshakeFailed'
  | 'noRoute'

export interface LinkState {
  status: LinkStatus
  route?: LinkRoute
  reason: LinkReason
  attempt: number
  retryIn?: number
  minProto?: number
}

export const IDLE_LINK: LinkState = { status: 'idle', reason: 'none', attempt: 0 }

export const CloseCode = {
  handshake: 4400,
  revoked: 4401,
  desktopReplaced: 4409,
  protoTooOld: 4426,
  rate: 4429,
  gatewayDown: 4503,
} as const

export const FrameKind = { json: 0, ptyOut: 1, ptyIn: 2, ping: 3 } as const

export function isTerminal(state: LinkState): boolean {
  return state.status === 'protoTooOld' || state.status === 'revoked'
}

export class RemoteCommandError extends Error {
  readonly code: string
  readonly detail?: string
  constructor(code: string, detail?: string) {
    super(code)
    this.code = code
    this.detail = detail
  }
}

export interface Machine {
  id: string
  name: string
  alias?: string
  mkey: Uint8Array
  pairedAt: number
  lastHost?: string
  lastPort?: number
  relayUrl?: string
  relayToken?: string
  relayTokenExp?: number
  revoked?: boolean
  policy: RoutePolicy
}

export interface ConnectionConfig {
  lanBudgetMs: number
  relayConnectTimeoutMs: number
  handshakeTimeoutMs: number
  backgroundGraceMs: number
  keepaliveMs: number
  deadAfterMs: number
  requestTimeoutMs: number
  backoffMs: number[]
  lanPort: number
  proto: number
  appName: string
  platform: string
}

export const DEFAULT_CONFIG: ConnectionConfig = {
  lanBudgetMs: 3000,
  relayConnectTimeoutMs: 8000,
  handshakeTimeoutMs: 5000,
  backgroundGraceMs: 30_000,
  keepaliveMs: 30_000,
  deadAfterMs: 75_000,
  requestTimeoutMs: 10_000,
  backoffMs: [1000, 2000, 4000, 8000, 16_000, 30_000],
  lanPort: 4320,
  proto: 1,
  appName: 'orrery-web',
  platform: 'web',
}

export function backoffFor(config: ConnectionConfig, failures: number): number {
  const index = failures <= 0 ? 0 : failures - 1
  return config.backoffMs[Math.min(index, config.backoffMs.length - 1)] ?? 30_000
}
