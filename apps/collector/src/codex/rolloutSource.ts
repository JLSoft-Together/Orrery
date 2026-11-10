import { reportCoreFallback } from '../coreFallback'
import { createCoreTranscript, type CoreTranscriptClient } from '../coreTranscript'
import { CoreRollouts } from './coreRollout'
import { nodeRollouts } from './nodeRollout'
import type { CodexMeta, CodexSignal } from './rollout'

export interface RolloutSource {
  read(): Promise<CodexSignal[][]>
  close?(): void
}

export interface RolloutSources {
  tail(file: string, fromEnd?: number): RolloutSource
  meta(file: string): Promise<CodexMeta | null>
  once(file: string, fromEnd?: number): Promise<CodexSignal[][]>
}

export type CodexCoreMode = 'rust' | 'node'

export function codexCoreMode(env: NodeJS.ProcessEnv = process.env): CodexCoreMode {
  return env.CCM_CORE_CODEX?.trim().toLowerCase() === 'node' ? 'node' : 'rust'
}

export function createRolloutSources(env: NodeJS.ProcessEnv = process.env): RolloutSources {
  if (codexCoreMode(env) !== 'rust') return nodeRollouts
  const bin = env.CCM_CORE_BIN?.trim()
  if (!bin) {
    console.warn('[ccm] CCM_CORE_BIN is not set, reading Codex rollouts in Node')
    reportCoreFallback('codex', 'CCM_CORE_BIN is not set')
    return nodeRollouts
  }
  let client: CoreTranscriptClient | null | undefined
  return new CoreRollouts({ client: () => (client === undefined ? (client = createCoreTranscript({ bin, timeoutMs: 30_000 })) : client) })
}
