import { compileLogMatch, isMcpLogCursor, MCP_LOG_LINES_DEFAULT, MCP_LOG_LINES_MAX, MCP_LOG_MATCH_MAX } from '../../../../../packages/shared/src/mcpRun'
import { ask, maskRun, maskSession, PAGE_ERRORS, sessionLine } from './runCommon'
import { toolResult, type McpTool, type ToolDef } from './types'

export const RUN_LOG_RATE_MAX = 30
export const RUN_LOG_RATE_MS = 60_000

export const RUN_LOG_TOOL = {
  name: 'run_log',
  description:
    "The last lines of the Run log of this session's project (the run the Run pane shows: build output, app output, logcat), with secrets masked, plus the build errors Orrery parsed (file:line:col). Use errorsOnly to get just error lines, match (a JavaScript regex per line, optional ignoreCase) to get just matching lines. Every result has a cursor: pass it back as since to get only the lines logged after it, oldest first, up to lines per call (call again while more come); reset=true means the cursor belonged to another run so the log restarts, truncated=true means older lines were dropped from the buffer before you read them.",
  inputSchema: {
    type: 'object',
    properties: {
      lines: { type: 'integer', minimum: 1, maximum: MCP_LOG_LINES_MAX, default: MCP_LOG_LINES_DEFAULT, description: `How many of the last lines, at most ${MCP_LOG_LINES_MAX}.` },
      errorsOnly: { type: 'boolean', default: false, description: 'Only error / fatal lines and lines that read like errors.' },
      match: { type: 'string', minLength: 1, maxLength: MCP_LOG_MATCH_MAX, description: 'JavaScript regular expression tested on each line; only matching lines are returned (also with errorsOnly: both must hold).' },
      ignoreCase: { type: 'boolean', default: false, description: 'Case-insensitive match.' },
      since: { type: 'string', description: 'The cursor of a previous run_log result: only lines after it, oldest first.' },
    },
    additionalProperties: false,
  },
  annotations: { title: 'Orrery Run log', readOnlyHint: true, destructiveHint: false, openWorldHint: false, idempotentHint: true },
} satisfies ToolDef

const ERRORS = {
  rate: `At most ${RUN_LOG_RATE_MAX} run_log calls a minute per session; wait a little.`,
  lines: `lines must be a whole number from 1 to ${MCP_LOG_LINES_MAX}.`,
  errorsOnly: 'errorsOnly must be true or false.',
  match: `match must be a valid JavaScript regular expression of 1 to ${MCP_LOG_MATCH_MAX} characters.`,
  ignoreCase: 'ignoreCase must be true or false.',
  since: 'since must be the cursor string of a previous run_log result.',
} as const

export const runLog: McpTool = {
  def: RUN_LOG_TOOL,
  async call(args, host) {
    const fail = (error: keyof typeof ERRORS) => toolResult(ERRORS[error], { ok: false, error }, true)
    if (!host.take('runLog', RUN_LOG_RATE_MAX, RUN_LOG_RATE_MS)) return fail('rate')
    const n = args.lines ?? MCP_LOG_LINES_DEFAULT
    if (typeof n !== 'number' || !Number.isInteger(n) || n < 1 || n > MCP_LOG_LINES_MAX) return fail('lines')
    if (args.errorsOnly !== undefined && args.errorsOnly !== null && typeof args.errorsOnly !== 'boolean') return fail('errorsOnly')
    const errorsOnly = args.errorsOnly === true
    const none = (v: unknown): boolean => v === undefined || v === null
    if (!none(args.ignoreCase) && typeof args.ignoreCase !== 'boolean') return fail('ignoreCase')
    const ignoreCase = args.ignoreCase === true
    if (!none(args.match) && (typeof args.match !== 'string' || !compileLogMatch(args.match, ignoreCase))) return fail('match')
    if (!none(args.since) && !isMcpLogCursor(args.since)) return fail('since')
    const match = none(args.match) ? undefined : (args.match as string)
    const since = none(args.since) ? undefined : (args.since as string)
    const res = await ask(host, { tool: 'run_log', lines: n, errorsOnly, ...(match !== undefined ? { match, ...(ignoreCase ? { ignoreCase } : {}) } : {}), ...(since !== undefined ? { since } : {}) })
    if (!res.ok) return toolResult(PAGE_ERRORS[res.error], res, true)
    if (res.tool !== 'run_log') return toolResult(PAGE_ERRORS.failed, { ok: false, error: 'failed' }, true)
    const session = maskSession(res.session)
    if (!session) return toolResult('Nothing has run in this project yet, so there is no Run log.', { ok: true, session: null, lines: [], errors: [] })
    const kept = since !== undefined ? res.lines.slice(0, n) : res.lines.slice(-n)
    const lines = kept.map((l) => ({ ...l, text: maskRun(l.text) }))
    const errors = res.errors.map((e) => ({ ...e, file: maskRun(e.file), message: maskRun(e.message) }))
    const notes = [
      res.partial ? '(Output from before the Orrery window reloaded is not available.)' : '',
      res.reset ? '(The cursor belonged to another run or a cleared log: this is the log from its start.)' : '',
      res.truncated ? '(Lines after the cursor were dropped from the log buffer before this call; some output is missing.)' : '',
    ].filter(Boolean)
    const head = `Run: ${sessionLine(session)}${notes.map((note) => `\n${note}`).join('')}`
    const filtered = errorsOnly || match !== undefined
    const body = lines.length ? lines.map((l) => `[${l.stream}${l.level ? ` ${l.level}` : ''}] ${l.text}`).join('\n') : since !== undefined ? '(no new lines)' : filtered ? '(no matching lines)' : '(no output yet)'
    const errs = errors.length ? `\n\nBuild errors:\n${errors.map((e) => `- ${e.file}:${e.line}:${e.col} ${e.severity}: ${e.message}`).join('\n')}` : ''
    const cur = res.cursor ? `\n\ncursor: ${res.cursor}` : ''
    return toolResult(`${head}\n\n${body}${errs}${cur}`, {
      ok: true,
      session,
      lines,
      errors,
      partial: res.partial,
      ...(res.cursor ? { cursor: res.cursor } : {}),
      ...(res.reset ? { reset: true } : {}),
      ...(res.truncated ? { truncated: true } : {}),
    })
  },
}
