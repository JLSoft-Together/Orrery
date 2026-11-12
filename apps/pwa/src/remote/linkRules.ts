import { CloseCode, type LinkReason, type LinkRoute, type LinkState } from './types'

export function linkState(status: LinkState['status'], extra: Partial<LinkState> = {}): LinkState {
  return { status, reason: 'none', attempt: 0, ...extra }
}

export function classifyClose(code: number | null, viaRelay: boolean, hint: LinkReason, proto: number): LinkState {
  const route: LinkRoute = viaRelay ? 'relay' : 'lan'
  switch (code) {
    case CloseCode.revoked:
      return linkState('revoked', { route })
    case CloseCode.protoTooOld:
      return linkState('protoTooOld', { route, minProto: proto })
    case CloseCode.gatewayDown:
      return linkState('reconnecting', { route, reason: hint === 'none' ? 'desktopAbsent' : hint })
    case CloseCode.rate:
      return linkState('reconnecting', { route, reason: 'rateLimited' })
    default:
      return linkState('reconnecting', { route, reason: hint === 'none' ? 'handshakeFailed' : hint })
  }
}

export function helloFailure(code: string, route: LinkRoute, proto: number): LinkState {
  if (code === 'proto') return linkState('protoTooOld', { route, minProto: proto })
  return linkState('reconnecting', { route, reason: 'handshakeFailed' })
}
