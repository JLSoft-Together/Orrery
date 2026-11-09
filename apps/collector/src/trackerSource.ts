import type { Agent, TelegramAsk, TerminalSession, ToolSpan } from '@ccm/shared'
import { createCoreTranscript, type CoreTranscriptClient } from './coreTranscript'
import { CoreTracker, type CoreHub } from './coreTracker'
import type { PeerInbound, PeerSent } from './peers'
import type { RegistryRecord } from './registry'
import { SessionTracker, type ActivityDraft, type ContextHints, type SyncScope } from './tracker'

export { terminalIdOf, type ActivityDraft, type ContextHints, type SyncScope } from './tracker'

export interface Tracker {
  readonly terminalId: string
  record: RegistryRecord
  shell?: string
  parentPid?: number
  endedAt?: string
  readonly sessionId: string | undefined
  readonly mainAgentId: string
  readonly ended: boolean
  readonly lastReply: string | undefined
  readonly pendingAsk: TelegramAsk | undefined
  setRecord(record: RegistryRecord): { sessionChanged: boolean }
  markEnded(at: string): void
  sync(catchUp: boolean, scope?: SyncScope): Promise<ActivityDraft[]>
  drainSpans(): ToolSpan[]
  drainPeers(): { inbound: PeerInbound[]; sent: PeerSent[] }
  closeOpenSpans(at: string): ToolSpan[]
  toolSpans(sinceMs?: number): ToolSpan[]
  codexCalls(): { agentId: string; at: string }[]
  lastDeviceCallAt(): string | undefined
  lastPortConflict(): { port: number; at: string } | undefined
  buildTerminal(): TerminalSession
  buildAgents(terminal: TerminalSession): Agent[]
  dispose?(): void
  takeChanged?(): boolean
}

export type TrackerMode = 'rust' | 'node'

export function trackerMode(env: NodeJS.ProcessEnv = process.env): TrackerMode {
  return env.CCM_CORE_TRANSCRIPT?.trim().toLowerCase() === 'rust' ? 'rust' : 'node'
}

export const SAFETY_NET_MS = 4_000

let keySeq = 0

export function createCoreHub(makeClient: () => CoreTranscriptClient | null, log: (line: string) => void = (l) => console.warn(l)): CoreHub {
  let client: CoreTranscriptClient | null | undefined
  let logged = false
  const registry = new FinalizationRegistry<string>((key) => client?.send('tracker.close', { key }))
  return {
    client: () => (client === undefined ? (client = makeClient()) : client),
    nextKey: () => `t${++keySeq}`,
    register: (tracker, key) => registry.register(tracker, key, tracker),
    unregister: (tracker) => void registry.unregister(tracker),
    log: (line) => {
      if (logged) return
      logged = true
      log(line)
    },
  }
}

let shared: CoreHub | null = null
let warned = false

function hubFor(env: NodeJS.ProcessEnv): CoreHub | null {
  if (shared) return shared
  const bin = env.CCM_CORE_BIN?.trim()
  if (!bin) {
    if (warned) return null
    warned = true
    console.warn('[ccm] CCM_CORE_TRANSCRIPT=rust but CCM_CORE_BIN is not set, using the Node session tracker')
    return null
  }
  shared = createCoreHub(() => createCoreTranscript({ bin, timeoutMs: 30_000 }))
  return shared
}

export function createTracker(terminalId: string, record: RegistryRecord, projectsDir: string, hints: ContextHints, now: () => number, env: NodeJS.ProcessEnv = process.env): Tracker {
  const hub = trackerMode(env) === 'rust' ? hubFor(env) : null
  return hub ? new CoreTracker(terminalId, record, projectsDir, hints, now, hub, { safetyNetMs: SAFETY_NET_MS }) : new SessionTracker(terminalId, record, projectsDir, hints, now)
}
