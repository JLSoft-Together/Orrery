import type { Opened } from './dialer'
import { classifyClose, linkState } from './linkRules'
import { NoiseError, type NoiseLink, type NoiseSuite } from './noise'
import { SocketClosed, WireTimeout } from './wire'
import { CloseCode, type LinkReason, type LinkState, type Machine } from './types'

export type HandshakeOutcome = { link: NoiseLink } | { failure: LinkState; revoked?: boolean }

export interface HandshakeInput {
  noise: NoiseSuite
  machine: Machine
  privateKey: Uint8Array
  deviceId: string
  opened: Opened
  timeoutMs: number
  proto: number
  hint: LinkReason
}

export async function runSessionHandshake(input: HandshakeInput): Promise<HandshakeOutcome> {
  const { opened, machine } = input
  const socket = opened.socket
  const viaRelay = opened.route === 'relay'
  const reconnecting = () => linkState('reconnecting', { route: opened.route, reason: 'handshakeFailed' })
  let start: ReturnType<NoiseSuite['sessionStart']> | null = null
  try {
    start = input.noise.sessionStart(input.privateKey, machine.mkey, input.deviceId)
    socket.sendBinary(start.msg1)
    const msg2 = await socket.next(input.timeoutMs)
    return { link: start.finish(msg2) }
  } catch (error) {
    start?.dispose()
    if (error instanceof SocketClosed) {
      const failure = classifyClose(error.code, viaRelay, input.hint, input.proto)
      return { failure, revoked: error.code === CloseCode.revoked }
    }
    socket.close(CloseCode.handshake)
    if (error instanceof WireTimeout) return { failure: reconnecting() }
    if (error instanceof NoiseError && error.code === 'protoTooOld') {
      return { failure: linkState('protoTooOld', { route: opened.route, minProto: input.proto }) }
    }
    return { failure: reconnecting() }
  }
}
