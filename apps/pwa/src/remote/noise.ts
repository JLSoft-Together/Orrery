import init, * as wasm from '../gen/core_remote_wasm.js'
import wasmUrl from '../gen/core_remote_wasm_bg.wasm?url'

export type WasmModule = typeof wasm

export interface NoiseMessage {
  kind: number
  stream: number
  claim: boolean
  payload: Uint8Array
  text(): string
}

export interface NoiseLink {
  seal(kind: number, stream: number, payload: Uint8Array, claim?: boolean): Uint8Array[]
  open(cipher: Uint8Array): NoiseMessage | null
  readonly closed: boolean
  dispose(): void
}

export interface SessionStart {
  msg1: Uint8Array
  finish(msg2: Uint8Array): NoiseLink
  dispose(): void
}

export interface PairStart {
  msg1: Uint8Array
  finish(msg2: Uint8Array): Uint8Array
  desktopStatic(): Uint8Array
  intoLink(): NoiseLink
  dispose(): void
}

export interface NoiseSuite {
  protoVersion(): number
  generatePrivateKey(): Uint8Array
  decodeQr(text: string): QrView
  sessionStart(privateKey: Uint8Array, desktopStatic: Uint8Array, deviceId: string): SessionStart
  pairFromQr(privateKey: Uint8Array, qr: string, deviceId: string, name: string, nowSeconds: number): PairStart
}

export interface QrView {
  v: number
  machine: string
  name: string
  host: string | null
  port: number | null
  lan?: Array<{ host: string; port: number }>
  relay: string | null
  pt?: string | null
  mkey: string
  exp: number
}

export class NoiseError extends Error {
  readonly code: string
  constructor(error: unknown) {
    const code = error instanceof Error ? error.message : String(error)
    super(code)
    this.code = code
  }
}

function guard<T>(run: () => T): T {
  try {
    return run()
  } catch (error) {
    throw new NoiseError(error)
  }
}

const strictDecoder = new TextDecoder('utf-8', { fatal: true })

function decodeText(bytes: Uint8Array): string {
  try {
    return strictDecoder.decode(bytes)
  } catch {
    throw new NoiseError(new Error('badFrame'))
  }
}

class WasmLink implements NoiseLink {
  private transport: wasm.Transport
  private readonly reassembler = new wasm.Reassembler()
  private disposed = false

  constructor(transport: wasm.Transport) {
    this.transport = transport
  }

  seal(kind: number, stream: number, payload: Uint8Array, claim = false): Uint8Array[] {
    return guard(() => this.transport.sealMessage(kind, stream, payload, claim) as Uint8Array[])
  }

  open(cipher: Uint8Array): NoiseMessage | null {
    const message = guard(() => this.transport.openMessage(cipher, this.reassembler))
    if (!message) return null
    const payload = message.payload
    const out: NoiseMessage = { kind: message.kind, stream: message.stream, claim: message.claim, payload, text: () => decodeText(payload) }
    message.free()
    return out
  }

  get closed(): boolean {
    return this.disposed || this.transport.isClosed
  }

  dispose(): void {
    if (this.disposed) return
    this.disposed = true
    this.reassembler.free()
    this.transport.free()
  }
}

function withIdentity<T>(privateKey: Uint8Array, use: (identity: wasm.Identity) => T): T {
  const identity = guard(() => wasm.Identity.fromPrivate(privateKey))
  try {
    return use(identity)
  } finally {
    identity.free()
  }
}

export function createWasmSuite(): NoiseSuite {
  return {
    protoVersion: () => wasm.protoVersion(),
    generatePrivateKey() {
      const identity = guard(() => wasm.Identity.generate())
      try {
        return identity.exportPrivate()
      } finally {
        identity.free()
      }
    },
    decodeQr: (text) => guard(() => JSON.parse(wasm.decodeQr(text)) as QrView),
    sessionStart(privateKey, desktopStatic, deviceId) {
      const session = withIdentity(privateKey, (identity) => guard(() => wasm.Session.start(identity, desktopStatic, deviceId)))
      const msg1 = session.msg1
      let used = false
      return {
        msg1,
        finish(msg2) {
          const transport = guard(() => session.finish(msg2))
          used = true
          session.free()
          return new WasmLink(transport)
        },
        dispose() {
          if (!used) session.free()
        },
      }
    },
    pairFromQr(privateKey, qr, deviceId, name, nowSeconds) {
      const pairing = withIdentity(privateKey, (identity) => guard(() => wasm.Pairing.fromQr(identity, qr, deviceId, name, nowSeconds)))
      return {
        msg1: pairing.msg1,
        finish: (msg2) => guard(() => pairing.finish(msg2)),
        desktopStatic: () => guard(() => pairing.desktopStatic),
        intoLink: () => new WasmLink(guard(() => pairing.intoTransport())),
        dispose: () => pairing.free(),
      }
    },
  }
}

let loaded: Promise<NoiseSuite> | null = null

export function loadNoise(): Promise<NoiseSuite> {
  loaded ??= init({ module_or_path: wasmUrl }).then(() => createWasmSuite())
  return loaded
}

export function useWasmBytes(bytes: BufferSource): NoiseSuite {
  wasm.initSync({ module: bytes })
  const suite = createWasmSuite()
  loaded = Promise.resolve(suite)
  return suite
}
