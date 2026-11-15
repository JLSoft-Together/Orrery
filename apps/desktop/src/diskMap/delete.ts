import type { DiskDeleteResult, DiskHistoryEntry } from '@ccm/shared'
import { checkDeletable, topLevel, type GuardContext } from './guard'

export interface DeleteAsk {
  count: number
  bytes: number
  first: string[]
  permanent: boolean
}

export interface DeleteDeps {
  guard: () => Promise<GuardContext>
  sizeOf: (path: string) => Promise<number | null>
  confirm: (ask: DeleteAsk) => Promise<boolean>
  confirmPermanent: (ask: DeleteAsk) => Promise<boolean>
  trash: (path: string) => Promise<void>
  remove: (path: string) => Promise<void>
  forget: (paths: string[]) => Promise<void>
  record: (entry: DiskHistoryEntry) => Promise<void>
  now?: () => number
  concurrency?: number
}

const SIZE_CONCURRENCY = 16
const TRASH_CONCURRENCY = 4

export function reasonOf(err: unknown): string {
  const e = err as { code?: unknown; message?: unknown } | null
  if (e && typeof e.code === 'string' && e.code) return e.code
  const msg = e && typeof e.message === 'string' ? e.message : String(err)
  return msg.replace(/\s+/g, ' ').trim().slice(0, 200) || 'failed'
}

export async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T, index: number) => Promise<R>): Promise<R[]> {
  const out = new Array<R>(items.length)
  let next = 0
  const worker = async (): Promise<void> => {
    while (next < items.length) {
      const i = next++
      out[i] = await fn(items[i]!, i)
    }
  }
  await Promise.all(Array.from({ length: Math.max(1, Math.min(limit, items.length)) }, worker))
  return out
}

export async function runDelete(paths: string[], permanent: boolean, deps: DeleteDeps): Promise<DiskDeleteResult> {
  const result: DiskDeleteResult = { done: [], failed: [], freed: 0, permanent }
  const ctx = await deps.guard()
  const allowed: string[] = []
  for (const p of paths) {
    const reason = checkDeletable(p, ctx)
    if (reason) result.failed.push({ path: typeof p === 'string' ? p : '', reason })
    else allowed.push(p)
  }
  const targets = topLevel(allowed, ctx.platform)
  if (!targets.length) return result
  const sizes = await mapLimit(targets, SIZE_CONCURRENCY, (p) => deps.sizeOf(p).catch(() => null))
  const ask: DeleteAsk = { count: targets.length, bytes: sizes.reduce<number>((a, s) => a + (s ?? 0), 0), first: targets.slice(0, 3), permanent }
  if (!(await deps.confirm(ask)) || (permanent && !(await deps.confirmPermanent(ask)))) {
    return { ...result, cancelled: true }
  }
  const gone: string[] = []
  await mapLimit(targets, deps.concurrency ?? TRASH_CONCURRENCY, async (p, i) => {
    try {
      await (permanent ? deps.remove(p) : deps.trash(p))
      const size = sizes[i] ?? 0
      result.done.push({ path: p, size })
      result.freed += size
      gone.push(p)
    } catch (err) {
      const reason = reasonOf(err)
      result.failed.push({ path: p, reason })
      if (reason === 'ENOENT') gone.push(p)
    }
  })
  const order = new Map(targets.map((p, i) => [p, i]))
  result.done.sort((a, b) => order.get(a.path)! - order.get(b.path)!)
  if (gone.length) await deps.forget(gone).catch(() => undefined)
  if (result.done.length || result.failed.length) {
    await deps
      .record({ at: (deps.now ?? Date.now)(), action: permanent ? 'delete' : 'trash', count: result.done.length, freed: result.freed, paths: result.done.map((d) => d.path), failed: result.failed.length })
      .catch(() => undefined)
  }
  return result
}
