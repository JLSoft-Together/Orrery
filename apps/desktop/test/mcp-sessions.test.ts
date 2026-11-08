import { afterEach, describe, expect, it, vi } from 'vitest'
import { MCP_TOOL_ACTION, ORRERY_ACTIONS, parseMcpPageCall, parseMcpPageResult, type McpPageResult } from '@ccm/shared'
import { handleMcp, type ToolHost } from '../src/pty/mcp'
import { MCP_TOOLS, mcpToolsFor } from '../src/pty/mcpTools'
import { SEND_RATE_MAX, SEND_RATE_MS, WAIT_POLL_MS, WAIT_RATE_MAX } from '../src/pty/mcpTools/sessions'

const fail = (): Promise<never> => Promise.reject(new Error('not expected'))
const hostWith = (over: Partial<ToolHost> = {}): ToolHost => ({ root: 'C:\\p', preset: 'claude', take: () => true, openSession: fail, notify: fail, page: fail, checkoutRoots: () => Promise.resolve([]), ...over })
const call = async (host: ToolHost, name: string, args: unknown = {}) => {
  const res = await handleMcp({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } }, host, '0')
  return (res.json as { result: { content: { text: string }[]; structuredContent: Record<string, unknown>; isError?: boolean } }).result
}

describe('tool list and the action table', () => {
  it('keeps every earlier tool in order and appends the session tools', () => {
    expect(MCP_TOOLS.map((t) => t.def.name)).toEqual([
      'open_session', 'notify_owner', 'open_in_workspace', 'who_is_editing', 'preview_layout', 'list_servers', 'free_port', 'run_app', 'run_status', 'run_log',
      'device_screenshot', 'telegram_notify', 'telegram_send', 'telegram_receive', 'hand_back', 'list_sessions', 'send_to_session', 'close_session', 'wait_session',
    ])
  })

  it('every tool except open_session and wait_session (host-side, built on list_sessions) is one action of the table', () => {
    const names = new Set(MCP_TOOLS.map((t) => t.def.name))
    for (const [tool, action] of Object.entries(MCP_TOOL_ACTION)) {
      expect(names.has(tool), tool).toBe(true)
      expect(Object.hasOwn(ORRERY_ACTIONS, action)).toBe(true)
      expect(ORRERY_ACTIONS[action].sources).toContain('mcp')
    }
    expect([...names].filter((n) => n !== 'open_session' && n !== 'wait_session' && !(n in MCP_TOOL_ACTION))).toEqual([])
  })

  it('DeepSeek pause keeps the session tools and drops only hand_back', () => {
    const off = mcpToolsFor(false).map((t) => t.def.name)
    expect(off).toEqual(expect.arrayContaining(['list_sessions', 'send_to_session', 'close_session', 'wait_session']))
    expect(off).not.toContain('hand_back')
    expect(off).toHaveLength(MCP_TOOLS.length - 1)
  })

  it('schemas are closed and the new tools carry the right hints', () => {
    const by = Object.fromEntries(MCP_TOOLS.map((t) => [t.def.name, t.def]))
    for (const n of ['list_sessions', 'send_to_session', 'close_session', 'wait_session']) expect(by[n]!.inputSchema.additionalProperties).toBe(false)
    expect(by.list_sessions!.annotations.readOnlyHint).toBe(true)
    expect(by.send_to_session!.inputSchema.required).toEqual(['session', 'text'])
    expect(by.close_session!.inputSchema.required).toEqual(['session'])
    expect(by.wait_session!.inputSchema.required).toEqual(['session'])
    expect(by.wait_session!.annotations.readOnlyHint).toBe(true)
    expect(by.wait_session!.inputSchema.properties.timeoutMs).toMatchObject({ minimum: 1000, maximum: 600_000, default: 120_000 })
  })
})

