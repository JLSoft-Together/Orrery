import { parseOutboxLine, type OutboxEvent } from './deepseek/outbox'
import { HookReader } from './hooks'
import type { OutboxTail, OutboxTailOptions, SideFileSources } from './sideFileSource'
import { StatusLineReader } from './statusline'
import { JsonlTailer } from './tailer'

export class NodeOutboxTail implements OutboxTail {
  readonly tailer: JsonlTailer

  constructor(file: string, opts: OutboxTailOptions = {}) {
    this.tailer = new JsonlTailer(file, undefined, opts)
  }

  readEach(onEvent: (event: OutboxEvent) => void): Promise<void> {
    return this.tailer.readEach((record) => {
      const e = parseOutboxLine(record)
      if (e) onEvent(e)
    })
  }
}

export const nodeSideFiles: SideFileSources = {
  hooks: (dir) => new HookReader(dir),
  statusLine: (dir) => new StatusLineReader(dir),
  outbox: (file, opts) => new NodeOutboxTail(file, opts),
}
