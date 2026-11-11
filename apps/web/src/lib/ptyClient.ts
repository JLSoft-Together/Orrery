import type { PtyClientMessage, PtyEndpoint, PtyPreset, PtyServerMessage } from '@ccm/shared'
import { desktop } from './desktop'

export interface PtyHandlers {
  onOutput: (bytes: Uint8Array) => void
  onMessage: (msg: PtyServerMessage) => void
  onClose: (code: number) => void
}

export type PtyStart = { spawn: { preset: PtyPreset; cwd?: string; resume?: string; open?: string; session?: string; integrate?: boolean; shim?: boolean } } | { attach: string }

const encoder = new TextEncoder()
// Host caps frames at 1 MB; a big paste goes out in slices instead of killing the socket.
const CHUNK = 64 * 1024
const OVERFLOW_CLOSE = 4001
const MAX_REATTACH = 5
const REATTACH_WINDOW_MS = 30_000
const RESET_TERMINAL = new Uint8Array([0x1b, 0x63])

async function endpoint(): Promise<PtyEndpoint> {
  const ep = await desktop?.ptyEndpoint()
  if (!ep) throw new Error('terminals unavailable')
  return ep
}

/** One WebSocket per pane; the session itself lives in the desktop PTY host and survives reconnects. */
export class PtyConnection {
  private ws: WebSocket | null = null
  private closedByUs = false
  private sessionId: string | null = null
  private cols = 80
  private rows = 24
  private reattaches = 0
  private reattachedAt = 0
  private resizedByOther = false

  constructor(private readonly handlers: PtyHandlers) {}

  /** Kill a session by id over a throwaway socket, for panes whose own socket is gone. */
  static async killById(id: string): Promise<void> {
    const ep = await endpoint().catch(() => null)
    if (!ep) return
    await new Promise<void>((resolve) => {
      const ws = new WebSocket(ep.url, [ep.protocol])
      const done = (): void => {
        clearTimeout(timer)
        ws.onopen = ws.onerror = ws.onclose = null
        ws.close()
        resolve()
      }
      const timer = setTimeout(done, 5_000)
      ws.onopen = () => {
        ws.send(JSON.stringify({ t: 'kill', id } satisfies PtyClientMessage))
        done()
      }
      ws.onerror = done
      ws.onclose = done
    })
  }

  /** One question to the host over a throwaway socket; null on failure or timeout. */
  static async request(msg: PtyClientMessage, timeoutMs = 20_000): Promise<PtyServerMessage | null> {
    const ep = await endpoint().catch(() => null)
    if (!ep) return null
    return new Promise((resolve) => {
      const ws = new WebSocket(ep.url, [ep.protocol])
      const done = (reply: PtyServerMessage | null): void => {
        clearTimeout(timer)
        ws.onclose = null
        ws.close()
        resolve(reply)
      }
      const timer = setTimeout(() => done(null), timeoutMs)
      ws.onopen = () => ws.send(JSON.stringify(msg))
      ws.onmessage = (ev: MessageEvent<unknown>) => {
        if (typeof ev.data !== 'string') return
        try {
          done(JSON.parse(ev.data) as PtyServerMessage)
        } catch {
          done(null)
        }
      }
      ws.onclose = () => done(null)
    })
  }

  get isOpen(): boolean {
    return this.ws?.readyState === WebSocket.OPEN
  }

  async open(start: PtyStart, cols: number, rows: number): Promise<void> {
    const ep = await endpoint()
    if (this.closedByUs) return
    this.cols = cols
    this.rows = rows
    const ws = new WebSocket(ep.url, [ep.protocol])
    ws.binaryType = 'arraybuffer'
    this.ws = ws
    ws.onopen = () => {
      if ('spawn' in start) this.control({ t: 'spawn', preset: start.spawn.preset, cwd: start.spawn.cwd, resume: start.spawn.resume, open: start.spawn.open, session: start.spawn.session, integrate: start.spawn.integrate, shim: start.spawn.shim, cols, rows })
      else this.control({ t: 'attach', id: start.attach })
    }
    ws.onmessage = (ev: MessageEvent<string | ArrayBuffer>) => {
      // A replaced connection may still deliver frames; they belong to the old session.
      if (this.closedByUs || this.ws !== ws) return
      if (typeof ev.data !== 'string') return this.handlers.onOutput(new Uint8Array(ev.data))
      try {
        const msg = JSON.parse(ev.data) as PtyServerMessage
        if (msg.t === 'ready') this.sessionId = msg.id
        if (msg.t === 'resized') this.resizedByOther = true
        this.handlers.onMessage(msg)
      } catch {
        // malformed control frame: ignore, like unknown protocol events elsewhere
      }
    }
    ws.onclose = (ev) => {
      if (this.ws === ws) this.ws = null
      if (this.closedByUs) return
      if (ev.code === OVERFLOW_CLOSE && this.reattach()) return
      this.handlers.onClose(ev.code)
    }
  }

  private reattach(): boolean {
    const id = this.sessionId
    if (!id) return false
    const now = Date.now()
    if (now - this.reattachedAt > REATTACH_WINDOW_MS) this.reattaches = 0
    if (this.reattaches >= MAX_REATTACH) return false
    this.reattaches++
    this.reattachedAt = now
    this.handlers.onOutput(RESET_TERMINAL)
    this.open({ attach: id }, this.cols, this.rows).catch(() => this.handlers.onClose(OVERFLOW_CLOSE))
    return true
  }

  write(text: string): void {
    this.sendBytes(encoder.encode(text))
  }

  /** xterm's binary channel (mouse reports): one byte per char, not UTF-8. */
  writeBinary(data: string): void {
    this.sendBytes(Uint8Array.from(data, (c) => c.charCodeAt(0) & 0xff))
  }

  resize(cols: number, rows: number): void {
    this.cols = cols
    this.rows = rows
    this.control({ t: 'resize', cols, rows })
  }

  claim(): void {
    this.resizedByOther = false
    this.control({ t: 'claim' })
  }

  claimIfNeeded(): boolean {
    if (!this.resizedByOther) return false
    this.claim()
    return true
  }

  reportCwd(cwd: string): void {
    this.control({ t: 'cwd', cwd })
  }

  kill(): void {
    this.control({ t: 'kill' })
  }

  /** Detach only: the PTY keeps running and can be re-attached by id. */
  close(): void {
    this.closedByUs = true
    this.ws?.close()
    this.ws = null
  }

  private sendBytes(bytes: Uint8Array): void {
    const ws = this.ws
    if (ws?.readyState !== WebSocket.OPEN) return
    for (let i = 0; i < bytes.length; i += CHUNK) ws.send(bytes.subarray(i, i + CHUNK))
  }

  private control(msg: PtyClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg))
  }
}