describe('parseMcpPageCall for the session tools', () => {
  const base = { id: 'i', from: 'pty-1', cwd: 'C:\\p' }
  it('cleans the text and rejects empty, oversize and missing ids', () => {
    expect(parseMcpPageCall({ ...base, tool: 'send_to_session', session: 's', text: ' hi\u0007 there ' }, 'send_to_session')).toMatchObject({ text: 'hi there' })
    expect(parseMcpPageCall({ ...base, tool: 'send_to_session', session: 's', text: '\u0007 ' }, 'send_to_session')).toBeNull()
    expect(parseMcpPageCall({ ...base, tool: 'send_to_session', session: 's', text: 'x'.repeat(16_001) }, 'send_to_session')).toBeNull()
    expect(parseMcpPageCall({ ...base, tool: 'send_to_session', text: 'x' }, 'send_to_session')).toBeNull()
    expect(parseMcpPageCall({ ...base, tool: 'close_session', session: '' }, 'close_session')).toBeNull()
    expect(parseMcpPageCall({ ...base, tool: 'list_sessions' }, 'list_sessions')).toMatchObject({ tool: 'list_sessions' })
  })

  it('page results are re-checked: bad rows dropped, names cleaned, new errors kept', () => {
    const raw = {
      ok: true,
      sessions: [
        { sessionId: 'a', name: 'lead\u202e\n', agent: 'claude', status: 'idle', self: true },
        { sessionId: 'b', name: 'x', agent: 'nope', status: 'idle' },
        { sessionId: 'c', name: 'y', agent: 'shell', status: 'Bad Status' },
      ],
    }
    expect(parseMcpPageResult(raw, 'list_sessions')).toEqual({ ok: true, tool: 'list_sessions', sessions: [{ sessionId: 'a', name: 'lead', agent: 'claude', status: 'idle', self: true, openedByYou: false }] })
    expect(parseMcpPageResult({ ok: false, error: 'target' }, 'close_session')).toEqual({ ok: false, error: 'target' })
    expect(parseMcpPageResult({ ok: false, error: 'busy' }, 'send_to_session')).toEqual({ ok: false, error: 'busy' })
    expect(parseMcpPageResult({ ok: true, name: '' }, 'send_to_session')).toEqual({ ok: false, error: 'failed' })
  })

  it('keeps a checked limit and idleSince, drops malformed ones', () => {
    const row = { sessionId: 'a', name: 'n', agent: 'claude', status: 'idle' }
    const at = '2026-11-08T10:00:00.000Z'
    const parse = (extra: object) => (parseMcpPageResult({ ok: true, sessions: [{ ...row, ...extra }] }, 'list_sessions') as { sessions: Record<string, unknown>[] }).sessions[0]
    expect(parse({ limit: { resetsAt: at, window: 'five_hour' }, idleSince: at })).toMatchObject({ limit: { resetsAt: at, window: 'five_hour' }, idleSince: at })
    expect(parse({ limit: {} })!.limit).toEqual({})
    const bad = parse({ limit: { resetsAt: 'soon', window: 'Five Hours!' }, idleSince: 'yesterday' })
    expect(bad!.limit).toEqual({})
    expect(bad).not.toHaveProperty('idleSince')
    expect(parse({ limit: 'yes' })).not.toHaveProperty('limit')
  })
})

describe('list_sessions', () => {
  it('asks the page and lists names, never content', async () => {
    const page = vi.fn(
      (): Promise<McpPageResult> =>
        Promise.resolve({
          ok: true,
          tool: 'list_sessions',
          sessions: [
            { sessionId: 'pty-1', name: 'lead', agent: 'claude', status: 'idle', self: true, openedByYou: false },
            { sessionId: 'pty-2', name: 'ds', agent: 'codex', status: 'working', self: false, openedByYou: true },
          ],
        }),
    )
    const res = await call(hostWith({ page }), 'list_sessions')
    expect(page).toHaveBeenCalledWith({ tool: 'list_sessions' })
    expect(res.isError).toBeUndefined()
    expect(res.content[0]!.text).toContain('"ds" (Codex, working, opened by you) id pty-2')
    expect(res.content[0]!.text).toContain('this is you')
  })

  it('tells idle time and the usage limit', async () => {
    const at = '2026-11-08T10:00:00.000Z'
    const page = (): Promise<McpPageResult> =>
      Promise.resolve({ ok: true, tool: 'list_sessions', sessions: [{ sessionId: 'pty-2', name: 'kid', agent: 'claude', status: 'idle', self: false, openedByYou: true, idleSince: at, limit: { window: 'five_hour', resetsAt: at } }] })
    const res = await call(hostWith({ page }), 'list_sessions')
    expect(res.content[0]!.text).toContain(`idle since ${at}, at usage limit (window five_hour, resets ${at})`)
    expect(res.structuredContent.sessions).toEqual([expect.objectContaining({ idleSince: at, limit: { window: 'five_hour', resetsAt: at } })])
  })

  it('maps a missing caller to an error and a throwing page to failed', async () => {
    expect(await call(hostWith({ page: () => Promise.resolve({ ok: false, error: 'caller' }) }), 'list_sessions')).toMatchObject({ isError: true, structuredContent: { error: 'caller' } })
    expect(await call(hostWith({ page: fail }), 'list_sessions')).toMatchObject({ isError: true, structuredContent: { error: 'failed' } })
  })
})

