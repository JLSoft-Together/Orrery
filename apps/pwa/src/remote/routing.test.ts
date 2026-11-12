import { describe, expect, it } from 'vitest'
import { dial, planRoutes } from './dialer'
import { classifyClose, helloFailure } from './linkRules'
import { lanUrl, parseRelayToken, relayPhoneUrl } from './relayToken'
import { backoffFor, CloseCode, DEFAULT_CONFIG } from './types'
import { createNetwork, sampleMachine, testConfig } from '../test/fakes'

function tokenWithExp(exp: number): string {
  const bytes = new Uint8Array(41 + 20)
  new DataView(bytes.buffer).setBigUint64(bytes.length - 40, BigInt(exp))
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

describe('relay token and urls', () => {
  it('reads exp from the token tail', () => {
    const token = tokenWithExp(1_900_000_000)
    expect(parseRelayToken(token)).toEqual({ token, exp: 1_900_000_000 })
    expect(parseRelayToken('short')).toBeNull()
    expect(parseRelayToken('!!!')).toBeNull()
  })

  it('builds phone and LAN urls', () => {
    expect(relayPhoneUrl('https://r.example/', 'm 1', 'a+b')).toBe('wss://r.example/p/m%201?t=a%2Bb')
    expect(relayPhoneUrl('http://127.0.0.1:8787', 'm', 't')).toBe('ws://127.0.0.1:8787/p/m?t=t')
    expect(lanUrl('192.168.1.5', 4320)).toBe('wss://192.168.1.5:4320/remote')
    expect(lanUrl('fe80::1', 4320)).toBe('wss://[fe80::1]:4320/remote')
  })
})

describe('planRoutes', () => {
  const both = { lastHost: '10.0.0.2', relayUrl: 'https://r' }
  it('orders by policy', () => {
    expect(planRoutes('auto', both)).toEqual(['lan', 'relay'])
    expect(planRoutes('lan', both)).toEqual(['lan'])
    expect(planRoutes('relay', both)).toEqual(['relay'])
  })
  it('skips routes the machine cannot use', () => {
    expect(planRoutes('auto', { relayUrl: 'https://r' })).toEqual(['relay'])
    expect(planRoutes('auto', { lastHost: '10.0.0.2' })).toEqual(['lan'])
    expect(planRoutes('lan', { relayUrl: 'https://r' })).toEqual([])
  })
})

describe('dial', () => {
  const machine = sampleMachine({ lastHost: '10.0.0.2', relayToken: 'tok' })

  function deps(net = createNetwork(), nowSeconds = 100) {
    return { net, deps: { connector: net.connector, config: testConfig(), nowSeconds: () => nowSeconds } }
  }

  it('prefers LAN in auto', async () => {
    const { net, deps: d } = deps()
    net.routes.set('wss://10.0.0.2:4320/remote', () => {})
    net.routes.set('wss://relay.example/p/m1', () => {})
    const result = await dial(d, machine, 'auto')
    expect('opened' in result && result.opened.route).toBe('lan')
    expect(net.opened).toHaveLength(1)
  })

  it('falls back to relay when LAN fails', async () => {
    const { net, deps: d } = deps()
    net.routes.set('wss://relay.example/p/m1', () => {})
    const result = await dial(d, machine, 'auto')
    expect('opened' in result && result.opened.route).toBe('relay')
    expect(net.opened[0]?.url).toBe('wss://relay.example/p/m1?t=tok')
  })

  it('relay policy never touches LAN', async () => {
    const { net, deps: d } = deps()
    net.routes.set('wss://10.0.0.2:4320/remote', () => {})
    net.routes.set('wss://relay.example/p/m1', () => {})
    await dial(d, machine, 'relay')
    expect(net.opened.map((o) => o.url.startsWith('wss://relay'))).toEqual([true])
  })

  it('lan policy fails as reconnecting without relay fallback', async () => {
    const { net, deps: d } = deps()
    net.routes.set('wss://relay.example/p/m1', () => {})
    const result = await dial(d, machine, 'lan')
    expect(result).toMatchObject({ failure: { status: 'reconnecting', route: 'lan' } })
    expect(net.opened).toHaveLength(0)
  })

  it('reports no route and expired tokens', async () => {
    const { deps: d } = deps()
    expect(await dial(d, sampleMachine({ relayUrl: undefined }), 'auto')).toMatchObject({ failure: { status: 'offline', reason: 'noRoute' } })
    const expired = sampleMachine({ relayTokenExp: 50 })
    expect(await dial(d, expired, 'relay')).toMatchObject({ failure: { status: 'offline', reason: 'tokenExpired' } })
  })

  it('reports a relay network failure', async () => {
    const { deps: d } = deps()
    expect(await dial(d, sampleMachine(), 'relay')).toMatchObject({ failure: { status: 'reconnecting', route: 'relay', reason: 'networkLost' } })
  })
})

describe('close codes and backoff', () => {
  it('classifies the documented codes', () => {
    expect(classifyClose(CloseCode.revoked, true, 'none', 1).status).toBe('revoked')
    expect(classifyClose(CloseCode.protoTooOld, false, 'none', 1)).toMatchObject({ status: 'protoTooOld', minProto: 1 })
    expect(classifyClose(CloseCode.gatewayDown, true, 'none', 1)).toMatchObject({ status: 'reconnecting', reason: 'desktopAbsent' })
    expect(classifyClose(CloseCode.gatewayDown, true, 'remoteOff', 1)).toMatchObject({ reason: 'remoteOff' })
    expect(classifyClose(CloseCode.rate, true, 'none', 1)).toMatchObject({ reason: 'rateLimited' })
    expect(classifyClose(1006, false, 'none', 1)).toMatchObject({ status: 'reconnecting', reason: 'handshakeFailed', route: 'lan' })
    expect(classifyClose(null, true, 'networkLost', 1)).toMatchObject({ reason: 'networkLost' })
  })

  it('maps hello failures', () => {
    expect(helloFailure('proto', 'lan', 1).status).toBe('protoTooOld')
    expect(helloFailure('internal', 'lan', 1).status).toBe('reconnecting')
  })

  it('walks the backoff ladder and saturates', () => {
    const steps = [1, 2, 3, 4, 5, 6, 7, 20].map((n) => backoffFor(DEFAULT_CONFIG, n))
    expect(steps).toEqual([1000, 2000, 4000, 8000, 16_000, 30_000, 30_000, 30_000])
    expect(backoffFor(DEFAULT_CONFIG, 0)).toBe(1000)
  })
})
