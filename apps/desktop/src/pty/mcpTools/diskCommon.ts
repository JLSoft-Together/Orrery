import type { AgentDiskItem, McpDeskResult } from '../../../../../packages/shared/src/mcpDesk'
import { toolResult, type ToolHost, type ToolResult } from './types'

export const DISK_KIND_VALUES = ['image', 'video', 'audio', 'doc', 'archive', 'installer', 'code'] as const
export const DISK_DELETE_RATE_MAX = 3
export const DISK_DELETE_RATE_MS = 60_000
export const DISK_READ_RATE_MAX = 30
export const DISK_READ_RATE_MS = 60_000

const ERRORS: Record<string, string> = {
  'not-indexed': 'The disk has not been scanned yet. Ask the user to open Settings > Data > Disk and run a scan; Orrery never scans because an agent asked.',
  scanning: 'The disk index is being built right now; try again when the scan has finished.',
  'not-in-index': 'That path is not in the disk index (outside the scanned folders, or it did not exist at the last scan).',
  busy: 'Another delete is waiting for the user; try again after they answer it.',
  rate: 'Too many requests from this session in the last minute; wait a little.',
  'rate-delete': 'Too many delete proposals from this session in the last minute (max 3); wait a little.',
  unavailable: 'Orrery cannot use the disk index right now.',
  invalid: 'Orrery rejected the arguments.',
  failed: 'Orrery could not complete the disk request.',
}

export const ARG_ERRORS: Record<string, string> = {
  query: 'query must be a string of at most 1024 characters.',
  kind: `kind must be one of ${DISK_KIND_VALUES.join(', ')}.`,
  size: 'minSize and maxSize must be whole numbers of bytes, 0 or more.',
  days: 'olderThanDays must be a whole number from 0 to 36500.',
  under: 'under must be an absolute path.',
  limit: 'limit must be a whole number in the range the tool allows.',
  path: 'path must be an absolute path.',
  paths: 'paths must be a list of 1 to 200 absolute paths.',
  reason: 'reason is required: a short text of at most 500 characters saying why these paths should go.',
  permanent: 'permanent must be true or false.',
}

export const isAbsPath = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && v.length <= 4096 && !/[\0\r\n]/.test(v) && (/^[A-Za-z]:[\\/]/.test(v) || /^[\\/]{2}[^\\/?.][^\\/]*[\\/][^\\/]+/.test(v) || v.startsWith('/'))

export const optInt = (v: unknown, lo: number, hi: number): number | undefined | null => (v === undefined || v === null ? undefined : typeof v === 'number' && Number.isInteger(v) && v >= lo && v <= hi ? v : null)

export function diskFail(error: string, field?: string, rate = false): ToolResult {
  const text = field ? (ARG_ERRORS[field] ?? ERRORS.invalid!) : ((rate && error === 'rate' ? ERRORS['rate-delete'] : ERRORS[error]) ?? ERRORS.failed!)
  return toolResult(text, { ok: false, error, ...(field ? { detail: field } : {}) }, true)
}

export function sizeText(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = Math.max(0, bytes)
  let u = 0
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024
    u++
  }
  return u === 0 ? `${v} B` : `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[u]}`
}

const dateText = (ms: number): string => (ms > 0 ? new Date(ms).toISOString().slice(0, 10) : 'unknown date')

export function itemLine(it: AgentDiskItem): string {
  return `- ${sizeText(it.size)} ${it.path} [${it.dir ? 'folder' : (it.kind ?? 'file')}, modified ${dateText(it.mtime)}]`
}

export async function askDesk(host: ToolHost, call: Parameters<NonNullable<ToolHost['desk']>>[0]): Promise<McpDeskResult> {
  if (!host.desk) return { ok: false, error: 'unavailable' }
  try {
    return await host.desk(call)
  } catch {
    return { ok: false, error: 'failed' }
  }
}
