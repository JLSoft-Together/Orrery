import fs from 'node:fs/promises'
import path from 'node:path'
import type { CleanCategory, CleanDeleteRequest, CleanItem, CleanLock, CleanProgress, CleanScanRequest, CleanScanResult, DiskDeleteResult, DiskHistoryEntry } from '@ccm/shared'
import { Cancelled } from '../agentData/fsTree'
import { refusedRoot } from '../buildClean/scan'
import { runDelete, type DeleteAsk } from '../diskMap/delete'
import { isUnder, normalizePath, topLevel } from '../diskMap/guard'
import { findJunk, type JunkHit } from './projects'
import { dirSize } from './size'
import { DEV_PROVIDERS, type CleanCtx, type CleanProvider } from './providers'

export interface RunningHolder {
  cwd: string
  lock: Extract<CleanLock, 'run' | 'server'>
}

export interface DiskCleanDeps {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  providers?: CleanProvider[]
  self: () => string[]
  running: () => RunningHolder[]
  keep: () => string[]
  emit: (p: CleanProgress) => void
  confirm: (ask: DeleteAsk, second: boolean) => Promise<boolean>
  trash: (p: string) => Promise<void>
  remove: (p: string) => Promise<void>
  record: (e: DiskHistoryEntry) => Promise<void>
  forget?: (paths: string[]) => Promise<void>
  indexSize?: (p: string) => Promise<number | null>
}

export function lockOf(item: Pick<CleanItem, 'path' | 'project'>, open: readonly string[], running: readonly RunningHolder[], self: readonly string[], platform: NodeJS.Platform): CleanLock | undefined {
  const inside = (cwd: string): boolean => (!!item.project && isUnder(cwd, item.project, platform)) || isUnder(cwd, item.path, platform)
  if (self.some((s) => s && (isUnder(item.path, s, platform) || isUnder(s, item.path, platform)))) return 'self'
  if (open.some((o) => o && inside(o))) return 'open'
  const r = running.find((h) => h.cwd && inside(h.cwd))
  return r?.lock
}

export class DiskCleanService {
  private abort: AbortController | null = null
  private deleting = false
  private readonly last = new Map<string, CleanItem>()
  private readonly providers: CleanProvider[]

  constructor(private readonly deps: DiskCleanDeps) {
    this.providers = (deps.providers ?? DEV_PROVIDERS).filter((p) => !p.platforms || p.platforms.includes(deps.platform))
  }

  private key(p: string): string {
    const n = normalizePath(p, this.deps.platform)
    return this.deps.platform === 'win32' || this.deps.platform === 'darwin' ? n.toLowerCase() : n
  }

  categories(): CleanCategory[] {
    return this.providers.map((p) => ({ id: p.id, group: p.group, preselect: p.preselect, titleKey: `diskClean.cat.${p.id}.title`, hintKey: `diskClean.cat.${p.id}.hint` }))
  }

  get scanning(): boolean {
    return !!this.abort
  }

  cancel(): boolean {
    if (!this.abort) return false
    this.abort.abort()
    return true
  }

  async cleanRoots(raw: readonly string[]): Promise<string[]> {
    const lib = this.deps.platform === 'win32' ? path.win32 : path.posix
    const ok: string[] = []
    for (const r of raw) {
      if (!lib.isAbsolute(r)) continue
      const abs = normalizePath(lib.resolve(r), this.deps.platform)
      if (refusedRoot(abs, this.deps.home, this.deps.platform)) continue
      const st = await fs.lstat(abs).catch(() => null)
      if (!st?.isDirectory() || st.isSymbolicLink()) continue
      ok.push(abs)
    }
    return topLevel(ok, this.deps.platform)
  }

