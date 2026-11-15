import fs from 'node:fs/promises'
import type { DiskDeleteResult, DiskHistoryEntry } from '@ccm/shared'
import { dirSize } from '../diskClean/size'
import { normalizePath, realIo, resolveDeletable, sameCanonical, topLevel, type Canonical, type CanonicalIo, type GuardContext } from './guard'

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
  known?: (path: string) => Promise<boolean>
  measure?: (path: string) => Promise<number | null>
  io?: CanonicalIo
}

const SIZE_CONCURRENCY = 16
const TRASH_CONCURRENCY = 4
const NO_ACCESS = new Set(['EACCES', 'EPERM'])

export function e2eAutoConfirm(env: NodeJS.ProcessEnv, packaged: boolean): boolean {
  return !packaged && env.CCM_E2E_DISKMAP_CONFIRM === '1'
}

export async function measureRest(p: string): Promise<number | null> {
  const st = await fs.lstat(p).catch((err: unknown) => ((err as { code?: unknown })?.code === 'ENOENT' ? 0 : null))
  if (st === null || st === 0) return st
  if (st.isSymbolicLink()) return 0
  return st.isDirectory() ? dirSize(p).catch(() => null) : st.size
}

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
  const io = deps.io ?? realIo
  const allowed: string[] = []
  const canon = new Map<string, Canonical>()
  const keyOf = (p: string): string => normalizePath(p, ctx.platform).toLowerCase()
  for (const p of paths) {
    const { reason, canonical } = await resolveDeletable(p, ctx, io)
    if (reason || !canonical) {
      result.failed.push({ path: typeof p === 'string' ? p : '', reason: reason ?? 'invalid' })
      continue
    }
    if (deps.known && !(await deps.known(p).catch(() => false))) {
      result.failed.push({ path: p, reason: 'outside' })
      continue
    }
    allowed.push(p)
    canon.set(keyOf(p), canonical)
  }
  const targets = topLevel(allowed, ctx.platform)
  if (!targets.length) return result
  const sizes = await mapLimit(targets, SIZE_CONCURRENCY, (p) => deps.sizeOf(p).catch(() => null))
  const ask: DeleteAsk = { count: targets.length, bytes: sizes.reduce<number>((a, s) => a + (s ?? 0), 0), first: targets.slice(0, 3), permanent }
  if (!(await deps.confirm(ask)) || (permanent && !(await deps.confirmPermanent(ask)))) {
    return { ...result, cancelled: true }
  }
  const gone: string[] = []
  const partial: string[] = []
  await mapLimit(targets, deps.concurrency ?? TRASH_CONCURRENCY, async (p, i) => {
    const before = canon.get(keyOf(p))
    const again = await resolveDeletable(p, ctx, io)
    if (again.reason || !again.canonical || !before || !sameCanonical(before, again.canonical, ctx.platform)) {
      result.failed.push({ path: p, reason: again.reason ?? 'changed' })
      return
    }
    const target = again.canonical.path
    const size = sizes[i] ?? 0
    try {
      await (permanent ? deps.remove(target) : deps.trash(target))
      result.done.push({ path: p, size })
      result.freed += size
      gone.push(p)
    } catch (err) {
      const reason = reasonOf(err)
      result.failed.push({ path: p, reason })
      if (reason === 'ENOENT') gone.push(p)
      else if (permanent && !NO_ACCESS.has(reason)) {
        gone.push(p)
        const rest = deps.measure ? await deps.measure(target).catch(() => null) : null
        if (rest !== null && size > rest) {
          result.freed += size - rest
          partial.push(p)
        }
      }
    }
  })
  const order = new Map(targets.map((p, i) => [p, i]))
  result.done.sort((a, b) => order.get(a.path)! - order.get(b.path)!)
  if (gone.length) await deps.forget(gone).catch(() => undefined)
  if (result.done.length || result.failed.length) {
    await deps
      .record({ at: (deps.now ?? Date.now)(), action: permanent ? 'delete' : 'trash', count: result.done.length, freed: result.freed, paths: [...result.done.map((d) => d.path), ...partial], failed: result.failed.length })
      .catch(() => undefined)
  }
  return result
}
