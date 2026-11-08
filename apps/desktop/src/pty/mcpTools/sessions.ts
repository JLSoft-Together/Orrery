import { cleanPanePrompt, PANE_OPEN_PROMPT_MAX, type PtyPreset } from '../../../../../packages/shared/src/pty'
import type { SessionRef } from '../../../../../packages/shared/src/mcpCall'
import { ask, PAGE_ERRORS } from './runCommon'
import { toolResult, type McpTool, type ToolDef } from './types'

export const SEND_RATE_MAX = 20
export const SEND_RATE_MS = 60_000
export const WAIT_RATE_MAX = 30
export const WAIT_DEFAULT_MS = 120_000
export const WAIT_MIN_MS = 1000
export const WAIT_MAX_MS = 600_000
export const WAIT_POLL_MS = 2000
const SESSION_ID_MAX = 200

const AGENT: Record<PtyPreset, string> = { claude: 'Claude Code', codex: 'Codex', deepseek: 'DeepSeek', shell: 'shell' }

// Names come from the owner or the folder; quoted so they read as data, not instructions.
const q = (v: string): string => JSON.stringify(v)

const sessionProp = { type: 'string', maxLength: SESSION_ID_MAX, description: 'Session id from list_sessions.' }

export const LIST_SESSIONS_TOOL = {
  name: 'list_sessions',
  description:
    "List the other Orrery sessions (Claude Code, Codex, shell panes) in this session's project: name, agent, status, session id, whether you opened it, since when it is idle (idleSince) and whether a Claude session is at its usage limit (limit, with resetsAt / window when known). Read-only: no terminal content. `id` is the Orrery pane id (open_session returns it as paneId): use it with send_to_session, close_session and wait_session. `agent session` is the agent's own conversation uuid: never pass it to those tools.",
  inputSchema: { type: 'object', properties: {}, additionalProperties: false },
  annotations: { title: 'List project sessions', readOnlyHint: true, destructiveHint: false, openWorldHint: false, idempotentHint: true },
} satisfies ToolDef

export const SEND_TO_SESSION_TOOL = {
  name: 'send_to_session',
  description:
    'Type a message into the input box of another agent session in this session\'s project (from list_sessions). It is NOT submitted: the user reviews it and presses Enter. Not for yourself, other projects or shell panes. At most 20 a minute.',
  inputSchema: {
    type: 'object',
    properties: { session: sessionProp, text: { type: 'string', minLength: 1, maxLength: PANE_OPEN_PROMPT_MAX, description: 'Text for the input box.' } },
    required: ['session', 'text'],
    additionalProperties: false,
  },
  annotations: { title: 'Prefill another session', readOnlyHint: false, destructiveHint: false, openWorldHint: false },
} satisfies ToolDef

export const CLOSE_SESSION_TOOL = {
  name: 'close_session',
  description:
    'Close a session pane that you opened with open_session. Closes like the owner does: a pane that still runs something asks the owner first, and the owner can undo the close. Sessions you did not open cannot be closed.',
  inputSchema: { type: 'object', properties: { session: sessionProp }, required: ['session'], additionalProperties: false },
  annotations: { title: 'Close a session you opened', readOnlyHint: false, destructiveHint: true, openWorldHint: false },
} satisfies ToolDef

export const WAIT_SESSION_TOOL = {
  name: 'wait_session',
  description:
    'Wait until another session of this project (from list_sessions) is idle or has exited, or until timeoutMs. Returns its status and whether it got there; never terminal content. Returns at once when it already is there. Not for yourself. At most 30 a minute.',
  inputSchema: {
    type: 'object',
    properties: {
      session: sessionProp,
      until: { type: 'string', enum: ['idle', 'exited'], default: 'idle', description: 'idle: the agent finished its turn (an exited pane also ends the wait, with reached false). exited: the pane was closed or its program ended.' },
      timeoutMs: { type: 'integer', minimum: WAIT_MIN_MS, maximum: WAIT_MAX_MS, default: WAIT_DEFAULT_MS, description: 'How long to wait at most, in milliseconds.' },
    },
    required: ['session'],
    additionalProperties: false,
  },
  annotations: { title: 'Wait for a session', readOnlyHint: true, destructiveHint: false, openWorldHint: false },
} satisfies ToolDef

const ERRORS = {
  ...PAGE_ERRORS,
  rate: 'Too many calls of this tool from this session in the last minute; wait a little.',
  session: 'session must be a session id from list_sessions.',
  text: `text must be 1 to ${PANE_OPEN_PROMPT_MAX} characters of plain text.`,
  until: "until must be 'idle' or 'exited'.",
  timeoutMs: `timeoutMs must be a whole number from ${WAIT_MIN_MS} to ${WAIT_MAX_MS}.`,
}

const fail = (error: keyof typeof ERRORS) => toolResult(ERRORS[error], { ok: false, error }, true)
const idOf = (v: unknown): string | null => (typeof v === 'string' && v.length > 0 && v.length <= SESSION_ID_MAX && !v.includes('\0') ? v : null)

