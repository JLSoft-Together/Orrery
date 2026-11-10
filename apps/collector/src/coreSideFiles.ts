import { reportCoreFallback } from './coreFallback'
import type { CoreMessage, CoreTranscriptClient } from './coreTranscript'
import type { OutboxEvent } from './deepseek/outbox'
import { HookReader, type HookRecord } from './hooks'
import { NodeOutboxTail } from './nodeSideFiles'
import type { DirReader, OutboxTail, OutboxTailOptions, SideFileSources } from './sideFileSource'
import { StatusLineReader, type StatusLineRecord } from './statusline'

export interface CoreSideFilesOptions {
  client: () => CoreTranscriptClient | null
  timeoutMs?: number
  log?: (line: string) => void
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

export class CoreSideFiles implements SideFileSources {
  private failed = false

  constructor(private readonly opts: CoreSideFilesOptions) {}

  get fellBack(): boolean {
    return this.failed
  }

  hooks(dir: string): DirReader<HookRecord> {
    return new CoreDirReader<HookRecord>(this, 'hooks', dir, new HookReader(dir))
  }

  statusLine(dir: string): DirReader<StatusLineRecord> {
    return new CoreDirReader<StatusLineRecord>(this, 'statusline', dir, new StatusLineReader(dir))
  }

  outbox(file: string, opts: OutboxTailOptions = {}): OutboxTail {
    return new CoreOutboxTail(this, file, opts)
  }

  async request(op: string, extra: Record<string, unknown>): Promise<CoreMessage | null> {
    if (this.failed) return null
    const client = this.opts.client()
    if (!client) {
      this.fail('core-transcript did not start')
      return null
    }
    const reply = await client.request(op, extra, { timeoutMs: this.opts.timeoutMs ?? 30_000, restartOnTimeout: false })
    if (!reply && !this.failed) this.fail(`${op} failed`)
    return this.failed ? null : reply
  }

  private fail(reason: string): void {
    this.failed = true
    const log = this.opts.log ?? ((line: string) => console.warn(line))
    log(`[ccm] hook / status line / outbox files: rust core unusable (${reason}), reading in Node`)
    reportCoreFallback('sidefiles', reason)
    this.opts.client()?.dispose()
  }
}

class CoreDirReader<T> implements DirReader<T> {
  constructor(
    private readonly core: CoreSideFiles,
    private readonly kind: 'hooks' | 'statusline',
    private readonly dir: string,
    private readonly node: DirReader<T>,
  ) {}

  async read(now: number): Promise<T[]> {
    if (this.core.fellBack) return this.node.read(now)
    const reply = await this.core.request(`sidefiles.${this.kind}`, { dir: this.dir, now })
    if (reply && Array.isArray(reply.records)) return reply.records.filter(isObj) as T[]
    return this.node.read(now)
  }
}

class CoreOutboxTail implements OutboxTail {
  private started = false
  private offset = 0
  private skipping = false
  private node: NodeOutboxTail | null = null
  private reading: Promise<void> | null = null

  constructor(
    private readonly core: CoreSideFiles,
    private readonly file: string,
    private readonly opts: OutboxTailOptions,
  ) {}

  get usesNode(): boolean {
    return this.node !== null
  }

  readEach(onEvent: (event: OutboxEvent) => void): Promise<void> {
    if (!this.reading) {
      this.reading = this.readNew(onEvent).finally(() => {
        this.reading = null
      })
    }
    return this.reading
  }

  private async readNew(onEvent: (event: OutboxEvent) => void): Promise<void> {
    if (!this.node && !this.core.fellBack && (await this.readCore(onEvent))) return
    return this.toNode().readEach(onEvent)
  }

  private async readCore(onEvent: (event: OutboxEvent) => void): Promise<boolean> {
    for (;;) {
      const reply = await this.core.request('sidefiles.outbox', {
        file: this.file,
        started: this.started,
        offset: this.offset,
        skipping: this.skipping,
        fromEnd: this.opts.fromEnd,
        maxLine: this.opts.maxLine,
      })
      if (!reply) return false
      if (reply.missing === true) return true
      if (!Array.isArray(reply.records) || typeof reply.offset !== 'number') return false
      this.started = true
      this.offset = reply.offset
      this.skipping = reply.skipping === true
      for (const r of reply.records) if (isObj(r)) onEvent(r as unknown as OutboxEvent)
      if (reply.more !== true) return true
    }
  }

  private toNode(): NodeOutboxTail {
    if (this.node) return this.node
    const node = new NodeOutboxTail(this.file, this.started ? { maxLine: this.opts.maxLine } : this.opts)
    if (this.started) node.tailer.resumeAt(this.offset, this.skipping)
    this.node = node
    return node
  }
}
