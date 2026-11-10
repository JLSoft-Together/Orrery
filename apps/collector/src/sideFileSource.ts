import { reportCoreFallback } from './coreFallback'
import { createCoreTranscript, type CoreTranscriptClient } from './coreTranscript'
import { CoreSideFiles } from './coreSideFiles'
import type { OutboxEvent } from './deepseek/outbox'
import { nodeSideFiles } from './nodeSideFiles'
import type { HookRecord } from './hooks'
import type { StatusLineRecord } from './statusline'

export interface DirReader<T> {
  read(now: number): Promise<T[]>
}

export interface OutboxTail {
  readEach(onEvent: (event: OutboxEvent) => void): Promise<void>
}

export interface OutboxTailOptions {
  fromEnd?: number
  maxLine?: number
}

export interface SideFileSources {
  hooks(dir: string): DirReader<HookRecord>
  statusLine(dir: string): DirReader<StatusLineRecord>
  outbox(file: string, opts?: OutboxTailOptions): OutboxTail
}

export type SideFilesMode = 'rust' | 'node'

export function sideFilesMode(env: NodeJS.ProcessEnv = process.env): SideFilesMode {
  return env.CCM_CORE_SIDEFILES?.trim().toLowerCase() === 'node' ? 'node' : 'rust'
}

let shared: { bin: string; sources: CoreSideFiles } | null = null

export function createSideFileSources(env: NodeJS.ProcessEnv = process.env): SideFileSources {
  if (sideFilesMode(env) !== 'rust') return nodeSideFiles
  const bin = env.CCM_CORE_BIN?.trim()
  if (!bin) {
    console.warn('[ccm] CCM_CORE_BIN is not set, reading hook / status line / outbox files in Node')
    reportCoreFallback('sidefiles', 'CCM_CORE_BIN is not set')
    return nodeSideFiles
  }
  if (shared?.bin === bin) return shared.sources
  let client: CoreTranscriptClient | null | undefined
  const sources = new CoreSideFiles({ client: () => (client === undefined ? (client = createCoreTranscript({ bin, timeoutMs: 30_000 })) : client) })
  shared = { bin, sources }
  return sources
}
