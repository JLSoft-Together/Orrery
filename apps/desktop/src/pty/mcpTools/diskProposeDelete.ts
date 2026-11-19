import { DISK_DELETE_MAX, DISK_REASON_MAX } from '../../../../../packages/shared/src/mcpDesk'
import { toolResult, type McpTool, type ToolDef } from './types'
import { askDesk, DISK_DELETE_RATE_MAX, DISK_DELETE_RATE_MS, diskFail, isAbsPath, sizeText } from './diskCommon'

export const DISK_PROPOSE_DELETE_TOOL = {
  name: 'disk_propose_delete',
  description:
    "Propose deleting files or folders to free disk space. Orrery always shows the user a native dialog (your session, your reason, the number of items, the total size, the first paths; the default button is Cancel) and nothing is removed unless the user confirms. Confirmed items go to the Recycle Bin / Trash (permanent: true asks the user a second time). Paths must be in Orrery's disk index and are checked against its protected list (system folders, the user's own folders and the like are refused). There is no way to skip the dialog. At most 3 proposals a minute.",
  inputSchema: {
    type: 'object',
    properties: {
      paths: { type: 'array', minItems: 1, maxItems: DISK_DELETE_MAX, items: { type: 'string', maxLength: 4096 }, description: 'Absolute paths to delete.' },
      reason: { type: 'string', minLength: 1, maxLength: DISK_REASON_MAX, description: 'Why these paths can go; shown to the user in the dialog.' },
      permanent: { type: 'boolean', description: 'Delete for good instead of moving to the Recycle Bin / Trash. Default false.' },
    },
    required: ['paths', 'reason'],
    additionalProperties: false,
  },
  annotations: { title: 'Propose deleting files (asks the user)', readOnlyHint: false, destructiveHint: true, openWorldHint: false, idempotentHint: false },
} satisfies ToolDef

export const diskProposeDelete: McpTool = {
  def: DISK_PROPOSE_DELETE_TOOL,
  async call(args, host) {
    const paths = args.paths
    if (!Array.isArray(paths) || paths.length < 1 || paths.length > DISK_DELETE_MAX || !paths.every(isAbsPath)) return diskFail('invalid', 'paths')
    if (typeof args.reason !== 'string' || args.reason.length > DISK_REASON_MAX || args.reason.includes('\0') || !args.reason.trim()) return diskFail('invalid', 'reason')
    if (args.permanent !== undefined && args.permanent !== null && typeof args.permanent !== 'boolean') return diskFail('invalid', 'permanent')
    if (!host.take('disk-delete', DISK_DELETE_RATE_MAX, DISK_DELETE_RATE_MS)) return diskFail('rate', undefined, true)
    const res = await askDesk(host, { tool: 'disk_propose_delete', paths: paths as string[], reason: args.reason, permanent: args.permanent === true })
    if (!res.ok) return diskFail(res.error)
    if (res.tool !== 'disk_propose_delete') return diskFail('failed')
    const bin = res.permanent ? 'deleted permanently' : 'moved to the Recycle Bin / Trash'
    const failed = res.failed.slice(0, 10).map((f) => `- ${f.path || '(invalid path)'}: ${f.reason}`)
    const more = res.failed.length > 10 ? [`...and ${res.failed.length - 10} more`] : []
    const tail = failed.length ? ['Not removed:', ...failed, ...more] : []
    if (res.cancelled) return toolResult('The user declined: nothing was deleted.', res)
    if (!res.done.length) return toolResult(['Nothing was deleted.', ...tail].join('\n'), res, true)
    return toolResult([`The user confirmed: ${res.done.length} ${res.done.length === 1 ? 'item' : 'items'} ${bin}, ${sizeText(res.freed)} freed.`, ...tail].join('\n'), res)
  },
}
