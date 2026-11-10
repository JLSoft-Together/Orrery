import { reportCoreFallback } from '../coreFallback'
import type { CoreMessage, CoreTranscriptClient } from '../coreTranscript'
import { NodeRolloutSource } from './nodeRollout'
import { readSessionMeta } from './reader'
import type { CodexMeta, CodexSignal } from './rollout'
import type { RolloutSource, RolloutSources } from './rolloutSource'

export interface CoreRolloutOptions {
  client: () => CoreTranscriptClient | null
  timeoutMs?: number
  maxBytes?: number
  chunkBytes?: number
  log?: (line: string) => void
}

export class CoreRollouts implements RolloutSources {
  private failed = false

  constructor(private readonly opts: CoreRolloutOptions) {}

  get fellBack(): boolean {
    return this.failed
  }

  get maxBytes(): number | undefined {
    return this.opts.maxBytes
  }

  get chunkBytes(): number | undefined {
    return this.opts.chunkBytes
  }

  tail(file: string, fromEnd?: number): RolloutSource {
    return new CoreRolloutSource(this, file, fromEnd)
  }

  async meta(file: string): Promise<CodexMeta | null> {
    const reply = await this.request('codex.meta', { file })
    return reply ? ((reply.meta as CodexMeta | null | undefined) ?? null) : readSessionMeta(file)
  }

  async once(file: string, fromEnd?: number): Promise<CodexSignal[][]> {
    const source = new CoreRolloutSource(this, file, fromEnd)
    try {
      return await source.read()
    } finally {
      source.close()
    }
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

  send(op: string, extra: Record<string, unknown>): void {
    if (!this.failed) this.opts.client()?.send(op, extra)
  }

  private fail(reason: string): void {
    this.failed = true
    const log = this.opts.log ?? ((line: string) => console.warn(line))
    log(`[ccm] codex rollouts: rust core unusable (${reason}), reading in Node`)
    reportCoreFallback('codex', reason)
    this.opts.client()?.dispose()
  }
}

export class CoreRolloutSource implements RolloutSource {
  private handle: number | null = null
  private node: NodeRolloutSource | null = null
  private resume: { at: number; skipping: boolean } | null = null
  private reading: Promise<CodexSignal[][]> | null = null
  private closed = false

  constructor(
    private readonly core: CoreRollouts,
    readonly file: string,
    private readonly fromEnd?: number,
  ) {}

  get usesNode(): boolean {
    return this.node !== null
  }

  read(): Promise<CodexSignal[][]> {
    if (!this.reading) {
      this.reading = this.readNew().finally(() => {
        this.reading = null
        if (this.closed) this.release()
      })
    }
    return this.reading
  }

  close(): void {
    this.closed = true
    if (!this.reading) this.release()
  }

  private release(): void {
    if (this.handle === null) return
    this.core.send('codex.close', { handle: this.handle })
    this.handle = null
  }

  private async readNew(): Promise<CodexSignal[][]> {
    const out: CodexSignal[][] = []
    if (this.closed) return out
    if (!this.node && !this.core.fellBack && (await this.readCore(out))) return out
    if (this.closed) return out
    return out.concat(await this.toNode().read())
  }

  private async readCore(out: CodexSignal[][]): Promise<boolean> {
    if (this.handle === null) {
      const opened = await this.core.request('codex.open', { file: this.file, fromEnd: Number.isFinite(this.fromEnd) ? this.fromEnd : undefined, chunkBytes: this.core.chunkBytes })
      if (typeof opened?.handle !== 'number') return false
      this.handle = opened.handle
    }
    for (;;) {
      if (this.closed) return true
      const reply = await this.core.request('codex.read', { handle: this.handle, maxBytes: this.core.maxBytes })
      if (!reply) return false
      for (const r of reply.records as CodexSignal[][]) out.push(r)
      if (typeof reply.resumeAt === 'number') this.resume = { at: reply.resumeAt, skipping: reply.skipping === true }
      if (reply.more !== true) return true
    }
  }

  private toNode(): NodeRolloutSource {
    if (this.node) return this.node
    const node = new NodeRolloutSource(this.file, this.fromEnd, this.core.chunkBytes)
    if (this.resume) node.tailer.resumeAt(this.resume.at, this.resume.skipping)
    this.node = node
    this.handle = null
    return node
  }
}
