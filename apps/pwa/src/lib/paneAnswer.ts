import { ref } from 'vue'
import type { WsPane } from './workspaceModel'

export type AskKind = 'permission' | 'menu' | 'question'
export type AnswerLabel = 'yes' | 'no'

export interface AnswerChoice {
  keys: string
  label: AnswerLabel
}

export type AnswerOutcome = 'sent' | 'cancelled' | 'changed' | 'busy' | 'failed'

export interface AnswerResult {
  outcome: AnswerOutcome
  error?: unknown
}

export interface PendingAnswer {
  paneKey: string
  keys: string
  seq: number
}

export interface AnswerDeps {
  now(): number
  dataAt(): number
  refresh(): Promise<void>
  read(paneKey: string): WsPane | null
  send(pane: WsPane, keys: string): Promise<void>
  schedule?(fn: () => void, ms: number): unknown
  cancel?(handle: unknown): void
}

export const ANSWER_UNDO_MS = 3000
export const ANSWER_STALE_MS = 30_000

const WAIT_KEYS = new Set(['permission', 'question', 'other'])

export function askKind(pane: WsPane): AskKind | null {
  if (pane.rail !== 'asked') return null
  if (pane.inbox?.reason === 'permission' || pane.waitText === 'permission') return 'permission'
  if (pane.inbox?.reason === 'menu') return 'menu'
  return 'question'
}

export function askText(pane: WsPane): string | null {
  const text = pane.waitText?.trim()
  return text && !WAIT_KEYS.has(text) ? text : null
}

export function answerChoices(pane: WsPane): AnswerChoice[] {
  if (askKind(pane) !== 'permission') return []
  return [
    { keys: 'approve', label: 'yes' },
    { keys: 'deny', label: 'no' },
  ]
}

export function askSignature(pane: WsPane): string {
  return JSON.stringify([pane.rail, pane.since, pane.inbox?.reason ?? null, pane.waitText, pane.terminalId, pane.sessionId])
}

export function createAnswerFlow(deps: AnswerDeps) {
  const schedule = deps.schedule ?? ((fn: () => void, ms: number) => setTimeout(fn, ms))
  const cancel = deps.cancel ?? ((handle: unknown) => clearTimeout(handle as ReturnType<typeof setTimeout>))
  const pending = ref<PendingAnswer | null>(null)
  let checking = false
  let seq = 0
  let timer: unknown = null
  let settle: ((result: AnswerResult) => void) | null = null
  let expected = ''

  function finish(result: AnswerResult): void {
    const done = settle
    settle = null
    timer = null
    pending.value = null
    done?.(result)
  }

  async function fire(): Promise<void> {
    const job = pending.value
    if (!job) return
    timer = null
    const fresh = deps.read(job.paneKey)
    if (!fresh || askSignature(fresh) !== expected) {
      finish({ outcome: 'changed' })
      return
    }
    try {
      await deps.send(fresh, job.keys)
      finish({ outcome: 'sent' })
    } catch (error) {
      finish({ outcome: 'failed', error })
    }
  }

  async function press(pane: WsPane, keys: string): Promise<AnswerResult> {
    if (pending.value || checking) return { outcome: 'busy' }
    const sig = askSignature(pane)
    if (deps.now() - deps.dataAt() > ANSWER_STALE_MS) {
      checking = true
      try {
        await deps.refresh()
      } catch (error) {
        return { outcome: 'failed', error }
      } finally {
        checking = false
      }
      const fresh = deps.read(pane.key)
      if (!fresh || askSignature(fresh) !== sig) return { outcome: 'changed' }
    }
    expected = sig
    return new Promise<AnswerResult>((resolve) => {
      settle = resolve
      pending.value = { paneKey: pane.key, keys, seq: ++seq }
      timer = schedule(() => void fire(), ANSWER_UNDO_MS)
    })
  }

  function undo(): void {
    if (!pending.value || timer === null) return
    cancel(timer)
    finish({ outcome: 'cancelled' })
  }

  return { pending, press, undo, dispose: undo }
}

export type AnswerFlow = ReturnType<typeof createAnswerFlow>
