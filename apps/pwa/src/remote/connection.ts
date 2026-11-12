import { dial, type DialDeps, type Opened } from './dialer'
import { Emitter } from './emitter'
import { runSessionHandshake } from './handshake'
import { classifyClose, helloFailure, linkState } from './linkRules'
import type { NoiseLink, NoiseSuite } from './noise'
import { parseRelayToken } from './relayToken'
import { RemoteSession, type PtyChunk } from './session'
import type { WireConnector } from './wire'
import { backoffFor, isTerminal, RemoteCommandError, type ConnectionConfig, type JsonMap, type LinkReason, type LinkState, type Machine } from './types'

export interface DeviceIdentity {
  privateKey: Uint8Array
  deviceId: string
}

export interface KeyStore {
  load(): Promise<DeviceIdentity>
}

export interface MachineStore {
  get(id: string): Machine | undefined
  save(machine: Machine): Promise<void>
}

export interface ConnectionDeps {
  noise: NoiseSuite
  connector: WireConnector
  keys: KeyStore
  machines: MachineStore
  config: ConnectionConfig
  now(): number
  onRevoked(machineId: string): void | Promise<void>
}

type Attempt = { session: RemoteSession } | { session?: undefined; state: LinkState }

export class MachineConnection {
  readonly machineId: string
  readonly states = new Emitter<LinkState>()
  readonly events = new Emitter<JsonMap>()
  readonly pty = new Emitter<PtyChunk>()
  readonly rtt = new Emitter<number>()
  readonly sessions = new Emitter<RemoteSession>()
  private readonly deps: ConnectionDeps
  private current: LinkState = linkState('idle')
  private session: RemoteSession | null = null
  private generation = 0
  private failures = 0
  private skipDelay = false
  private suspended = false
  private hint: LinkReason = 'none'
  private keepalive: ReturnType<typeof setInterval> | null = null
  private backgroundTimer: ReturnType<typeof setTimeout> | null = null
  private backoffTimer: ReturnType<typeof setTimeout> | null = null
  private backoffWake: (() => void) | null = null

  constructor(machineId: string, deps: ConnectionDeps) {
    this.machineId = machineId
    this.deps = deps
  }

  get state(): LinkState {
    return this.current
  }

  get activeSession(): RemoteSession | null {
    return this.session
  }

  get isReady(): boolean {
    return this.current.status === 'ready' && this.session !== null && !this.session.isClosed
  }

  private emit(next: LinkState): void {
    const same = next.status === this.current.status && next.route === this.current.route && next.reason === this.current.reason && next.attempt === this.current.attempt && next.retryIn === this.current.retryIn && next.minProto === this.current.minProto
    if (same) return
    this.current = next
    this.states.emit(next)
  }

  connect(): void {
    if (this.backgroundTimer) clearTimeout(this.backgroundTimer)
    this.suspended = false
    const generation = ++this.generation
    this.wake()
    this.dropSession()
    this.failures = 0
    this.skipDelay = false
    this.hint = 'none'
    this.emit(linkState('discovering'))
    void this.run(generation)
  }

  disconnect(): void {
    if (this.backgroundTimer) clearTimeout(this.backgroundTimer)
    this.generation++
    this.wake()
    this.dropSession()
    this.emit(linkState('idle'))
  }

  goBackground(): void {
    if (this.suspended) return
    if (this.backgroundTimer) clearTimeout(this.backgroundTimer)
    this.backgroundTimer = setTimeout(() => {
      this.suspended = true
      this.generation++
      this.wake()
      this.dropSession()
      this.emit(linkState('idle', { reason: 'background' }))
    }, this.deps.config.backgroundGraceMs)
  }

  async goForeground(): Promise<void> {
    if (this.backgroundTimer) clearTimeout(this.backgroundTimer)
    if (this.suspended) {
      this.connect()
      return
    }
    if (isTerminal(this.current) || this.current.status === 'idle') return
    const session = this.session
    if (!session || session.isClosed || this.deps.now() - session.lastActivity > this.deps.config.deadAfterMs) {
      this.skipDelay = true
      this.failures = 0
      if (session && !session.isClosed) this.dropSession()
      this.wake()
      return
    }
    await this.refresh()
  }

