import type { AgentNoteLevel } from '../../../../packages/shared/src/mcpCall'
import type { HostHandlers } from '../desk/host'
import type { DeskAsk, OpenArgs, PageAsk, ToolHost } from './mcpTools/types'

export const MCP_HANDLE_CHANNEL = 'desk:mcp-handle'

export interface McpReply {
  status: number
  json?: unknown
}

export type McpHandle = (body: unknown, host: ToolHost) => Promise<McpReply | null>

type Invoke = (channel: string, args: unknown[], hosts: HostHandlers) => Promise<unknown>

const obj = (v: unknown): Record<string, unknown> => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {})

export function mcpToolHosts(host: ToolHost): HostHandlers {
  return {
    'mcp.take': (raw) => {
      const p = obj(raw)
      return typeof p.key === 'string' && typeof p.max === 'number' && typeof p.windowMs === 'number' ? host.take(p.key, p.max, p.windowMs) : false
    },
    'mcp.open': (raw) => host.openSession(obj(obj(raw).args) as OpenArgs),
    'mcp.notify': (raw) => {
      const p = obj(raw)
      return host.notify(String(p.text ?? ''), p.level as AgentNoteLevel)
    },
    'mcp.roots': () => host.checkoutRoots(),
    'mcp.page': (raw) => host.page(obj(obj(raw).call) as PageAsk),
    'mcp.desk': (raw) => (host.desk ? host.desk(obj(obj(raw).call) as DeskAsk) : { ok: false, error: 'unavailable' }),
  }
}

export function parseMcpReply(raw: unknown): McpReply | null {
  const r = obj(raw)
  if (typeof r.status !== 'number' || !Number.isInteger(r.status)) return null
  return 'json' in r ? { status: r.status, json: r.json } : { status: r.status }
}

export function rustMcpHandle(invoke: Invoke, version: string, deepseek: boolean): McpHandle {
  return async (body, host) => {
    const req = { body, version, deepseek, root: host.root, ...(host.preset ? { preset: host.preset } : {}), desk: !!host.desk }
    const raw = await invoke(MCP_HANDLE_CHANNEL, [req], mcpToolHosts(host)).catch(() => null)
    return parseMcpReply(raw)
  }
}