describe('send_to_session', () => {
  const sent = (): Promise<McpPageResult> => Promise.resolve({ ok: true, tool: 'send_to_session', name: 'ds' })

  it('sends only the cleaned text, with the open_session rate window', async () => {
    const page = vi.fn(sent)
    const take = vi.fn(() => true)
    const res = await call(hostWith({ page, take }), 'send_to_session', { session: 'pty-2', text: ' run\u0007 tests ' })
    expect(take).toHaveBeenCalledWith('send_to_session', SEND_RATE_MAX, SEND_RATE_MS)
    expect(page).toHaveBeenCalledWith({ tool: 'send_to_session', session: 'pty-2', text: 'run tests' })
    expect(res.isError).toBeUndefined()
    expect(res.content[0]!.text).toContain('the user presses Enter')
  })

  it('refuses bad input before the rate window and the page', async () => {
    const page = vi.fn(fail)
    const take = vi.fn(() => true)
    const bad = [{}, { session: 'a' }, { session: '', text: 'x' }, { session: 'a', text: '' }, { session: 'a', text: '\u0007\u0001' }, { session: 'a', text: 'x'.repeat(8001) }, { session: 'a', text: 5 }, { session: 7, text: 'x' }]
    for (const args of bad) {
      const res = await call(hostWith({ page, take }), 'send_to_session', args)
      expect(res.isError, JSON.stringify(args)).toBe(true)
    }
    expect(page).not.toHaveBeenCalled()
    expect(take).not.toHaveBeenCalled()
  })

  it('says rate when the window is full, target / busy when the page refuses', async () => {
    const page = vi.fn(sent)
    expect(await call(hostWith({ page, take: () => false }), 'send_to_session', { session: 'a', text: 'x' })).toMatchObject({ isError: true, structuredContent: { error: 'rate' } })
    expect(page).not.toHaveBeenCalled()
    for (const error of ['target', 'busy'] as const) {
      const res = await call(hostWith({ page: () => Promise.resolve({ ok: false, error }) }), 'send_to_session', { session: 'a', text: 'x' })
      expect(res).toMatchObject({ isError: true, structuredContent: { error } })
    }
  })
})

describe('close_session', () => {
  it('relays the id and tells whether the owner was asked', async () => {
    const page = vi.fn((): Promise<McpPageResult> => Promise.resolve({ ok: true, tool: 'close_session', name: 'ds', confirm: false }))
    const res = await call(hostWith({ page }), 'close_session', { session: 'pty-2' })
    expect(page).toHaveBeenCalledWith({ tool: 'close_session', session: 'pty-2' })
    expect(res.content[0]!.text).toContain('the owner can undo it')
    const asked = await call(hostWith({ page: () => Promise.resolve({ ok: true, tool: 'close_session', name: 'ds', confirm: true }) }), 'close_session', { session: 'pty-2' })
    expect(asked.content[0]!.text).toContain('confirm')
  })

  it('refuses a missing id and passes the page refusal on', async () => {
    const page = vi.fn(fail)
    expect(await call(hostWith({ page }), 'close_session', {})).toMatchObject({ isError: true, structuredContent: { error: 'session' } })
    expect(page).not.toHaveBeenCalled()
    expect(await call(hostWith({ page: () => Promise.resolve({ ok: false, error: 'target' }) }), 'close_session', { session: 'x' })).toMatchObject({ isError: true, structuredContent: { error: 'target' } })
  })
})

