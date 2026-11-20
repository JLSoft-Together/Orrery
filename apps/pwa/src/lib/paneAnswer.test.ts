import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ANSWER_STALE_MS, ANSWER_UNDO_MS, answerChoices, askKind, askText, createAnswerFlow, type AnswerDeps } from './paneAnswer'
import type { WsPane } from './workspaceModel'

const pane = (over: Partial<WsPane> = {}): WsPane => ({
  key: 'p1',
  title: 'Fix',
  provider: 'claude',
  rail: 'asked',
  tool: null,
  toolSummary: null,
  waitText: 'permission',
  subagents: 0,
  cwd: 'D:\\code',
  branch: null,
  sessionId: 's1',
  terminalId: 't1',
  isTerm: true,
  canAttach: true,
  asked: true,
  since: 1000,
  attention: 'waiting',
  inbox: { reason: 'permission' },
  ...over,
})

describe('card answer choices', () => {
  it('offers Yes / No only for a permission prompt', () => {
    expect(askKind(pane())).toBe('permission')
    expect(answerChoices(pane())).toEqual([
      { keys: 'approve', label: 'yes' },
      { keys: 'deny', label: 'no' },
    ])
    expect(askKind(pane({ waitText: 'question', inbox: { reason: 'menu' } }))).toBe('menu')
    expect(answerChoices(pane({ waitText: 'question', inbox: { reason: 'menu' } }))).toEqual([])
    expect(askKind(pane({ waitText: 'other', inbox: null }))).toBe('question')
    expect(askKind(pane({ rail: 'working' }))).toBeNull()
  })

  it('shows a real question text but never a raw reason key', () => {
    expect(askText(pane())).toBeNull()
    expect(askText(pane({ waitText: 'Chạy npm test?' }))).toBe('Chạy npm test?')
  })
})

describe('answer flow: undo first, then send', () => {
  let now = 100_000
  let current: WsPane | null
  let sent: Array<{ key: string; keys: string }>
  let refreshes: number
  const deps = (over: Partial<AnswerDeps> = {}): AnswerDeps => ({
    now: () => now,
    dataAt: () => now - 1000,
    refresh: async () => {
      refreshes++
    },
    read: () => current,
    send: async (p, keys) => {
      sent.push({ key: p.key, keys })
    },
    ...over,
  })

  beforeEach(() => {
    vi.useFakeTimers()
    now = 100_000
    current = pane()
    sent = []
    refreshes = 0
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('sends the keys only after the 3 second undo window', async () => {
    const flow = createAnswerFlow(deps())
    const done = flow.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(0)
    expect(flow.pending.value).toMatchObject({ paneKey: 'p1', keys: 'approve' })
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS - 1)
    expect(sent).toEqual([])
    await vi.advanceTimersByTimeAsync(1)
    expect(await done).toEqual({ outcome: 'sent' })
    expect(sent).toEqual([{ key: 'p1', keys: 'approve' }])
    expect(flow.pending.value).toBeNull()
  })

  it('Undo cancels before anything is sent', async () => {
    const flow = createAnswerFlow(deps())
    const done = flow.press(pane(), 'deny')
    await vi.advanceTimersByTimeAsync(1000)
    flow.undo()
    expect(await done).toEqual({ outcome: 'cancelled' })
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS * 2)
    expect(sent).toEqual([])
  })

  it('refuses a second press while one is pending', async () => {
    const flow = createAnswerFlow(deps())
    void flow.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(0)
    expect(await flow.press(pane(), 'deny')).toEqual({ outcome: 'busy' })
    flow.dispose()
    expect(sent).toEqual([])
  })

  it('does not send when the question changed during the undo window', async () => {
    const flow = createAnswerFlow(deps())
    const done = flow.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(1000)
    current = pane({ since: 2000 })
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS)
    expect(await done).toEqual({ outcome: 'changed' })
    expect(sent).toEqual([])
  })

  it('re-reads data older than 30 seconds first and stops when it changed', async () => {
    const flow = createAnswerFlow(
      deps({
        dataAt: () => now - ANSWER_STALE_MS - 1,
        refresh: async () => {
          refreshes++
          current = pane({ rail: 'working', inbox: null })
        },
      }),
    )
    expect(await flow.press(pane(), 'approve')).toEqual({ outcome: 'changed' })
    expect(refreshes).toBe(1)
    expect(flow.pending.value).toBeNull()
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS * 2)
    expect(sent).toEqual([])
  })

  it('re-reads stale data and still waits for the undo window when nothing changed', async () => {
    const flow = createAnswerFlow(deps({ dataAt: () => 0 }))
    const done = flow.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(0)
    expect(refreshes).toBe(1)
    expect(sent).toEqual([])
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS)
    expect(await done).toEqual({ outcome: 'sent' })
    expect(sent).toEqual([{ key: 'p1', keys: 'approve' }])
  })

  it('does not refresh fresh data', async () => {
    const flow = createAnswerFlow(deps())
    const done = flow.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS)
    await done
    expect(refreshes).toBe(0)
  })

  it('reports a failed refresh or send without retrying', async () => {
    const boom = new Error('offline')
    const stale = createAnswerFlow(deps({ dataAt: () => 0, refresh: async () => Promise.reject(boom) }))
    expect(await stale.press(pane(), 'approve')).toEqual({ outcome: 'failed', error: boom })
    let calls = 0
    const failing = createAnswerFlow(deps({ send: async () => { calls++; throw boom } }))
    const done = failing.press(pane(), 'approve')
    await vi.advanceTimersByTimeAsync(ANSWER_UNDO_MS)
    expect(await done).toEqual({ outcome: 'failed', error: boom })
    expect(calls).toBe(1)
    expect(sent).toEqual([])
  })
})