  async refresh(): Promise<void> {
    const session = this.session
    if (!session) return
    try {
      await session.request('resync', undefined, 4000)
      await session.request('ws.get', undefined, 4000)
    } catch (error) {
      if (error instanceof RemoteCommandError && (error.code === 'timeout' || error.code === 'offline')) {
        this.skipDelay = true
        this.dropSession()
      }
    }
  }

  private wake(): void {
    if (this.backoffTimer) clearTimeout(this.backoffTimer)
    this.backoffTimer = null
    const wake = this.backoffWake
    this.backoffWake = null
    wake?.()
  }

  private delay(ms: number): Promise<void> {
    return new Promise<void>((resolve) => {
      this.backoffWake = resolve
      this.backoffTimer = setTimeout(() => {
        this.backoffWake = null
        resolve()
      }, ms)
    })
  }

  private dropSession(): void {
    if (this.keepalive) clearInterval(this.keepalive)
    this.keepalive = null
    const session = this.session
    this.session = null
    if (session && !session.isClosed) session.close()
  }

  private classify(code: number | null, viaRelay: boolean): LinkState {
    const hint = this.hint
    this.hint = 'none'
    return classifyClose(code, viaRelay, hint, this.deps.config.proto)
  }

  private async run(generation: number): Promise<void> {
    while (generation === this.generation) {
      const attempt = await this.attempt(generation)
      if (generation !== this.generation) {
        attempt.session?.close()
        return
      }
      let next: LinkState
      if (attempt.session) {
        this.failures = 0
        const code = await attempt.session.closed
        if (generation !== this.generation) return
        this.session = null
        if (this.keepalive) clearInterval(this.keepalive)
        this.keepalive = null
        next = this.classify(code, attempt.session.viaRelay)
        if (isTerminal(next)) {
          this.finishTerminal(next)
          return
        }
        this.failures = 1
      } else {
        next = attempt.state
        if (isTerminal(next)) {
          this.finishTerminal(next)
          return
        }
        this.failures += 1
      }
      this.emit({ ...next, attempt: this.failures })
      let wait = backoffFor(this.deps.config, this.failures)
      if (this.current.reason === 'rateLimited') wait = this.deps.config.backoffMs[this.deps.config.backoffMs.length - 1] ?? wait
      if (this.skipDelay) {
        this.skipDelay = false
        wait = 0
      }
      if (wait > 0) {
        this.emit({ ...this.current, retryIn: wait })
        await this.delay(wait)
        if (generation !== this.generation) return
      }
      this.emit({ ...this.current, status: 'discovering', retryIn: undefined })
    }
  }

  private finishTerminal(state: LinkState): void {
    if (state.status === 'revoked') void this.deps.onRevoked(this.machineId)
    this.emit(state)
  }

  private async attempt(generation: number): Promise<Attempt> {
    const machine = this.deps.machines.get(this.machineId)
    if (!machine) return { state: linkState('offline', { reason: 'noRoute' }) }
    if (machine.revoked) return { state: linkState('revoked') }
    const identity = await this.deps.keys.load()
    if (generation !== this.generation) return { state: linkState('idle') }
    this.emit({ ...this.current, status: 'discovering', route: undefined })
    const dialDeps: DialDeps = { connector: this.deps.connector, config: this.deps.config, nowSeconds: () => Math.floor(this.deps.now() / 1000) }
    const dialed = await dial(dialDeps, machine, machine.policy)
    if ('failure' in dialed) return { state: dialed.failure }
    if (generation !== this.generation) {
      dialed.opened.socket.close()
      return { state: linkState('idle') }
    }
    return this.handshake(generation, machine, identity, dialed.opened)
  }

