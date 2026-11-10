import { readSessionMeta, RolloutTailer } from './reader'
import { extractCodexSignals, type CodexSignal } from './rollout'
import type { RolloutSource, RolloutSources } from './rolloutSource'

export class NodeRolloutSource implements RolloutSource {
  readonly tailer: RolloutTailer

  constructor(file: string, fromEnd?: number, chunk?: number) {
    this.tailer = new RolloutTailer(file, fromEnd, chunk)
  }

  async read(): Promise<CodexSignal[][]> {
    return (await this.tailer.read()).map(extractCodexSignals)
  }
}

export const nodeRollouts: RolloutSources = {
  tail: (file, fromEnd) => new NodeRolloutSource(file, fromEnd),
  meta: (file) => readSessionMeta(file),
  once: (file, fromEnd) => new NodeRolloutSource(file, fromEnd).read(),
}