  async scan(req: CleanScanRequest): Promise<CleanScanResult> {
    if (this.abort) return { items: [], busy: true }
    const abort = new AbortController()
    this.abort = abort
    const signal = abort.signal
    const items: CleanItem[] = []
    try {
      const roots = await this.cleanRoots(req.roots)
      const running = this.deps.running()
      const self = this.deps.self()
      const lock = (it: CleanItem): CleanItem => {
        const l = lockOf(it, req.open, running, self, this.deps.platform)
        const { locked: _drop, ...rest } = it
        return l ? { ...rest, locked: l } : rest
      }
      let junk: Promise<JunkHit[]> | null = null
      const seen = new Set<string>()
      for (const id of req.ids) {
        const provider = this.providers.find((p) => p.id === id)
        if (!provider) continue
        if (signal.aborted) throw new Cancelled()
        let count = 0
        let bytes = 0
        this.deps.emit({ category: id, state: 'start', count, bytes })
        const ctx: CleanCtx = {
          roots,
          home: this.deps.home,
          platform: this.deps.platform,
          env: this.deps.env,
          signal,
          junk: () => (junk ??= (async () => (await Promise.all(roots.map((r) => findJunk(r, signal)))).flat())()),
          size: async (p) => {
            const hint = await this.deps.indexSize?.(p).catch(() => null)
            if (typeof hint === 'number' && hint >= 0) return hint
            return dirSize(p, signal)
          },
          emit: (batch) => {
            const fresh = batch.filter((b) => !seen.has(this.key(b.path))).map(lock)
            if (!fresh.length) return
            for (const f of fresh) {
              seen.add(this.key(f.path))
              items.push(f)
              count++
              bytes += f.size
            }
            this.deps.emit({ category: id, state: 'items', items: fresh, count, bytes })
          },
        }
        await provider.scan(ctx)
        if (signal.aborted) throw new Cancelled()
        this.deps.emit({ category: id, state: 'done', count, bytes })
      }
      for (const [k, v] of this.last) if (req.ids.includes(v.category)) this.last.delete(k)
      for (const it of items) this.last.set(this.key(it.path), it)
      return { items }
    } catch (err) {
      if (err instanceof Cancelled || signal.aborted) return { items, cancelled: true }
      throw err
    } finally {
      this.abort = null
    }
  }

  known(p: string): CleanItem | undefined {
    return this.last.get(this.key(p))
  }

  async remove(req: CleanDeleteRequest): Promise<DiskDeleteResult> {
    const permanent = req.permanent
    if (this.deleting || this.abort) return { done: [], failed: req.paths.map((p) => ({ path: p, reason: 'busy' })), freed: 0, permanent }
    this.deleting = true
    try {
      const failed: DiskDeleteResult['failed'] = []
      const allowed: string[] = []
      const running = this.deps.running()
      const self = this.deps.self()
      for (const p of req.paths) {
        const item = this.known(p)
        if (!item) {
          failed.push({ path: p, reason: 'invalid' })
          continue
        }
        const st = await fs.lstat(item.path).catch(() => null)
        if (!st) {
          failed.push({ path: item.path, reason: 'ENOENT' })
          continue
        }
        if (!st.isDirectory() || st.isSymbolicLink()) {
          failed.push({ path: item.path, reason: 'changed' })
          continue
        }
        const l = lockOf(item, req.open, running, self, this.deps.platform)
        if (l) {
          failed.push({ path: item.path, reason: `locked:${l}` })
          continue
        }
        allowed.push(item.path)
      }
      if (!allowed.length) return { done: [], failed, freed: 0, permanent, ...(failed.length ? {} : { cancelled: true }) }
      const result = await runDelete(allowed, permanent, {
        guard: async () => ({ platform: this.deps.platform, home: this.deps.home, keep: this.deps.keep(), roots: allowed }),
        sizeOf: async (p) => this.known(p)?.size ?? null,
        confirm: (a) => this.deps.confirm(a, false),
        confirmPermanent: (a) => this.deps.confirm(a, true),
        trash: this.deps.trash,
        remove: this.deps.remove,
        forget: async (gone) => {
          for (const g of gone) this.last.delete(this.key(g))
          await this.deps.forget?.(gone)
        },
        record: this.deps.record,
      })
      return { ...result, failed: [...failed, ...result.failed] }
    } finally {
      this.deleting = false
    }
  }
}
