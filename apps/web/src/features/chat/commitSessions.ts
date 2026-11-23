import { shallowRef, type Ref, type ShallowRef } from 'vue'
import type { CommitSessionChip, CommitSessions } from '@ccm/shared'
import { bridgeCall, hasChatView, type ChatCall, type ChatTarget } from './feed'

export interface CommitRef {
  hash: string
  time: number
}

export const COMMIT_BATCH = 60
export const COMMIT_WAIT_MS = 250

type Obj = Record<string, unknown>
const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v)
const str = (v: unknown, max: number): string | null => (typeof v === 'string' && v.length > 0 ? v.slice(0, max) : null)
const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : null)

function parseChip(raw: unknown): CommitSessionChip | null {
  if (!isObj(raw)) return null
  const agent = raw.agent === 'claude' || raw.agent === 'codex' ? raw.agent : null
  const session = str(raw.session, 100)
  const turn = num(raw.turn)
  const score = num(raw.score)
  if (!agent || !session || turn === null || score === null) return null
  return { agent, session, label: str(raw.label, 40) ?? '', cwd: str(raw.cwd, 1024) ?? '', turn: Math.floor(turn), score: Math.min(1, score), strong: raw.strong === true, prompt: str(raw.prompt, 120) ?? '', at: num(raw.at) ?? 0 }
}

export function parseCommitSessions(raw: unknown): Map<string, CommitSessions> | null {
  if (!isObj(raw) || raw.ok !== true || !isObj(raw.commits)) return null
  const out = new Map<string, CommitSessions>()
  for (const [hash, v] of Object.entries(raw.commits)) {
    if (!isObj(v) || !Array.isArray(v.chips)) continue
    const chips = v.chips.flatMap((c) => parseChip(c) ?? []).slice(0, 2)
    if (chips.length) out.set(hash.toLowerCase(), { chips, more: Math.floor(num(v.more) ?? 0) })
  }
  return out
}

export function chipTarget(chip: CommitSessionChip): ChatTarget {
  return { key: `commit-chat:${chip.agent}:${chip.session}`, agent: chip.agent, session: chip.session, cwd: chip.cwd }
}

export function chipPercent(chip: CommitSessionChip): number {
  return Math.round(chip.score * 100)
}

export interface CommitSessionsApi {
  sessions: ShallowRef<ReadonlyMap<string, CommitSessions>>
  want: (commits: readonly CommitRef[]) => void
  reset: () => void
}

export function useCommitSessions(root: Ref<string | null>, call: ChatCall = bridgeCall, enabled = hasChatView): CommitSessionsApi {
  const sessions = shallowRef<ReadonlyMap<string, CommitSessions>>(new Map())
  const asked = new Set<string>()
  let queue: CommitRef[] = []
  let timer: ReturnType<typeof setTimeout> | undefined
  let epoch = 0

  async function flush(): Promise<void> {
    timer = undefined
    const at = root.value
    const mine = epoch
    const batch = queue.splice(0, COMMIT_BATCH)
    if (queue.length) timer = setTimeout(() => void flush(), COMMIT_WAIT_MS)
    if (!at || !batch.length) return
    let got: Map<string, CommitSessions> | null = null
    try {
      got = parseCommitSessions(await call({ op: 'commits', root: at, commits: batch.map((c) => ({ hash: c.hash, time: c.time })) }))
    } catch {
      got = null
    }
    if (mine !== epoch || !got || !got.size) return
    const next = new Map(sessions.value)
    for (const [hash, v] of got) next.set(hash, v)
    sessions.value = next
  }

  function want(commits: readonly CommitRef[]): void {
    if (!enabled || !root.value) return
    for (const c of commits) {
      const hash = c.hash.toLowerCase()
      if (asked.has(hash)) continue
      asked.add(hash)
      queue.push({ hash, time: c.time })
    }
    if (queue.length && !timer) timer = setTimeout(() => void flush(), COMMIT_WAIT_MS)
  }

  function reset(): void {
    epoch++
    asked.clear()
    queue = []
    clearTimeout(timer)
    timer = undefined
    sessions.value = new Map()
  }

  return { sessions, want, reset }
}

export interface ChatDialogRequest {
  chip: CommitSessionChip
  hash: string
}
