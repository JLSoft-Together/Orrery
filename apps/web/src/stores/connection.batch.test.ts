import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { Agent, MonitorSnapshot, ServerMessage, TerminalSession } from '@ccm/shared'
import { useAgentsStore } from './agents'
import { useConnectionStore } from './connection'
import { useTerminalsStore } from './terminals'
import { useToolSpansStore } from './toolSpans'
import { useUiStore } from './ui'

const terminal = (id: string, over: Partial<TerminalSession> = {}): TerminalSession => ({
  id,
  title: id,
  status: 'working',
  startedAt: '2026-10-07T10:00:00.000Z',
  ...over,
})

const agent = (id: string, terminalId: string, over: Partial<Agent> = {}): Agent => ({
  id,
  terminalId,
  role: 'subagent',
  status: 'working',
  parentId: `${terminalId}:main`,
  ...over,
})

const snapshot = (terminals: TerminalSession[] = [terminal('t1')], agents: Agent[] = []): MonitorSnapshot => ({
  protocolVersion: 1,
  collectorStartedAt: '2026-10-07T10:00:00.000Z',
  supportedClaudeVersion: '2.1.289',
  terminals,
  agents,
  activity: [],
})

const msg = (seq: number, events: ServerMessage['events'], direct = false) => JSON.stringify({ seq, events, ...(direct ? { direct: true } : {}) })

describe('connection batching', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    setActivePinia(createPinia())
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('holds incremental events until the frame, then applies them in one go', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    expect(terminals.byId.t1).toBeDefined()

    conn.receive(msg(2, [{ type: 'terminal.updated', payload: { id: 't1', title: 'one' } }]))
    conn.receive(msg(3, [{ type: 'terminal.updated', payload: { id: 't1', title: 'two' } }]))
    expect(terminals.byId.t1?.title).toBe('t1')

    vi.advanceTimersByTime(60)
    expect(terminals.byId.t1?.title).toBe('two')
  })

  it('applies a pending batch by timer when animation frames never run', () => {
    vi.stubGlobal('requestAnimationFrame', () => 1)
    vi.stubGlobal('cancelAnimationFrame', () => undefined)
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'terminal.updated', payload: { id: 't1', title: 'later' } }]))
    expect(terminals.byId.t1?.title).toBe('t1')
    vi.advanceTimersByTime(60)
    expect(terminals.byId.t1?.title).toBe('later')
    vi.unstubAllGlobals()
  })

  it('keeps event order for removed then created with the same id in one batch', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'terminal.removed', payload: { id: 't1' } }]))
    conn.receive(msg(3, [{ type: 'terminal.created', payload: terminal('t1', { title: 'reborn' }) }]))
    conn.receive(msg(4, [{ type: 'terminal.updated', payload: { id: 't1', status: 'idle' } }]))
    vi.advanceTimersByTime(60)
    expect(terminals.byId.t1).toMatchObject({ title: 'reborn', status: 'idle' })
  })

  it('a created then removed pair in one batch leaves nothing behind', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'agent.created', payload: agent('a1', 't1') }]))
    conn.receive(msg(3, [{ type: 'agent.removed', payload: { id: 'a1' } }]))
    vi.advanceTimersByTime(60)
    expect(agents.byId.a1).toBeUndefined()
  })

  it('a snapshot flushes the queue first and wins over later events only by order', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'terminal.created', payload: terminal('t2') }]))
    conn.receive(msg(3, [{ type: 'snapshot', payload: snapshot([terminal('t9')]) }]))
    expect(Object.keys(terminals.byId)).toEqual(['t9'])
    conn.receive(msg(4, [{ type: 'terminal.created', payload: terminal('t3') }]))
    vi.advanceTimersByTime(60)
    expect(Object.keys(terminals.byId).sort()).toEqual(['t3', 't9'])
  })

  it('direct replies apply after the events queued before them', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'terminal.updated', payload: { id: 't1', title: 'queued' } }]))
    conn.receive(msg(0, [{ type: 'terminal.updated', payload: { id: 't1', title: 'direct' } }], true))
    expect(terminals.byId.t1?.title).toBe('direct')
    vi.advanceTimersByTime(60)
    expect(terminals.byId.t1?.title).toBe('direct')
  })

  it('a sequence gap still triggers a resync and drops the message', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(5, [{ type: 'terminal.created', payload: terminal('gap') }]))
    vi.advanceTimersByTime(60)
    expect(terminals.byId.gap).toBeUndefined()
  })

  it('flush applies what is pending right away', () => {
    const conn = useConnectionStore()
    const terminals = useTerminalsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    conn.receive(msg(2, [{ type: 'terminal.updated', payload: { id: 't1', title: 'now' } }]))
    conn.flush()
    expect(terminals.byId.t1?.title).toBe('now')
    vi.advanceTimersByTime(60)
    expect(terminals.byId.t1?.title).toBe('now')
  })
})