  private async handshake(generation: number, machine: Machine, identity: DeviceIdentity, opened: Opened): Promise<Attempt> {
    this.emit({ ...this.current, status: 'handshaking', route: opened.route })
    const outcome = await runSessionHandshake({
      noise: this.deps.noise,
      machine,
      privateKey: identity.privateKey,
      deviceId: identity.deviceId,
      opened,
      timeoutMs: this.deps.config.handshakeTimeoutMs,
      proto: this.deps.config.proto,
      hint: this.hint,
    })
    if ('failure' in outcome) return { state: outcome.failure }
    if (generation !== this.generation) {
      outcome.link.dispose()
      opened.socket.close()
      return { state: linkState('idle') }
    }
    return this.bringUp(generation, machine, opened, outcome.link)
  }

  private async bringUp(generation: number, machine: Machine, opened: Opened, link: NoiseLink): Promise<Attempt> {
    const session = new RemoteSession({ socket: opened.socket, link, viaRelay: opened.route === 'relay', requestTimeoutMs: this.deps.config.requestTimeoutMs })
    this.session = session
    session.onEvent = (event) => this.onSessionEvent(event)
    session.onPty = (chunk) => this.pty.emit(chunk)
    session.start()
    let reply: JsonMap
    try {
      reply = await session.request('hello', { proto: this.deps.config.proto, app: this.deps.config.appName, platform: this.deps.config.platform, caps: [] })
    } catch (error) {
      const code = error instanceof RemoteCommandError ? error.code : 'internal'
      if (code === 'offline') {
        const closeCode = await Promise.race([session.closed, new Promise<null>((resolve) => setTimeout(() => resolve(null), 500))])
        this.dropSession()
        return { state: this.classify(closeCode, opened.route === 'relay') }
      }
      this.dropSession()
      return { state: helloFailure(code, opened.route, this.deps.config.proto) }
    }
    if (generation !== this.generation) {
      session.close()
      return { state: linkState('idle') }
    }
    await this.rememberRoute(machine, opened, reply)
    this.events.emit({ ...reply, t: 'hello' })
    this.emit(linkState('ready', { route: opened.route }))
    this.startKeepalive(session)
    this.sessions.emit(session)
    return { session }
  }

  private async rememberRoute(machine: Machine, opened: Opened, reply: JsonMap): Promise<void> {
    let updated = machine
    if (opened.host && (machine.lastHost !== opened.host || machine.lastPort !== opened.port)) {
      updated = { ...updated, lastHost: opened.host, lastPort: opened.port }
    }
    if (typeof reply.relayToken === 'string') {
      const parsed = parseRelayToken(reply.relayToken)
      if (parsed) updated = { ...updated, relayToken: parsed.token, relayTokenExp: parsed.exp }
    }
    if (typeof reply.name === 'string' && reply.name && reply.name !== machine.name) updated = { ...updated, name: reply.name }
    if (updated !== machine) await this.deps.machines.save(updated)
  }

  private startKeepalive(session: RemoteSession): void {
    if (this.keepalive) clearInterval(this.keepalive)
    if (!session.viaRelay) {
      session.onRtt = (ms) => {
        if (!session.isClosed) this.rtt.emit(ms)
      }
      session.sendKeepalive()
    }
    this.keepalive = setInterval(() => {
      if (session.isClosed) return
      if (this.deps.now() - session.lastActivity > this.deps.config.deadAfterMs) {
        this.hint = 'networkLost'
        session.close()
        return
      }
      session.sendKeepalive()
    }, this.deps.config.keepaliveMs)
  }

  private onSessionEvent(event: JsonMap): void {
    switch (event.t) {
      case 'device.revoked':
        this.generation++
        this.wake()
        this.dropSession()
        this.finishTerminal(linkState('revoked'))
        break
      case 'remote.off':
        this.hint = 'remoteOff'
        break
      case 'machine.restarting':
        this.hint = 'restarting'
        break
      case 'relay.token':
        if (typeof event.token === 'string') void this.storeRelayToken(event.token)
        break
    }
    this.events.emit(event)
  }

  private async storeRelayToken(token: string): Promise<void> {
    const parsed = parseRelayToken(token)
    const machine = this.deps.machines.get(this.machineId)
    if (!parsed || !machine) return
    await this.deps.machines.save({ ...machine, relayToken: parsed.token, relayTokenExp: parsed.exp })
  }
}