describe('wait_session', () => {
  afterEach(() => vi.useRealTimers())
  const row = (sessionId: string, status: string, self = false) => ({ sessionId, name: sessionId, agent: 'codex' as const, status, self, openedByYou: true })
  const listing = (...rows: ReturnType<typeof row>[]): McpPageResult => ({ ok: true, tool: 'list_sessions', sessions: rows })
  const pageOf = (answers: McpPageResult[]) => {
    let i = 0
    return vi.fn((): Promise<McpPageResult> => Promise.resolve(answers[Math.min(i++, answers.length - 1)]!))
  }

  it('returns at once when the pane already is idle, with the per-pane rate window', async () => {
    const take = vi.fn(() => true)
    const page = pageOf([listing(row('pty-1', 'idle', true), row('pty-2', 'idle'))])
    const res = await call(hostWith({ page, take }), 'wait_session', { session: 'pty-2' })
    expect(take).toHaveBeenCalledWith('wait_session', WAIT_RATE_MAX, 60_000)
    expect(page).toHaveBeenCalledWith({ tool: 'list_sessions' })
    expect(res.isError).toBeUndefined()
    expect(res.structuredContent).toMatchObject({ ok: true, tool: 'wait_session', session: 'pty-2', status: 'idle', reached: true })
    expect(typeof res.structuredContent.waitedMs).toBe('number')
  })

  it('polls every 2 s until idle', async () => {
    vi.useFakeTimers()
    const page = pageOf([listing(row('pty-2', 'working')), listing(row('pty-2', 'working')), listing(row('pty-2', 'idle'))])
    const done = call(hostWith({ page }), 'wait_session', { session: 'pty-2', timeoutMs: 60_000 })
    await vi.advanceTimersByTimeAsync(WAIT_POLL_MS * 2)
    const res = await done
    expect(page).toHaveBeenCalledTimes(3)
    expect(res.structuredContent).toMatchObject({ status: 'idle', reached: true, waitedMs: WAIT_POLL_MS * 2 })
  })

  it('a pane that leaves the list has exited: reached for until exited, not for idle', async () => {
    vi.useFakeTimers()
    const exited = call(hostWith({ page: pageOf([listing(row('pty-2', 'idle')), listing()]) }), 'wait_session', { session: 'pty-2', until: 'exited' })
    await vi.advanceTimersByTimeAsync(WAIT_POLL_MS)
    expect((await exited).structuredContent).toMatchObject({ status: 'exited', reached: true })
    const idle = call(hostWith({ page: pageOf([listing(row('pty-2', 'working')), listing()]) }), 'wait_session', { session: 'pty-2' })
    await vi.advanceTimersByTimeAsync(WAIT_POLL_MS)
    expect((await idle).structuredContent).toMatchObject({ status: 'exited', reached: false })
  })

  it('gives up at the timeout with the last status, and rides out a page hiccup', async () => {
    vi.useFakeTimers()
    const page = pageOf([listing(row('pty-2', 'working')), { ok: false, error: 'unavailable' }, listing(row('pty-2', 'waiting'))])
    const done = call(hostWith({ page }), 'wait_session', { session: 'pty-2', timeoutMs: 5000 })
    await vi.advanceTimersByTimeAsync(5000)
    const res = await done
    expect(res.isError).toBeUndefined()
    expect(res.structuredContent).toMatchObject({ status: 'waiting', reached: false, waitedMs: 5000 })
    expect(page).toHaveBeenCalledTimes(4)
  })

  it('refuses bad input before the rate window and the page', async () => {
    const page = vi.fn(fail)
    const take = vi.fn(() => true)
    const cases: [unknown, string][] = [
      [{}, 'session'],
      [{ session: '' }, 'session'],
      [{ session: 'a', until: 'working' }, 'until'],
      [{ session: 'a', timeoutMs: 999 }, 'timeoutMs'],
      [{ session: 'a', timeoutMs: 600_001 }, 'timeoutMs'],
      [{ session: 'a', timeoutMs: 1500.5 }, 'timeoutMs'],
    ]
    for (const [args, error] of cases) expect(await call(hostWith({ page, take }), 'wait_session', args)).toMatchObject({ isError: true, structuredContent: { ok: false, error } })
    expect(page).not.toHaveBeenCalled()
    expect(take).not.toHaveBeenCalled()
    expect(await call(hostWith({ page, take: () => false }), 'wait_session', { session: 'a' })).toMatchObject({ isError: true, structuredContent: { error: 'rate' } })
  })

  it('target for itself or an unknown pane; caller / page errors pass on', async () => {
    const self = await call(hostWith({ page: pageOf([listing(row('pty-1', 'idle', true))]) }), 'wait_session', { session: 'pty-1' })
    expect(self).toMatchObject({ isError: true, structuredContent: { error: 'target' } })
    const unknown = await call(hostWith({ page: pageOf([listing(row('pty-1', 'idle', true))]) }), 'wait_session', { session: 'pty-9' })
    expect(unknown).toMatchObject({ isError: true, structuredContent: { error: 'target' } })
    for (const error of ['caller', 'unavailable'] as const)
      expect(await call(hostWith({ page: pageOf([{ ok: false, error }]) }), 'wait_session', { session: 'pty-2' })).toMatchObject({ isError: true, structuredContent: { ok: false, error } })
  })

  it('stops waiting once the calling pane is gone', async () => {
    vi.useFakeTimers()
    const page = pageOf([listing(row('pty-2', 'working')), { ok: false, error: 'caller' }])
    const done = call(hostWith({ page }), 'wait_session', { session: 'pty-2', timeoutMs: 600_000 })
    await vi.advanceTimersByTimeAsync(WAIT_POLL_MS)
    expect(await done).toMatchObject({ isError: true, structuredContent: { error: 'caller' } })
    expect(page).toHaveBeenCalledTimes(2)
  })
})