describe('connection batching, store level', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    setActivePinia(createPinia())
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('many patches for one agent in a batch write one new frozen object with the final fields', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1')]) }]))
    const before = agents.byId.a1
    expect(Object.isFrozen(before)).toBe(true)
    conn.receive(msg(2, [{ type: 'agent.updated', payload: { id: 'a1', currentTool: 'Bash' } }]))
    conn.receive(msg(3, [{ type: 'agent.updated', payload: { id: 'a1', currentTool: 'Read', status: 'idle' } }]))
    conn.receive(msg(4, [{ type: 'agent.updated', payload: { id: 'a1', status: 'teleporting' as never } }]))
    vi.advanceTimersByTime(60)
    const after = agents.byId.a1
    expect(after).not.toBe(before)
    expect(after).toMatchObject({ currentTool: 'Read', status: 'unknown' })
    expect(before).toMatchObject({ status: 'working' })
    expect(before?.currentTool).toBeUndefined()
    expect(Object.isFrozen(after)).toBe(true)
  })

  it('a patch that changes nothing keeps the same object', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1', { currentTool: 'Bash' })]) }]))
    const before = agents.byId.a1
    conn.receive(msg(2, [{ type: 'agent.updated', payload: { id: 'a1', currentTool: 'Bash', status: 'working' } }]))
    vi.advanceTimersByTime(60)
    expect(agents.byId.a1).toBe(before)
  })

  it('mixes created, updated and removed ids in one batch', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1'), agent('a2', 't1')]) }]))
    conn.receive(
      msg(2, [
        { type: 'agent.removed', payload: { id: 'a1' } },
        { type: 'agent.created', payload: agent('a3', 't1') },
        { type: 'agent.updated', payload: { id: 'a3', currentTool: 'Edit' } },
        { type: 'agent.updated', payload: { id: 'a2', status: 'completed' } },
        { type: 'agent.updated', payload: { id: 'ghost', status: 'idle' } },
      ]),
    )
    vi.advanceTimersByTime(60)
    expect(Object.keys(agents.byId).sort()).toEqual(['a2', 'a3'])
    expect(agents.byId.a3).toMatchObject({ currentTool: 'Edit', status: 'working' })
    expect(agents.byId.a2?.status).toBe('completed')
  })

  it('a patch after a removal in the same batch is dropped, a recreate after it wins', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1', { currentTool: 'Bash' })]) }]))
    conn.receive(msg(2, [{ type: 'agent.removed', payload: { id: 'a1' } }, { type: 'agent.updated', payload: { id: 'a1', currentTool: 'Read' } }]))
    vi.advanceTimersByTime(60)
    expect(agents.byId.a1).toBeUndefined()
    conn.receive(msg(3, [{ type: 'agent.created', payload: agent('a1', 't1', { currentTool: 'Bash' }) }]))
    vi.advanceTimersByTime(60)
    conn.receive(msg(4, [{ type: 'agent.removed', payload: { id: 'a1' } }, { type: 'agent.created', payload: agent('a1', 't1', { status: 'idle' }) }]))
    vi.advanceTimersByTime(60)
    expect(agents.byId.a1).toMatchObject({ status: 'idle' })
    expect(agents.byId.a1?.currentTool).toBeUndefined()
  })

  it('computeds keep tracking after objects are replaced instead of mutated', () => {
    const conn = useConnectionStore()
    const agents = useAgentsStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('t1:main', 't1', { role: 'main', parentId: undefined }), agent('a1', 't1')]) }]))
    expect(agents.byTerminal.t1?.map((a) => a.status)).toEqual(['working', 'working'])
    conn.receive(msg(2, [{ type: 'agent.updated', payload: { id: 'a1', status: 'completed' } }]))
    vi.advanceTimersByTime(60)
    expect(agents.byTerminal.t1?.map((a) => a.status)).toEqual(['working', 'completed'])
    expect(agents.subagents).toHaveLength(1)
  })

  it('deselects a removed agent after the batch, but not one a later snapshot brings back', () => {
    const conn = useConnectionStore()
    const ui = useUiStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1')]) }]))
    ui.selectAgent('a1')
    conn.receive(msg(2, [{ type: 'agent.removed', payload: { id: 'a1' } }]))
    vi.advanceTimersByTime(60)
    expect(ui.selectedAgentId).toBeNull()
    conn.receive(msg(3, [{ type: 'agent.created', payload: agent('a1', 't1') }]))
    vi.advanceTimersByTime(60)
    ui.selectAgent('a1')
    conn.onMessage(msg(4, [{ type: 'agent.removed', payload: { id: 'a1' } }, { type: 'snapshot', payload: snapshot([terminal('t1')], [agent('a1', 't1')]) }]))
    expect(ui.selectedAgentId).toBe('a1')
  })

  it('tool spans of one batch bump the version once', () => {
    const conn = useConnectionStore()
    const spans = useToolSpansStore()
    conn.receive(msg(1, [{ type: 'snapshot', payload: snapshot() }]))
    const v = spans.version
    const span = (id: string) => ({ id, terminalId: 't1', agentId: 't1:main', tool: 'Bash', start: '2026-10-09T10:00:00.000Z' })
    conn.receive(msg(2, [{ type: 'tool.span', payload: span('s1') }, { type: 'tool.span', payload: span('s2') }, { type: 'tool.span', payload: span('s3') }]))
    vi.advanceTimersByTime(60)
    expect(spans.version).toBe(v + 1)
    expect(spans.spansFor('t1')).toHaveLength(3)
  })
})
