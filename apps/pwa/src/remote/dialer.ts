import { lanUrl, relayPhoneUrl } from './relayToken'
import { linkState } from './linkRules'
import type { ConnectionConfig, LinkRoute, LinkState, Machine, RoutePolicy } from './types'
import type { WireConnector, WireSocket } from './wire'

export interface Opened {
  socket: WireSocket
  route: LinkRoute
  host?: string
  port?: number
}

export type DialResult = { opened: Opened } | { failure: LinkState }

export interface DialDeps {
  connector: WireConnector
  config: ConnectionConfig
  nowSeconds(): number
}

export function planRoutes(policy: RoutePolicy, machine: Pick<Machine, 'lastHost' | 'relayUrl'>): LinkRoute[] {
  const lan = machine.lastHost ? (['lan'] as const) : []
  const relay = machine.relayUrl ? (['relay'] as const) : []
  if (policy === 'lan') return [...lan]
  if (policy === 'relay') return [...relay]
  return [...lan, ...relay]
}

async function openLan(deps: DialDeps, machine: Machine): Promise<Opened | null> {
  if (!machine.lastHost) return null
  const port = machine.lastPort ?? deps.config.lanPort
  try {
    const socket = await deps.connector(lanUrl(machine.lastHost, port), deps.config.lanBudgetMs)
    return { socket, route: 'lan', host: machine.lastHost, port }
  } catch {
    return null
  }
}

async function openRelay(deps: DialDeps, machine: Machine): Promise<DialResult> {
  if (!machine.relayUrl || !machine.relayToken) return { failure: linkState('offline', { reason: 'noRoute' }) }
  if (machine.relayTokenExp !== undefined && deps.nowSeconds() >= machine.relayTokenExp) {
    return { failure: linkState('offline', { reason: 'tokenExpired' }) }
  }
  try {
    const socket = await deps.connector(relayPhoneUrl(machine.relayUrl, machine.id, machine.relayToken), deps.config.relayConnectTimeoutMs)
    return { opened: { socket, route: 'relay' } }
  } catch {
    return { failure: linkState('reconnecting', { route: 'relay', reason: 'networkLost' }) }
  }
}

export async function dial(deps: DialDeps, machine: Machine, policy: RoutePolicy): Promise<DialResult> {
  const routes = planRoutes(policy, machine)
  if (routes.length === 0) return { failure: linkState('offline', { reason: 'noRoute' }) }
  let last: DialResult = { failure: linkState('offline', { reason: 'noRoute' }) }
  for (const route of routes) {
    if (route === 'lan') {
      const opened = await openLan(deps, machine)
      if (opened) return { opened }
      last = { failure: linkState('reconnecting', { route: 'lan', reason: 'networkLost' }) }
    } else {
      const result = await openRelay(deps, machine)
      if ('opened' in result) return result
      last = result
    }
  }
  return last
}
