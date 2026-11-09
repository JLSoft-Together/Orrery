import type { IncomingMessage, ServerResponse } from 'node:http'
import { MCP_TOOLS } from './mcpTools'
import type { Json, McpTool, ToolHost } from './mcpTools/types'

export { OPEN_SESSION_TOOL, openText, toOpenResult } from './mcpTools/openSession'
export type { OpenArgs, OpenOutcome, ToolHost } from './mcpTools/types'

export const MCP_PROTOCOL_VERSION = '2025-06-18'

export const MCP_INSTRUCTIONS =
  'This server connects you to the Orrery workspace you run in. Use open_session to open new Claude Code, Codex, DeepSeek or shell sessions as panes instead of launching terminal windows; notify_owner to leave the user a short note; open_in_workspace to show the user a file or its diff; who_is_editing before editing a file other sessions may be changing; run_app, run_status and run_log to run the project app and read its log; device_screenshot to see a connected device; preview_layout to render an Android XML layout; list_servers and free_port for ports your session left listening; list_sessions, send_to_session, close_session and wait_session to see, prefill, close and wait for the other sessions of your project; hand_back, in a DeepSeek session another agent opened, to return the finished work to that agent.'

const MCP_INSTRUCTIONS_NO_DEEPSEEK =
  'This server connects you to the Orrery workspace you run in. Use open_session to open new Claude Code, Codex or shell sessions as panes instead of launching terminal windows; notify_owner to leave the user a short note; open_in_workspace to show the user a file or its diff; who_is_editing before editing a file other sessions may be changing; run_app, run_status and run_log to run the project app and read its log; device_screenshot to see a connected device; preview_layout to render an Android XML layout; list_servers and free_port for ports your session left listening; list_sessions, send_to_session, close_session and wait_session to see, prefill, close and wait for the other sessions of your project.'

const isObj = (v: unknown): v is Json => !!v && typeof v === 'object' && !Array.isArray(v)

const rpcError = (id: unknown, code: number, message: string): Json => ({ jsonrpc: '2.0', id, error: { code, message } })

const invalid = (): Json => rpcError(null, -32600, 'Invalid Request')

/** A batch runs its calls one after the other on the host's event loop: cap it. */
export const BATCH_MAX = 16

async function answer(msg: unknown, host: ToolHost, version: string, tools: readonly McpTool[]): Promise<Json | undefined> {
  if (!isObj(msg) || msg.jsonrpc !== '2.0') return invalid()
  const { id, method } = msg
  if (id !== undefined && id !== null && typeof id !== 'string' && typeof id !== 'number') return invalid()
  if (typeof method !== 'string') return id !== undefined && ('result' in msg || 'error' in msg) ? undefined : invalid()
  if (id === undefined) return undefined
  const ok = (result: unknown): Json => ({ jsonrpc: '2.0', id, result })
  const params = isObj(msg.params) ? msg.params : {}
  switch (method) {
    case 'initialize': {
      const asked = params.protocolVersion
      return ok({
        protocolVersion: typeof asked === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(asked) ? asked : MCP_PROTOCOL_VERSION,
        capabilities: { tools: { listChanged: false } },
        serverInfo: { name: 'orrery', version },
        instructions: tools.some((t) => t.def.name === 'hand_back') ? MCP_INSTRUCTIONS : MCP_INSTRUCTIONS_NO_DEEPSEEK,
      })
    }
    case 'ping':
      return ok({})
    case 'tools/list':
      return ok({ tools: tools.map((t) => t.def) })
    case 'tools/call': {
      const tool = tools.find((t) => t.def.name === params.name)
      if (!tool) return rpcError(id, -32602, 'Unknown tool')
      const args = params.arguments ?? {}
      if (!isObj(args)) return rpcError(id, -32602, 'Invalid arguments')
      return ok(await tool.call(args, host))
    }
    default:
      return rpcError(id, -32601, 'Method not found')
  }
}

export async function handleMcp(body: unknown, host: ToolHost, version: string, tools: readonly McpTool[] = MCP_TOOLS): Promise<{ status: number; json?: unknown }> {
  if (!Array.isArray(body)) {
    const res = await answer(body, host, version, tools)
    return res ? { status: 200, json: res } : { status: 202 }
  }
  if (!body.length) return { status: 200, json: invalid() }
  if (body.length > BATCH_MAX) return { status: 200, json: rpcError(null, -32600, `Invalid Request: at most ${BATCH_MAX} messages per batch`) }
  const out: Json[] = []
  for (const msg of body) {
    const res = await answer(msg, host, version, tools)
    if (res) out.push(res)
  }
  return out.length ? { status: 200, json: out } : { status: 202 }
}

export function mcpGate(
  req: { method?: string; url?: string; host?: string; origin?: string; authorization?: string },
  port: number,
): { status: number; allow?: string } | { token: string } {
  if ((req.url ?? '').split('?')[0] !== '/mcp') return { status: 404 }
  if (req.host !== `127.0.0.1:${port}`) return { status: 403 }
  if (req.origin) return { status: 403 }
  if (req.method !== 'POST') return { status: 405, allow: 'POST' }
  const auth = /^(\S+) ([0-9a-f]{64})$/.exec(req.authorization ?? '')
  if (!auth || auth[1]!.toLowerCase() !== 'bearer') return { status: 401 }
  return { token: auth[2]! }
}

export const BODY_MAX = 64 * 1024

export interface McpHttpOptions {
  port: () => number
  caller: (token: string) => ToolHost | null
  version: string
  tools: readonly McpTool[]
  log?: (line: string) => void
}

function readBody(req: IncomingMessage): Promise<string | null> {
  return new Promise((resolve) => {
    const chunks: Buffer[] = []
    let size = 0
    req.on('data', (chunk: Buffer) => {
      size += chunk.length
      if (size > BODY_MAX) {
        req.removeAllListeners('data')
        req.pause()
        resolve(null)
      } else chunks.push(chunk)
    })
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')))
    req.on('error', () => resolve(null))
  })
}

const replyJson = (res: ServerResponse, status: number, json: unknown): void =>
  void res.writeHead(status, { 'Content-Type': 'application/json' }).end(JSON.stringify(json))

async function onMcpRequest(req: IncomingMessage, res: ServerResponse, opts: McpHttpOptions): Promise<void> {
  const gate = mcpGate({ method: req.method, url: req.url, host: req.headers.host, origin: req.headers.origin, authorization: req.headers.authorization }, opts.port())
  if ('status' in gate) return void res.writeHead(gate.status, gate.allow ? { Allow: gate.allow } : {}).end()
  const host = opts.caller(gate.token)
  if (!host) return void res.writeHead(401).end()
  if (Number(req.headers['content-length'] ?? 0) > BODY_MAX) return void res.writeHead(413, { Connection: 'close' }).end()
  const raw = await readBody(req)
  if (raw === null) return void res.writeHead(413, { Connection: 'close' }).end(() => req.destroy())
  let body: unknown
  try {
    body = JSON.parse(raw)
  } catch {
    return replyJson(res, 400, { jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } })
  }
  const out = await handleMcp(body, host, opts.version, opts.tools)
  if (out.json === undefined) res.writeHead(out.status).end()
  else replyJson(res, out.status, out.json)
}

export function mcpHttpHandler(opts: McpHttpOptions): (req: IncomingMessage, res: ServerResponse) => void {
  return (req, res) => {
    onMcpRequest(req, res, opts).catch((err: unknown) => {
      opts.log?.(`mcp request failed: ${String(err)}`)
      if (!res.headersSent) res.writeHead(500)
      res.end()
    })
  }
}
