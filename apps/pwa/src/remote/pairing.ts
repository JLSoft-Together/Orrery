import { fromHex } from './bytes'
import type { ConnectionDeps } from './connection'
import type { QrView } from './noise'
import { lanUrl, parseRelayToken, relayPairUrl, relayPhoneUrl } from './relayToken'
import type { Machine } from './types'
import { FrameKind } from './types'
import type { WireSocket } from './wire'

export interface PairInput {
  qr: string
  token?: string
  deviceName: string
  onRoute?: (route: 'lan' | 'relay') => void
}

export interface PairFlow {
  pair(input: PairInput): Promise<Machine>
}

export class PairError extends Error {
  readonly code: string
  constructor(code: string) {
    super(code)
    this.code = code
  }
}

interface Candidate {
  url: string
  route: 'lan' | 'relay'
  budgetMs: number
}

export function lanEndpoints(view: QrView, defaultPort: number): Array<{ host: string; port: number }> {
  if (view.lan?.length) return view.lan
  return view.host ? [{ host: view.host, port: view.port ?? defaultPort }] : []
}

export function pairCandidates(view: QrView, input: Pick<PairInput, 'token'>, lanBudgetMs: number, relayBudgetMs: number, defaultPort: number): Candidate[] {
  const out: Candidate[] = lanEndpoints(view, defaultPort).map((a) => ({ url: lanUrl(a.host, a.port), route: 'lan' as const, budgetMs: lanBudgetMs }))
  if (view.relay && view.pt) out.push({ url: relayPairUrl(view.relay, view.machine, view.pt), route: 'relay', budgetMs: relayBudgetMs })
  else if (view.relay && input.token) out.push({ url: relayPhoneUrl(view.relay, view.machine, input.token), route: 'relay', budgetMs: relayBudgetMs })
  return out
}

async function openFirst(deps: ConnectionDeps, candidates: Candidate[], onRoute?: (route: 'lan' | 'relay') => void): Promise<WireSocket> {
  for (const candidate of candidates) {
    onRoute?.(candidate.route)
    try {
      return await deps.connector(candidate.url, candidate.budgetMs)
    } catch {
      continue
    }
  }
  throw new PairError('noRoute')
}

export function createPairFlow(deps: ConnectionDeps): PairFlow {
  return {
    async pair(input) {
      const { noise, config } = deps
      const view = noise.decodeQr(input.qr)
      const candidates = pairCandidates(view, input, config.lanBudgetMs, config.relayConnectTimeoutMs, config.lanPort)
      if (candidates.length === 0) throw new PairError('noRoute')
      const identity = await deps.keys.load()
      const socket = await openFirst(deps, candidates, input.onRoute)
      const pairing = noise.pairFromQr(identity.privateKey, input.qr, identity.deviceId, input.deviceName, Math.floor(deps.now() / 1000), config.platform)
      try {
        socket.sendBinary(pairing.msg1)
        const msg2 = await socket.next(config.handshakeTimeoutMs)
        socket.sendBinary(pairing.finish(msg2))
        const link = pairing.intoLink()
        try {
          const deadline = deps.now() + config.handshakeTimeoutMs
          for (;;) {
            const message = link.open(await socket.next(Math.max(1, deadline - deps.now())))
            if (!message || message.kind !== FrameKind.json) continue
            const reply = JSON.parse(message.text()) as Record<string, unknown>
            if (reply.t === 'pair.denied') throw new PairError('denied')
            if (reply.t !== 'pair.ok') continue
            const relayUrl = typeof reply.relay === 'string' ? reply.relay : (view.relay ?? undefined)
            const parsed = typeof reply.relayToken === 'string' ? parseRelayToken(reply.relayToken) : null
            const lan = lanEndpoints(view, config.lanPort)[0]
            return {
              id: view.machine,
              name: typeof reply.name === 'string' && reply.name ? reply.name : view.name,
              mkey: fromHex(view.mkey),
              pairedAt: deps.now(),
              lastHost: lan?.host,
              lastPort: lan?.port,
              relayUrl,
              relayToken: parsed?.token,
              relayTokenExp: parsed?.exp,
              policy: 'auto',
            }
          }
        } finally {
          link.dispose()
        }
      } finally {
        pairing.dispose()
        socket.close()
      }
    },
  }
}
