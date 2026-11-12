import { FrameKind, RemoteCommandError, type JsonMap } from './types'
import type { NoiseLink } from './noise'
import { SocketClosed, type WireSocket } from './wire'

export interface PtyChunk {
  stream: number
  bytes: Uint8Array
}

interface Pending {
  resolve(value: JsonMap): void
  reject(error: Error): void
  timer: ReturnType<typeof setTimeout>
}

const MAX_JSON_BYTES = 1 << 20
const encoder = new TextEncoder()

export interface SessionOptions {
  socket: WireSocket
  link: NoiseLink
  viaRelay: boolean
  requestTimeoutMs: number
}

export class RemoteSession {
  readonly viaRelay: boolean
  readonly closed: Promise<number | null>
  onEvent: (event: JsonMap) => void = () => {}
  onPty: (chunk: PtyChunk) => void = () => {}
  onRtt: (ms: number) => void = () => {}
  private readonly socket: WireSocket
  private readonly link: NoiseLink
  private readonly requestTimeoutMs: number
  private readonly pending = new Map<string, Pending>()
  private settle!: (code: number | null) => void
  private done = false
  private counter = 0
  private pingAt: number | null = null

  constructor(options: SessionOptions) {
    this.socket = options.socket
    this.link = options.link
    this.viaRelay = options.viaRelay
    this.requestTimeoutMs = options.requestTimeoutMs
    this.closed = new Promise<number | null>((resolve) => {
      this.settle = resolve
    })
  }

  get isClosed(): boolean {
    return this.done
  }

  get lastActivity(): number {
    return this.socket.lastActivity
  }

  start(): void {
    void this.loop()
  }

  private async loop(): Promise<void> {
    let code: number | null = null
    try {
      for (;;) {
        const cipher = await this.socket.next()
        this.handleCipher(cipher)
      }
    } catch (error) {
      if (error instanceof SocketClosed) {
        code = error.code
      } else {
        code = 4400
        this.socket.close(4400)
      }
    }
    this.finish(code)
  }

  private finish(code: number | null): void {
    if (this.done) return
    this.done = true
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer)
      pending.reject(new RemoteCommandError('offline'))
    }
    this.pending.clear()
    this.link.dispose()
    this.settle(code)
  }

  private handleCipher(cipher: Uint8Array): void {
    const message = this.link.open(cipher)
    if (!message) return
    if (message.kind === FrameKind.json) this.handleJson(message.text())
    else if (message.kind === FrameKind.ptyOut) this.onPty({ stream: message.stream, bytes: message.payload })
    else if (message.kind === FrameKind.ping) this.handlePing()
  }

  private handlePing(): void {
    if (this.pingAt !== null) {
      const rtt = Math.max(0, Date.now() - this.pingAt)
      this.pingAt = null
      this.onRtt(rtt)
    } else {
      this.sendFrame(FrameKind.ping, 0, new Uint8Array(0))
    }
  }

  private handleJson(text: string): void {
    let decoded: unknown
    try {
      decoded = JSON.parse(text)
    } catch {
      return
    }
    if (!decoded || typeof decoded !== 'object' || Array.isArray(decoded)) return
    const map = decoded as JsonMap
    const re = map.re
    if (typeof re === 'string' && typeof map.t !== 'string') {
      const pending = this.pending.get(re)
      if (!pending) return
      this.pending.delete(re)
      clearTimeout(pending.timer)
      if (map.ok === true) pending.resolve(map)
      else pending.reject(new RemoteCommandError(typeof map.error === 'string' ? map.error : 'internal', typeof map.detail === 'string' ? map.detail : undefined))
      return
    }
    this.onEvent(map)
  }

  private sendFrame(kind: number, stream: number, payload: Uint8Array, claim = false): void {
    if (this.done) return
    for (const cipher of this.link.seal(kind, stream, payload, claim)) this.socket.sendBinary(cipher)
  }

  request(type: string, fields?: JsonMap, timeoutMs?: number): Promise<JsonMap> {
    if (this.done) return Promise.reject(new RemoteCommandError('offline'))
    const id = `c${(++this.counter).toString(36)}`
    const body = encoder.encode(JSON.stringify({ id, t: type, ...fields }))
    if (body.length > MAX_JSON_BYTES) return Promise.reject(new RemoteCommandError('invalid', 'size'))
    return new Promise<JsonMap>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new RemoteCommandError('timeout'))
      }, timeoutMs ?? this.requestTimeoutMs)
      this.pending.set(id, { resolve, reject, timer })
      try {
        this.sendFrame(FrameKind.json, 0, body)
      } catch {
        clearTimeout(timer)
        this.pending.delete(id)
        reject(new RemoteCommandError('internal'))
      }
    })
  }

  send(message: JsonMap): void {
    const body = encoder.encode(JSON.stringify(message))
    if (body.length <= MAX_JSON_BYTES) this.sendFrame(FrameKind.json, 0, body)
  }

  sendPtyInput(stream: number, bytes: Uint8Array, claim = false): void {
    if (bytes.length > 0) this.sendFrame(FrameKind.ptyIn, stream, bytes, claim)
  }

  sendKeepalive(): void {
    if (this.done) return
    if (this.viaRelay) {
      this.socket.sendText('ping')
    } else {
      this.pingAt = Date.now()
      this.sendFrame(FrameKind.ping, 0, new Uint8Array(0))
    }
  }

  close(code?: number): void {
    this.socket.close(code)
    this.finish(this.socket.closeCode ?? code ?? null)
  }
}
