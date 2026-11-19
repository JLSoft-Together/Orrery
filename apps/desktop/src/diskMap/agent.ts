import type { McpDeskCall, McpDeskResult } from '../../../../packages/shared/src/mcpDesk'
import type { DeleteAsk } from './delete'
import type { DiskMapService } from './ipc'
import { cleanSearch, isAbsolutePath, toChildren, toSearch } from './map'

export const AGENT_SEARCH_MAX = 200
export const AGENT_USAGE_MAX = 100
export const AGENT_DELETE_MAX = 200
export const AGENT_REASON_MAX = 500
const DAY_MS = 86_400_000
const DISPLAY_STRIP = /[\x00-\x1f\x7f-\x9f\u2028\u2029\u202a-\u202e\u2066-\u2069]+/g

export type DiskDeskCall = Extract<McpDeskCall, { tool: 'disk_search' | 'disk_usage' | 'disk_propose_delete' }>

export interface AgentConfirm {
  pane: string
  reason: string
  ask: DeleteAsk
  second: boolean
}

export interface DiskAgentDeps {
  service: () => DiskMapService | null
  platform: NodeJS.Platform
  confirm: (spec: AgentConfirm) => Promise<boolean>
  now?: () => number
}

export const displayText = (v: string, max: number): string => v.replace(DISPLAY_STRIP, ' ').trim().slice(0, max)

const fail = (error: Extract<McpDeskResult, { ok: false }>['error']): McpDeskResult => ({ ok: false, error })

const toItem = (e: ReturnType<typeof toSearch>['items'][number]) => ({ path: e.path, name: e.name, size: e.size, dir: e.dir, mtime: e.mtime, ...(e.kind ? { kind: e.kind } : {}) })

export function createDiskAgent(deps: DiskAgentDeps): (call: DiskDeskCall) => Promise<McpDeskResult> {
  const now = deps.now ?? Date.now

  async function ready(svc: DiskMapService): Promise<McpDeskResult | null> {
    const st = await svc.status().catch(() => null)
    if (!st) return fail('unavailable')
    if (!st.indexed) return fail(st.scanning ? 'scanning' : 'not-indexed')
    return null
  }

  const intIn = (v: unknown, lo: number, hi: number): v is number => typeof v === 'number' && Number.isInteger(v) && v >= lo && v <= hi

  async function search(svc: DiskMapService, call: Extract<DiskDeskCall, { tool: 'disk_search' }>): Promise<McpDeskResult> {
    if (!intIn(call.limit, 1, AGENT_SEARCH_MAX)) return fail('invalid')
    if (call.olderThanDays !== undefined && !intIn(call.olderThanDays, 0, 36_500)) return fail('invalid')
    const q = cleanSearch(
      {
        query: call.query,
        kind: call.kind,
        minSize: call.minSize,
        maxSize: call.maxSize,
        under: call.under,
        before: call.olderThanDays === undefined ? undefined : Math.max(0, now() - call.olderThanDays * DAY_MS),
        sort: 'size',
        limit: call.limit,
      },
      deps.platform,
    )
    if (!q) return fail('invalid')
    const early = await ready(svc)
    if (early) return early
    let raw: unknown
    try {
      raw = await svc.host.call('disk.search', q)
    } catch (err) {
      return fail(err instanceof Error && err.message.startsWith('not in index') ? 'not-in-index' : 'failed')
    }
    const res = toSearch(raw)
    return { ok: true, tool: 'disk_search', total: res.total, ms: res.ms, items: res.items.slice(0, AGENT_SEARCH_MAX).map(toItem) }
  }

  async function usage(svc: DiskMapService, call: Extract<DiskDeskCall, { tool: 'disk_usage' }>): Promise<McpDeskResult> {
    if (!isAbsolutePath(call.path, deps.platform) || !intIn(call.limit, 1, AGENT_USAGE_MAX)) return fail('invalid')
    const early = await ready(svc)
    if (early) return early
    let raw: unknown
    try {
      raw = await svc.host.call('disk.children', { path: call.path, sort: 'size', limit: call.limit })
    } catch (err) {
      return fail(err instanceof Error && err.message.startsWith('not in index') ? 'not-in-index' : 'failed')
    }
    const res = toChildren(raw)
    if (!res) return fail('not-in-index')
    return { ok: true, tool: 'disk_usage', path: res.path || call.path, size: res.size, total: res.total, items: res.items.slice(0, AGENT_USAGE_MAX).map(toItem) }
  }

  async function propose(svc: DiskMapService, call: Extract<DiskDeskCall, { tool: 'disk_propose_delete' }>): Promise<McpDeskResult> {
    const paths = Array.isArray(call.paths) ? call.paths : []
    if (!paths.length || paths.length > AGENT_DELETE_MAX || !paths.every((p) => isAbsolutePath(p, deps.platform))) return fail('invalid')
    const reason = typeof call.reason === 'string' ? displayText(call.reason, AGENT_REASON_MAX) : ''
    if (!reason || typeof call.permanent !== 'boolean') return fail('invalid')
    if (svc.isDeleting) return fail('busy')
    const early = await ready(svc)
    if (early) return early
    const pane = displayText(typeof call.name === 'string' ? call.name : '', 80) || 'agent'
    const res = await svc.removeWith(
      [...new Set(paths)],
      call.permanent,
      (ask, second) => deps.confirm({ pane, reason, ask: { ...ask, first: ask.first.map((p) => displayText(p, 400)) }, second }),
      5,
    )
    if (!res.cancelled && !res.done.length && res.failed.length && res.failed.every((f) => f.reason === 'busy')) return fail('busy')
    return { ok: true, tool: 'disk_propose_delete', done: res.done, failed: res.failed, freed: res.freed, permanent: res.permanent, ...(res.cancelled ? { cancelled: true as const } : {}) }
  }

  return async (call) => {
    const svc = deps.service()
    if (!svc) return fail('unavailable')
    try {
      if (call.tool === 'disk_search') return await search(svc, call)
      if (call.tool === 'disk_usage') return await usage(svc, call)
      if (call.tool === 'disk_propose_delete') return await propose(svc, call)
      return fail('unavailable')
    } catch {
      return fail('failed')
    }
  }
}
