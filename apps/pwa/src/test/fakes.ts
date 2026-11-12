import type { ConnectionDeps, DeviceIdentity, MachineStore } from '../remote/connection'
import { DEFAULT_CONFIG, FrameKind, type ConnectionConfig, type JsonMap, type Machine } from '../remote/types'
import { NoiseError, type NoiseLink, type NoiseMessage, type NoiseSuite } from '../remote/noise'
import { SocketClosed, WireTimeout, type WireConnector, type WireSocket } from '../remote/wire'

const encoder = new TextEncoder()
const decoder = new TextDecoder()

export function frame(kind: number, stream: number, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(5 + payload.length)
  out[0] = kind
  new DataView(out.buffer).setUint32(1, stream)
  out.set(payload, 5)
  return out
}

export function jsonFrame(value: unknown): Uint8Array {
  return frame(FrameKind.json, 0, encoder.encode(JSON.stringify(value)))
}

export class FakeLink implements NoiseLink {
  closed = false
  disposed = false
  seal(kind: number, stream: number, payload: Uint8Array): Uint8Array[] {
    return [frame(kind, stream, payload)]
  }
  open(cipher: Uint8Array): NoiseMessage | null {
    const payload = cipher.slice(5)
    return { kind: cipher[0] ?? 0, stream: new DataView(cipher.buffer, cipher.byteOffset).getUint32(1), claim: false, payload, text: () => decoder.decode(payload) }
  }
  dispose(): void {
    this.disposed = true
  }
}

export const fakeNoise: NoiseSuite = {
  protoVersion: () => 1,
  generatePrivateKey: () => new Uint8Array(32).fill(7),
  decodeQr: (text) => JSON.parse(text),
  sessionStart() {
    return {
      msg1: new Uint8Array([1]),
      finish(msg2) {
        if (msg2[0] !== 2) throw new NoiseError(new Error('noise'))
        return new FakeLink()
      },
      dispose() {},
    }
  },
  pairFromQr() {
    throw new Error('not used')
  },
}

export class FakeSocket implements WireSocket {
  closeCode: number | null = null
  lastActivity = Date.now()
  readonly sent: Uint8Array[] = []
  readonly texts: string[] = []
  private readonly queue: Uint8Array[] = []
  private waiter: { resolve(b: Uint8Array): void; reject(e: Error): void; timer: ReturnType<typeof setTimeout> | null } | null = null
  private closed = false
  onClientSend: (data: Uint8Array) => void = () => {}
  awaitingPong = false

  get isClosed(): boolean {
    return this.closed
  }

  serverPing(): void {
    this.awaitingPong = true
    this.serverPush(frame(FrameKind.ping, 0, new Uint8Array(0)))
  }

  next(timeoutMs?: number): Promise<Uint8Array> {
    const ready = this.queue.shift()
    if (ready) return Promise.resolve(ready)
    if (this.closed) return Promise.reject(new SocketClosed(this.closeCode))
    return new Promise((resolve, reject) => {
      const waiter = { resolve, reject, timer: null as ReturnType<typeof setTimeout> | null }
      if (timeoutMs !== undefined) waiter.timer = setTimeout(() => reject(new WireTimeout('read')), timeoutMs)
      this.waiter = waiter
    })
  }

  sendBinary(data: Uint8Array): void {
    this.sent.push(data)
    this.onClientSend(data)
  }

  sendText(text: string): void {
    this.texts.push(text)
  }

  close(code?: number): void {
    this.serverClose(this.closeCode ?? code ?? null)
  }

  serverPush(data: Uint8Array): void {
    this.lastActivity = Date.now()
    const waiter = this.waiter
    this.waiter = null
    if (waiter) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.resolve(data)
    } else {
      this.queue.push(data)
    }
  }

  serverJson(value: unknown): void {
    this.serverPush(jsonFrame(value))
  }

  serverClose(code: number | null): void {
    if (this.closed) return
    this.closed = true
    this.closeCode = code
    const waiter = this.waiter
    this.waiter = null
    if (waiter) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.reject(new SocketClosed(code))
    }
  }
}