function limitText(l: NonNullable<SessionRef['limit']>): string {
  const parts = [l.window ? `window ${l.window}` : '', l.resetsAt ? `resets ${l.resetsAt}` : ''].filter(Boolean).join(', ')
  return `at usage limit${parts ? ` (${parts})` : ''}`
}

function line(s: SessionRef): string {
  const flags = [s.self ? 'this is you' : '', s.openedByYou ? 'opened by you' : '', s.idleSince ? `idle since ${s.idleSince}` : '', s.limit ? limitText(s.limit) : '']
    .filter(Boolean)
    .join(', ')
  return `- ${q(s.name)} (${AGENT[s.agent]}, ${s.status}${flags ? `, ${flags}` : ''}) id ${s.sessionId}${s.agentSessionId ? `, agent session ${s.agentSessionId}` : ''}`
}

export const listSessions: McpTool = {
  def: LIST_SESSIONS_TOOL,
  async call(_args, host) {
    const res = await ask(host, { tool: 'list_sessions' })
    if (!res.ok) return fail(res.error)
    if (res.tool !== 'list_sessions') return fail('failed')
    const out = { ok: true, sessions: res.sessions }
    return toolResult(res.sessions.length ? `Sessions in this project:\n${res.sessions.map(line).join('\n')}` : 'No session of this project is open in the workspace.', out)
  },
}

export const sendToSession: McpTool = {
  def: SEND_TO_SESSION_TOOL,
  async call(args, host) {
    const session = idOf(args.session)
    if (!session) return fail('session')
    if (typeof args.text !== 'string' || args.text.length > PANE_OPEN_PROMPT_MAX * 2) return fail('text')
    const text = cleanPanePrompt(args.text)
    if (!text || args.text.trim().length > PANE_OPEN_PROMPT_MAX) return fail('text')
    if (!host.take('send_to_session', SEND_RATE_MAX, SEND_RATE_MS)) return fail('rate')
    const res = await ask(host, { tool: 'send_to_session', session, text })
    if (!res.ok) return fail(res.error)
    if (res.tool !== 'send_to_session') return fail('failed')
    return toolResult(`Typed your message into ${q(res.name)}. It is waiting in its input box; the user presses Enter to send it.`, { ok: true, session, name: res.name })
  },
}

export const closeSession: McpTool = {
  def: CLOSE_SESSION_TOOL,
  async call(args, host) {
    const session = idOf(args.session)
    if (!session) return fail('session')
    const res = await ask(host, { tool: 'close_session', session })
    if (!res.ok) return fail(res.error)
    if (res.tool !== 'close_session') return fail('failed')
    const text = res.confirm ? `${q(res.name)} still runs something, so Orrery asked the owner to confirm the close.` : `Closed ${q(res.name)}; the owner can undo it.`
    return toolResult(text, { ok: true, session, name: res.name, confirm: res.confirm })
  },
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

function timeoutOf(v: unknown): number | null {
  if (v === undefined) return WAIT_DEFAULT_MS
  return typeof v === 'number' && Number.isInteger(v) && v >= WAIT_MIN_MS && v <= WAIT_MAX_MS ? v : null
}

/**
 * Polls the page's list (the host has no monitor status of its own): a pane missing from it has exited. A page
 * hiccup mid-wait is retried; `caller` ends the wait, the calling pane is gone and nobody reads the answer.
 */
export const waitSession: McpTool = {
  def: WAIT_SESSION_TOOL,
  async call(args, host) {
    const session = idOf(args.session)
    if (!session) return fail('session')
    const until = args.until === undefined ? 'idle' : args.until
    if (until !== 'idle' && until !== 'exited') return fail('until')
    const timeoutMs = timeoutOf(args.timeoutMs)
    if (timeoutMs === null) return fail('timeoutMs')
    if (!host.take('wait_session', WAIT_RATE_MAX, SEND_RATE_MS)) return fail('rate')
    const started = Date.now()
    let first = true
    let status = 'unknown'
    for (;;) {
      const res = await ask(host, { tool: 'list_sessions' })
      if (!res.ok || res.tool !== 'list_sessions') {
        const error = res.ok ? 'failed' : res.error
        if (first || error === 'caller') return fail(error)
      } else {
        const row = res.sessions.find((r) => r.sessionId === session)
        if (row?.self || (first && !row)) return fail('target')
        status = row ? row.status : 'exited'
        if (status === 'exited' || (until === 'idle' && status === 'idle')) break
      }
      first = false
      const left = timeoutMs - (Date.now() - started)
      if (left <= 0) break
      await sleep(Math.min(WAIT_POLL_MS, left))
    }
    const reached = status === until
    const waitedMs = Date.now() - started
    const text = reached ? `The session is ${until} (waited ${Math.round(waitedMs / 1000)} s).` : `The session is ${status}, not ${until}, after ${Math.round(waitedMs / 1000)} s.`
    return toolResult(text, { ok: true, tool: 'wait_session', session, status, reached, waitedMs })
  },
}
