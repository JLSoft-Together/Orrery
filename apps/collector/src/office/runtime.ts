import type { OfficeHub } from './hub'

export interface OfficeParts {
  hub: Pick<OfficeHub, 'start' | 'stop' | 'subscribe' | 'dropClient'>
  recorder: { start(): void; stop(): void }
  flush?: () => void
}

export class OfficeRuntime {
  private readonly enabled = new Set<string>()
  private readonly subscribed = new Set<string>()
  private parts: OfficeParts | null = null
  private ready = false
  private closed = false

  constructor(private readonly create: () => OfficeParts) {}

  get active(): boolean {
    return this.parts !== null
  }

  setReady(): void {
    this.ready = true
    this.sync()
  }

  close(): void {
    this.closed = true
    this.sync()
  }

  setEnabled(clientId: string, on: boolean): void {
    if (on) this.enabled.add(clientId)
    else this.enabled.delete(clientId)
    this.sync()
  }

  subscribe(clientId: string, on: boolean): void {
    if (on) this.subscribed.add(clientId)
    else this.subscribed.delete(clientId)
    this.parts?.hub.subscribe(clientId, on)
  }

  dropClient(clientId: string): void {
    this.enabled.delete(clientId)
    this.subscribed.delete(clientId)
    this.parts?.hub.dropClient(clientId)
    this.sync()
  }

  private sync(): void {
    const want = this.ready && !this.closed && this.enabled.size > 0
    if (want && !this.parts) {
      const parts = this.create()
      this.parts = parts
      parts.recorder.start()
      parts.hub.start()
      for (const id of this.subscribed) parts.hub.subscribe(id, true)
    } else if (!want && this.parts) {
      const parts = this.parts
      this.parts = null
      parts.hub.stop()
      parts.recorder.stop()
      parts.flush?.()
    }
  }
}