export interface DesktopOptions {
  hello?: JsonMap
  onJson?: (message: JsonMap, socket: FakeSocket) => boolean
  silentHello?: boolean
}

export function attachDesktop(socket: FakeSocket, options: DesktopOptions = {}): void {
  socket.onClientSend = (data) => {
    if (data.length === 1 && data[0] === 1) {
      socket.serverPush(new Uint8Array([2]))
      return
    }
    if (data[0] !== FrameKind.json) {
      if (data[0] === FrameKind.ping) {
        if (socket.awaitingPong) socket.awaitingPong = false
        else socket.serverPush(frame(FrameKind.ping, 0, new Uint8Array(0)))
      }
      return
    }
    const message = JSON.parse(decoder.decode(data.slice(5))) as JsonMap
    if (options.onJson?.(message, socket)) return
    if (message.t === 'hello') {
      if (options.silentHello) return
      socket.serverJson({ re: message.id, ok: true, machine: 'm1', name: 'Desk', version: '9.9.9', minProto: 1, ...options.hello })
      return
    }
    if (message.t === 'resync' || message.t === 'ws.get') socket.serverJson({ re: message.id, ok: true })
  }
}

export interface FakeNetwork {
  connector: WireConnector
  opened: Array<{ url: string; socket: FakeSocket }>
  routes: Map<string, (socket: FakeSocket) => void | Error>
  failUrls: Set<string>
}

export function createNetwork(): FakeNetwork {
  const net: FakeNetwork = {
    opened: [],
    routes: new Map(),
    failUrls: new Set(),
    connector: async (url) => {
      const key = [...net.routes.keys(), ...net.failUrls].find((k) => url.startsWith(k))
      if (!key || net.failUrls.has(key)) throw new Error('unreachable')
      const socket = new FakeSocket()
      const result = net.routes.get(key)?.(socket)
      if (result instanceof Error) throw result
      net.opened.push({ url, socket })
      return socket
    },
  }
  return net
}

export function sampleMachine(overrides: Partial<Machine> = {}): Machine {
  return {
    id: 'm1',
    name: 'Desk',
    mkey: new Uint8Array(32).fill(1),
    pairedAt: 1000,
    relayUrl: 'https://relay.example',
    relayToken: 'tok',
    policy: 'auto',
    ...overrides,
  }
}

export class MemoryMachines implements MachineStore {
  readonly byId = new Map<string, Machine>()
  async init(): Promise<Machine[]> {
    return this.list()
  }
  list(): Machine[] {
    return [...this.byId.values()]
  }
  get(id: string): Machine | undefined {
    return this.byId.get(id)
  }
  async save(machine: Machine): Promise<void> {
    this.byId.set(machine.id, machine)
  }
  async add(machine: Machine): Promise<void> {
    this.byId.set(machine.id, machine)
  }
  async forget(id: string): Promise<void> {
    this.byId.delete(id)
  }
}

export const IDENTITY: DeviceIdentity = { privateKey: new Uint8Array(32).fill(7), deviceId: 'pwa-test' }

export function testConfig(overrides: Partial<ConnectionConfig> = {}): ConnectionConfig {
  return { ...DEFAULT_CONFIG, lanBudgetMs: 50, handshakeTimeoutMs: 200, backoffMs: [100, 200, 400], ...overrides }
}

export function connectionDeps(net: FakeNetwork, machines: MachineStore, overrides: Partial<ConnectionDeps> = {}): ConnectionDeps {
  return {
    noise: fakeNoise,
    connector: net.connector,
    keys: { load: async () => IDENTITY },
    machines,
    config: testConfig(),
    now: () => Date.now(),
    onRevoked: () => {},
    ...overrides,
  }
}
