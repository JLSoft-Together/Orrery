import { DISK_SEARCH_DEFAULT, DISK_SEARCH_MAX } from '../../../../../packages/shared/src/mcpDesk'
import { toolResult, type McpTool, type ToolDef } from './types'
import { askDesk, DISK_KIND_VALUES, DISK_READ_RATE_MAX, DISK_READ_RATE_MS, diskFail, isAbsPath, itemLine, optInt } from './diskCommon'

export const DISK_SEARCH_TOOL = {
  name: 'disk_search',
  description:
    "Search Orrery's disk index (the one behind the Disk view) by name, kind, size, age or folder: path, size, modified date and kind of the biggest matches, plus the total match count. Read-only and instant. It never scans: when the disk has not been scanned yet it says so and the user has to run the scan in Settings > Data > Disk.",
  inputSchema: {
    type: 'object',
    properties: {
      query: { type: 'string', maxLength: 1024, description: 'Words that must appear in the file or folder name or path (space separated).' },
      kind: { type: 'string', enum: [...DISK_KIND_VALUES], description: 'Only files of this kind.' },
      minSize: { type: 'integer', minimum: 0, description: 'Only entries of at least this many bytes.' },
      maxSize: { type: 'integer', minimum: 0, description: 'Only entries of at most this many bytes.' },
      olderThanDays: { type: 'integer', minimum: 0, maximum: 36500, description: 'Only entries last modified more than this many days ago.' },
      under: { type: 'string', maxLength: 4096, description: 'Absolute folder path inside the index to search in.' },
      limit: { type: 'integer', minimum: 1, maximum: DISK_SEARCH_MAX, description: `How many matches to return, biggest first. Default ${DISK_SEARCH_DEFAULT}.` },
    },
    additionalProperties: false,
  },
  annotations: { title: 'Search the disk index', readOnlyHint: true, destructiveHint: false, openWorldHint: false, idempotentHint: true },
} satisfies ToolDef

export const diskSearch: McpTool = {
  def: DISK_SEARCH_TOOL,
  async call(args, host) {
    const query = args.query === undefined || args.query === null ? undefined : typeof args.query === 'string' && args.query.length <= 1024 && !args.query.includes('\0') ? args.query : null
    if (query === null) return diskFail('invalid', 'query')
    const kind = args.kind === undefined || args.kind === null ? undefined : typeof args.kind === 'string' && (DISK_KIND_VALUES as readonly string[]).includes(args.kind) ? args.kind : null
    if (kind === null) return diskFail('invalid', 'kind')
    const minSize = optInt(args.minSize, 0, Number.MAX_SAFE_INTEGER)
    const maxSize = optInt(args.maxSize, 0, Number.MAX_SAFE_INTEGER)
    if (minSize === null || maxSize === null) return diskFail('invalid', 'size')
    const olderThanDays = optInt(args.olderThanDays, 0, 36_500)
    if (olderThanDays === null) return diskFail('invalid', 'days')
    const under = args.under === undefined || args.under === null ? undefined : isAbsPath(args.under) ? args.under : null
    if (under === null) return diskFail('invalid', 'under')
    const limit = optInt(args.limit, 1, DISK_SEARCH_MAX)
    if (limit === null) return diskFail('invalid', 'limit')
    if (!host.take('disk-read', DISK_READ_RATE_MAX, DISK_READ_RATE_MS)) return diskFail('rate')
    const res = await askDesk(host, {
      tool: 'disk_search',
      ...(query !== undefined ? { query } : {}),
      ...(kind !== undefined ? { kind } : {}),
      ...(minSize !== undefined ? { minSize } : {}),
      ...(maxSize !== undefined ? { maxSize } : {}),
      ...(olderThanDays !== undefined ? { olderThanDays } : {}),
      ...(under !== undefined ? { under } : {}),
      limit: limit ?? DISK_SEARCH_DEFAULT,
    })
    if (!res.ok) return diskFail(res.error)
    if (res.tool !== 'disk_search') return diskFail('failed')
    if (!res.items.length) return toolResult(`No entries match (${res.total} total).`, res)
    const head = `${res.total} matching ${res.total === 1 ? 'entry' : 'entries'} (${res.ms} ms), showing ${res.items.length}, biggest first:`
    return toolResult([head, ...res.items.map(itemLine)].join('\n'), res)
  },
}
