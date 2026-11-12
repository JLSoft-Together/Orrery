import { REMOTE_ATTACH_MAX } from '../lib/limits'
import { RemoteCommandError, type JsonMap } from '../remote/types'

export interface PtyTransport {
  ready(): boolean
  request(type: string, fields: JsonMap): Promise<JsonMap>
  sendInput(stream: number, bytes: Uint8Array, claim: boolean): void
}

export interface PtyHandlers {
  onOutput(bytes: Uint8Array): void
  onEvent(event: JsonMap): void
}

export interface PtyAttachment {
  readonly stream: number
  readonly sessionId: string
  cols: number
  rows: number
  suspended: boolean
  pending: Uint8Array[]
  pendingClaim: boolean
  timer: ReturnType<typeof setTimeout> | null
  handlers: PtyHandlers
}

export interface PtyOptions {
  coalesceMs?: number
  limit?: number
}

function concat(chunks: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0))
  let at = 0
  for (const chunk of chunks) {
    out.set(chunk, at)
    at += chunk.length
  }
  return out
}

export class PtyStreams {
  private readonly entries = new Map<number, PtyAttachment>()
  private readonly transport: PtyTransport
  private readonly coalesceMs: number
  private readonly limit: number
  private next = 1

  constructor(transport: PtyTransport, options: PtyOptions = {}) {
    this.transport = transport
    this.coalesceMs = options.coalesceMs ?? 30
    this.limit = options.limit ?? REMOTE_ATTACH_MAX
  }

  get count(): number {
    return this.entries.size
  }

  async attach(sessionId: string, cols: number, rows: number, handlers: PtyHandlers): Promise<PtyAttachment> {
    if (this.entries.size >= this.limit) throw new RemoteCommandError('limit')
    const entry: PtyAttachment = { stream: this.next++, sessionId, cols, rows, suspended: false, pending: [], pendingClaim: false, timer: null, handlers }
    this.entries.set(entry.stream, entry)
    if (this.transport.ready()) {
      try {
        await this.sendAttach(entry)
      } catch (error) {
        this.entries.delete(entry.stream)
        throw error
      }
    }
    return entry
  }

  private async sendAttach(entry: PtyAttachment): Promise<void> {
    await this.transport.request('pty.attach', { stream: entry.stream, sessionId: entry.sessionId, cols: entry.cols, rows: entry.rows })
  }

  async detach(entry: PtyAttachment): Promise<void> {
    if (!this.entries.delete(entry.stream)) return
    this.clearInput(entry)
    if (!entry.suspended && this.transport.ready()) {
      try {
        await this.transport.request('pty.detach', { stream: entry.stream })
      } catch {
        return
      }
    }
  }

  private clearInput(entry: PtyAttachment): void {
    if (entry.timer) clearTimeout(entry.timer)
    entry.timer = null
    entry.pending = []
    entry.pendingClaim = false
  }

  input(entry: PtyAttachment, bytes: Uint8Array, claim: boolean): void {
    if (bytes.length === 0 || entry.suspended || !this.transport.ready()) return
    if (!entry.timer) {
      this.transport.sendInput(entry.stream, bytes, claim)
      entry.timer = setTimeout(() => this.flush(entry), this.coalesceMs)
      return
    }
    entry.pending.push(bytes)
    entry.pendingClaim = entry.pendingClaim || claim
  }

  private flush(entry: PtyAttachment): void {
    entry.timer = null
    if (entry.pending.length === 0) return
    const bytes = concat(entry.pending)
    const claim = entry.pendingClaim
    entry.pending = []
    entry.pendingClaim = false
    if (!this.transport.ready()) return
    this.transport.sendInput(entry.stream, bytes, claim)
    entry.timer = setTimeout(() => this.flush(entry), this.coalesceMs)
  }

  async resize(entry: PtyAttachment, cols: number, rows: number): Promise<void> {
    entry.cols = cols
    entry.rows = rows
    if (entry.suspended || !this.transport.ready()) return
    try {
      await this.transport.request('pty.resize', { stream: entry.stream, cols, rows })
    } catch {
      return
    }
  }

  feed(stream: number, bytes: Uint8Array): void {
    this.entries.get(stream)?.handlers.onOutput(bytes)
  }

  event(event: JsonMap): void {
    const type = event.t
    if (typeof type !== 'string' || !type.startsWith('pty.') || typeof event.stream !== 'number') return
    const entry = this.entries.get(event.stream)
    if (!entry) return
    entry.handlers.onEvent(event)
    if (type === 'pty.overflow') void this.reattach(entry)
  }

  private async reattach(entry: PtyAttachment): Promise<void> {
    if (entry.suspended || !this.transport.ready()) return
    try {
      await this.transport.request('pty.detach', { stream: entry.stream })
    } catch (error) {
      if (!(error instanceof RemoteCommandError) || error.code !== 'notFound') return
    }
    await this.attachOne(entry)
  }

  private async attachOne(entry: PtyAttachment): Promise<void> {
    if (!this.entries.has(entry.stream) || entry.suspended || !this.transport.ready()) return
    try {
      await this.sendAttach(entry)
    } catch (error) {
      entry.handlers.onEvent({ t: 'pty.error', stream: entry.stream, error: error instanceof RemoteCommandError ? error.code : 'internal' })
    }
  }

  async onReady(): Promise<void> {
    for (const entry of [...this.entries.values()]) {
      if (!entry.suspended) await this.attachOne(entry)
    }
  }

  async suspendAll(): Promise<void> {
    for (const entry of this.entries.values()) {
      const was = entry.suspended
      entry.suspended = true
      this.clearInput(entry)
      if (!was && this.transport.ready()) {
        try {
          await this.transport.request('pty.detach', { stream: entry.stream })
        } catch {
          continue
        }
      }
    }
  }

  async resumeAll(): Promise<void> {
    for (const entry of this.entries.values()) entry.suspended = false
    if (this.transport.ready()) await this.onReady()
  }

  dispose(): void {
    for (const entry of this.entries.values()) this.clearInput(entry)
    this.entries.clear()
  }
}
