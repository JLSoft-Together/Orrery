import type { ToolSpan } from '@ccm/shared'
import { SPAN_LIMIT, SPAN_RETENTION_MS } from './tool-spans'

interface Entry {
  span: ToolSpan
  startMs: number
  endMs?: number
}

export type SpanEvent = ToolSpan | { drop: string }

export const isDrop = (e: SpanEvent): e is { drop: string } => typeof (e as { drop?: unknown }).drop === 'string'

export class SpanMirror {
  private spans = new Map<string, Entry>()

  constructor(private readonly now: () => number) {}

  clear(): void {
    this.spans.clear()
  }

  apply(e: SpanEvent): void {
    if (isDrop(e)) {
      this.spans.delete(e.drop)
      return
    }
    const startMs = Date.parse(e.start)
    const endMs = e.end === undefined ? undefined : Date.parse(e.end)
    this.spans.set(e.id, { span: { ...e }, startMs, endMs })
  }

  all(): ToolSpan[] {
    return [...this.spans.values()].map((e) => ({ ...e.span }))
  }

  closeOpen(at: string, agentId?: string): ToolSpan[] {
    const t = Date.parse(at)
    if (Number.isNaN(t)) return []
    const changed: ToolSpan[] = []
    for (const entry of this.spans.values()) {
      if (entry.endMs !== undefined || (agentId !== undefined && entry.span.agentId !== agentId)) continue
      const early = t < entry.startMs
      entry.span.end = early ? entry.span.start : at
      entry.endMs = early ? entry.startMs : t
      entry.span.outcome = 'interrupted'
      changed.push({ ...entry.span })
    }
    return changed
  }

  list(sinceMs?: number): ToolSpan[] {
    this.prune()
    const out: Entry[] = []
    for (const entry of this.spans.values()) {
      if (sinceMs !== undefined && entry.endMs !== undefined && entry.endMs < sinceMs) continue
      out.push(entry)
    }
    return out.sort((a, b) => a.startMs - b.startMs).map((e) => ({ ...e.span }))
  }

  private prune(): void {
    const cutoff = this.now() - SPAN_RETENTION_MS
    for (const [id, entry] of this.spans) if (entry.endMs !== undefined && entry.endMs < cutoff) this.spans.delete(id)
    for (const [id, entry] of this.spans) {
      if (this.spans.size <= SPAN_LIMIT) break
      if (entry.endMs !== undefined) this.spans.delete(id)
    }
  }
}
