import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MachineConnection } from './connection'
import { CloseCode, FrameKind, type LinkState } from './types'
import { attachDesktop, connectionDeps, createNetwork, MemoryMachines, sampleMachine, testConfig, type FakeNetwork, type FakeSocket } from '../test/fakes'

let net: FakeNetwork
let machines: MemoryMachines
let sockets: FakeSocket[]
let seen: LinkState[]

function relayRoute(options: Parameters<typeof attachDesktop>[1] = {}) {
  net.routes.set('wss://relay.example/p/m1', (socket) => {
    sockets.push(socket)
    attachDesktop(socket, options)
  })
}

function make(onRevoked = vi.fn(), config = testConfig()) {
  const conn = new MachineConnection('m1', connectionDeps(net, machines, { onRevoked, config }))
  conn.states.on((s) => seen.push(s))
  return conn
}

const tick = (ms: number) => vi.advanceTimersByTimeAsync(ms)

beforeEach(() => {
  vi.useFakeTimers()
  net = createNetwork()
  machines = new MemoryMachines()
  machines.byId.set('m1', sampleMachine())
  sockets = []
  seen = []
})

afterEach(() => {
  vi.useRealTimers()
})

describe('MachineConnection', () => {
  it('connects over the relay, says hello and goes ready', async () => {
    relayRoute()
    const conn = make()
    const events: string[] = []
    conn.events.on((e) => events.push(String(e.t)))
    conn.connect()
    await tick(10)
    expect(conn.state).toMatchObject({ status: 'ready', route: 'relay' })
    expect(events).toContain('hello')
    expect(seen.map((s) => s.status)).toEqual(['discovering', 'handshaking', 'ready'])
    conn.disconnect()
  })

  it('uses LAN first in auto and remembers the host', async () => {
    net.routes.set('wss://10.0.0.2:4320/remote', (socket) => {
      sockets.push(socket)
      attachDesktop(socket)
    })
    machines.byId.set('m1', sampleMachine({ lastHost: '10.0.0.2' }))
    const conn = make()
    conn.connect()
    await tick(10)
    expect(conn.state.route).toBe('lan')
    conn.disconnect()
  })

  it('falls back to the relay when LAN is unreachable', async () => {
    relayRoute()
    machines.byId.set('m1', sampleMachine({ lastHost: '10.0.0.2' }))
    const conn = make()
    conn.connect()
    await tick(10)
    expect(conn.state).toMatchObject({ status: 'ready', route: 'relay' })
    conn.disconnect()
  })

  it('goes back to the relay after the LAN session dies', async () => {
    relayRoute()
    net.routes.set('wss://10.0.0.2:4320/remote', (socket) => {
      sockets.push(socket)
      attachDesktop(socket)
    })
    machines.byId.set('m1', sampleMachine({ lastHost: '10.0.0.2' }))
    const conn = make()
    conn.connect()
    await tick(10)
    expect(conn.state.route).toBe('lan')
    net.routes.delete('wss://10.0.0.2:4320/remote')
    sockets[0]?.serverClose(1006)
    await tick(10)
    expect(conn.state.status).toBe('reconnecting')
    await tick(150)
    expect(conn.state).toMatchObject({ status: 'ready', route: 'relay' })
    conn.disconnect()
  })

  it('treats close 4401 as revoked, terminal, and reports it once', async () => {
    relayRoute()
    const revoked = vi.fn()
    const conn = make(revoked)
    conn.connect()
    await tick(10)
    sockets[0]?.serverClose(CloseCode.revoked)
    await tick(10)
    expect(conn.state.status).toBe('revoked')
    expect(revoked).toHaveBeenCalledTimes(1)
    expect(revoked).toHaveBeenCalledWith('m1')
    await tick(5000)
    expect(net.opened).toHaveLength(1)
  })

  it('treats a 4401 during the handshake as revoked', async () => {
    net.routes.set('wss://relay.example/p/m1', (socket) => {
      sockets.push(socket)
      socket.onClientSend = () => socket.serverClose(CloseCode.revoked)
    })
    const revoked = vi.fn()
    const conn = make(revoked)
    conn.connect()
    await tick(10)
    expect(conn.state.status).toBe('revoked')
    expect(revoked).toHaveBeenCalledWith('m1')
  })

  it('stops on 4426 as protoTooOld without retrying', async () => {
    net.routes.set('wss://relay.example/p/m1', (socket) => {
      sockets.push(socket)
      socket.onClientSend = () => socket.serverClose(CloseCode.protoTooOld)
    })
    const conn = make()
    conn.connect()
    await tick(5000)
    expect(conn.state.status).toBe('protoTooOld')
    expect(net.opened).toHaveLength(1)
  })

  it('retries with backoff after the desktop goes absent (4503)', async () => {
    relayRoute()
    const conn = make()
    conn.connect()
    await tick(10)
    sockets[0]?.serverClose(CloseCode.gatewayDown)
    await tick(10)
    expect(conn.state).toMatchObject({ status: 'reconnecting', reason: 'desktopAbsent', attempt: 1, retryIn: 100 })
    await tick(120)
    expect(conn.state.status).toBe('ready')
    conn.disconnect()
  })

  it('fails a handshake that never answers and retries', async () => {
    net.routes.set('wss://relay.example/p/m1', (socket) => {
      sockets.push(socket)
    })
    const conn = make()
    conn.connect()
    await tick(250)
    expect(conn.state).toMatchObject({ status: 'reconnecting', reason: 'handshakeFailed' })
    conn.disconnect()
  })

  it('suspends after the background grace and reconnects in the foreground', async () => {
    relayRoute()
    const conn = make(vi.fn(), testConfig({ backgroundGraceMs: 1000 }))
    conn.connect()
    await tick(10)
    conn.goBackground()
    await tick(1100)
    expect(conn.state).toMatchObject({ status: 'idle', reason: 'background' })
    expect(sockets[0]?.isClosed).toBe(true)
    void conn.goForeground()
    await tick(10)
    expect(conn.state.status).toBe('ready')
    expect(sockets).toHaveLength(2)
    conn.disconnect()
  })

  it('reconnects immediately when the foreground finds a dead socket', async () => {
    relayRoute()
    const conn = make(vi.fn(), testConfig({ backoffMs: [60_000] }))
    conn.connect()
    await tick(10)
    sockets[0]?.serverClose(1006)
    await tick(10)
    expect(conn.state.status).toBe('reconnecting')
    void conn.goForeground()
    await tick(10)
    expect(conn.state.status).toBe('ready')
    expect(sockets).toHaveLength(2)
    conn.disconnect()
  })

  it('drops a silent socket on foreground refresh timeout', async () => {
    let silent = false
    relayRoute({
      onJson: (message) => {
        if (silent && (message.t === 'resync' || message.t === 'ws.get')) return true
        return false
      },
    })
    const conn = make()
    conn.connect()
    await tick(10)
    silent = true
    void conn.goForeground()
    await tick(4100)
    await tick(50)
    expect(sockets).toHaveLength(2)
    expect(conn.state.status).toBe('ready')
    conn.disconnect()
  })

  it('kills the session when nothing arrives for deadAfter', async () => {
    relayRoute()
    const conn = make(vi.fn(), testConfig({ keepaliveMs: 1000, deadAfterMs: 2500, backoffMs: [100] }))
    conn.connect()
    await tick(10)
    const socket = sockets[0] as FakeSocket
    socket.lastActivity = Date.now() - 10_000
    await tick(1100)
    await tick(200)
    expect(sockets).toHaveLength(2)
    conn.disconnect()
  })

  it('stores a refreshed relay token event', async () => {
    relayRoute()
    const conn = make()
    conn.connect()
    await tick(10)
    const bytes = new Uint8Array(61)
    new DataView(bytes.buffer).setBigUint64(21, BigInt(2_000_000_000))
    const token = btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
    sockets[0]?.serverJson({ t: 'relay.token', token })
    await tick(10)
    expect(machines.byId.get('m1')).toMatchObject({ relayToken: token, relayTokenExp: 2_000_000_000 })
    conn.disconnect()
  })

  it('treats a device.revoked event like a revoke close', async () => {
    relayRoute()
    const revoked = vi.fn()
    const conn = make(revoked)
    conn.connect()
    await tick(10)
    sockets[0]?.serverJson({ t: 'device.revoked' })
    await tick(10)
    expect(conn.state.status).toBe('revoked')
    expect(revoked).toHaveBeenCalledWith('m1')
  })

  it('sends a text ping on relay keepalive and a noise ping on LAN', async () => {
    relayRoute()
    const conn = make(vi.fn(), testConfig({ keepaliveMs: 1000 }))
    conn.connect()
    await tick(1100)
    expect(sockets[0]?.texts).toContain('ping')
    expect(sockets[0]?.sent.some((s) => s[0] === FrameKind.ping)).toBe(false)
    conn.disconnect()
  })

  it('answers desktop pings and reports LAN rtt', async () => {
    net.routes.set('wss://10.0.0.2:4320/remote', (socket) => {
      sockets.push(socket)
      attachDesktop(socket)
    })
    machines.byId.set('m1', sampleMachine({ lastHost: '10.0.0.2' }))
    const conn = make()
    const rtts: number[] = []
    conn.rtt.on((ms) => rtts.push(ms))
    conn.connect()
    await tick(10)
    expect(rtts).toHaveLength(1)
    const before = sockets[0]?.sent.length ?? 0
    sockets[0]?.serverPing()
    await tick(10)
    expect(sockets[0]?.sent.length).toBe(before + 1)
    conn.disconnect()
  })

  it('reports offline when the machine has no route', async () => {
    machines.byId.set('m1', sampleMachine({ relayUrl: undefined }))
    const conn = make()
    conn.connect()
    await tick(10)
    expect(conn.state).toMatchObject({ status: 'offline', reason: 'noRoute' })
    conn.disconnect()
  })
})
