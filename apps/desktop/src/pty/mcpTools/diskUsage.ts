import { DISK_USAGE_DEFAULT, DISK_USAGE_MAX } from '../../../../../packages/shared/src/mcpDesk'
import { toolResult, type McpTool, type ToolDef } from './types'
import { askDesk, DISK_READ_RATE_MAX, DISK_READ_RATE_MS, diskFail, isAbsPath, itemLine, optInt, sizeText } from './diskCommon'

export const DISK_USAGE_TOOL = {
  name: 'disk_usage',
  description:
    "Size of a folder in Orrery's disk index and its biggest children (folders and files), biggest first. Read-only and instant. The path must be inside the scanned folders; it never scans.",
  inputSchema: {
    type: 'object',
    properties: {
      path: { type: 'string', maxLength: 4096, description: 'Absolute folder (or file) path inside the index.' },
      limit: { type: 'integer', minimum: 1, maximum: DISK_USAGE_MAX, description: `How many children to list. Default ${DISK_USAGE_DEFAULT}.` },
    },
    required: ['path'],
    additionalProperties: false,
  },
  annotations: { title: 'Disk usage of a folder', readOnlyHint: true, destructiveHint: false, openWorldHint: false, idempotentHint: true },
} satisfies ToolDef

export const diskUsage: McpTool = {
  def: DISK_USAGE_TOOL,
  async call(args, host) {
    if (!isAbsPath(args.path)) return diskFail('invalid', 'path')
    const limit = optInt(args.limit, 1, DISK_USAGE_MAX)
    if (limit === null) return diskFail('invalid', 'limit')
    if (!host.take('disk-read', DISK_READ_RATE_MAX, DISK_READ_RATE_MS)) return diskFail('rate')
    const res = await askDesk(host, { tool: 'disk_usage', path: args.path, limit: limit ?? DISK_USAGE_DEFAULT })
    if (!res.ok) return diskFail(res.error)
    if (res.tool !== 'disk_usage') return diskFail('failed')
    const head = `${res.path}: ${sizeText(res.size)} in total, ${res.total} ${res.total === 1 ? 'entry' : 'entries'} directly inside${res.items.length ? `, showing ${res.items.length}, biggest first:` : '.'}`
    return toolResult([head, ...res.items.map(itemLine)].join('\n'), res)
  },
}
