import type { RemoteHub } from '../remote/hub'
import { RemoteCommandError } from '../remote/types'
import { PtyStreams } from './ptyStreams'

export class PtyRegistry {
  private readonly streams = new Map<string, PtyStreams>()
  private readonly hub: RemoteHub
  private readonly offs: Array<() => void>

  constructor(hub: RemoteHub) {
    this.hub = hub
    this.offs = [
      hub.pty.on((chunk) => this.streams.get(chunk.machineId)?.feed(chunk.stream, chunk.bytes)),
      hub.device.on((item) => this.streams.get(item.machineId)?.event(item.event)),
      hub.events.on((event) => {
        if (event.kind === 'link' && event.link.status === 'ready') void this.streams.get(event.machineId)?.onReady()
        if (event.kind === 'machines') this.prune()
      }),
    ]
  }

  for(machineId: string): PtyStreams {
    let streams = this.streams.get(machineId)
    if (!streams) {
      streams = new PtyStreams({
        ready: () => this.hub.link(machineId).status === 'ready' && this.hub.session(machineId) !== null,
        request: (type, fields) => {
          const session = this.hub.session(machineId)
          return session ? session.request(type, fields) : Promise.reject(new RemoteCommandError('offline'))
        },
        sendInput: (stream, bytes, claim) => this.hub.session(machineId)?.sendPtyInput(stream, bytes, claim),
      })
      this.streams.set(machineId, streams)
    }
    return streams
  }

  suspendAll(): void {
    for (const streams of this.streams.values()) void streams.suspendAll()
  }

  resumeAll(): void {
    for (const streams of this.streams.values()) void streams.resumeAll()
  }

  private prune(): void {
    const known = new Set(this.hub.machines().map((m) => m.id))
    for (const [id, streams] of this.streams) {
      if (known.has(id)) continue
      streams.dispose()
      this.streams.delete(id)
    }
  }

  dispose(): void {
    for (const off of this.offs) off()
    for (const streams of this.streams.values()) streams.dispose()
    this.streams.clear()
  }
}
