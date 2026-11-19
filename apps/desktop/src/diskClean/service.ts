import fs from 'node:fs/promises'
import path from 'node:path'
import {
  CLEAN_ONE_TAP,
  CLEAN_PERMANENT_OK,
  type CleanBinResult,
  type CleanCategory,
  type CleanCommandResult,
  type CleanDeleteRequest,
  type CleanItem,
  type CleanLock,
  type CleanNote,
  type CleanProgress,
  type CleanScanRequest,
  type CleanScanResult,
  type DiskDeleteResult,
  type DiskHistoryEntry,
} from '@ccm/shared'
import { Cancelled } from '../agentData/fsTree'
import { refusedRoot } from '../buildClean/scan'
import { measureRest, runDelete, type DeleteAsk } from '../diskMap/delete'
import { canonicalOf, isUnder, normalizePath, topLevel } from '../diskMap/guard'
import { findJunk, type JunkHit } from './projects'
import { dirSize } from './size'
import { DEV_PROVIDERS, type CleanCtx, type CleanProvider } from './providers'
import { DOCKER_KINDS, DOCKER_PRUNE, DOCKER_PRUNE_MS, dockerProvider, dockerRows, type DockerDeps, type DockerKind } from './docker'
import { avdBusy, SDK_PROVIDERS, type SdkDeps } from './sdk'
import { appCacheLock, browserSpots, isForbidden, runningApps, runningBrowsers, SYSTEM_PROVIDERS, type Member, type SystemDeps } from './system'

export interface RunningHolder {
  cwd: string
  lock: Extract<CleanLock, 'run' | 'server'>
}

export interface OneTapAsk {
  bytes: number
  count: number
  permBytes: number
  permCount: number
  trashBytes: number
  trashCount: number
}

export type OneTapChoice = 'clean' | 'trash' | 'cancel'

export interface CommandAsk {
  kind: string
  bytes: number
  count: number
  command: string
  warn: boolean
}

export interface DiskCleanDeps {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  providers?: CleanProvider[]
  system?: SystemDeps
  sdk?: SdkDeps
  docker?: DockerDeps
  askCommand?: (ask: CommandAsk) => Promise<boolean>
  allowTree?: () => Promise<string[]>
  self: () => string[]
  running: () => RunningHolder[]
  openNow?: () => Promise<string[]>
  keep: () => string[]
  emit: (p: CleanProgress) => void
  confirm: (ask: DeleteAsk, second: boolean) => Promise<boolean>
  askOneTap?: (ask: OneTapAsk) => Promise<OneTapChoice>
  askBin?: (ask: { bytes: number; count: number }) => Promise<boolean>
  trash: (p: string) => Promise<void>
  remove: (p: string) => Promise<void>
  record: (e: DiskHistoryEntry) => Promise<void>
  forget?: (paths: string[]) => Promise<void>
  indexSize?: (p: string) => Promise<number | null>
  now?: () => number
}

export function lockOf(item: Pick<CleanItem, 'path' | 'project'>, open: readonly string[], running: readonly RunningHolder[], self: readonly string[], platform: NodeJS.Platform): CleanLock | undefined {
  const inside = (cwd: string): boolean => (!!item.project && isUnder(cwd, item.project, platform)) || isUnder(cwd, item.path, platform)
  if (self.some((s) => s && (isUnder(item.path, s, platform) || isUnder(s, item.path, platform)))) return 'self'
  if (open.some((o) => o && inside(o))) return 'open'
  const r = running.find((h) => h.cwd && inside(h.cwd))
  return r?.lock
}

const fail = (code: string): Error => Object.assign(new Error(code), { code })

interface Target {
  path: string
  item: CleanItem
  member?: Member
}

export class DiskCleanService {
  private abort: AbortController | null = null
  private deleting = false
  private readonly last = new Map<string, CleanItem>()
  private readonly members = new Map<string, Member[]>()
  private readonly providers: CleanProvider[]

  constructor(private readonly deps: DiskCleanDeps) {
    const base = deps.providers ?? [...DEV_PROVIDERS, ...(deps.sdk ? SDK_PROVIDERS : []), ...(deps.docker ? [dockerProvider] : []), ...(deps.system ? SYSTEM_PROVIDERS : [])]
    this.providers = base.filter((p) => !p.platforms || p.platforms.includes(deps.platform))
  }

  private key(p: string): string {
    const n = normalizePath(p, this.deps.platform)
    return this.deps.platform === 'win32' || this.deps.platform === 'darwin' ? n.toLowerCase() : n
  }

  private now(): number {
    return (this.deps.now ?? Date.now)()
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

  private async openList(fromPage: readonly string[]): Promise<string[]> {
    const own = await (this.deps.openNow?.() ?? Promise.resolve([] as string[])).catch(() => [] as string[])
    const out: string[] = []
    for (const p of [...own, ...fromPage]) if (p && !out.some((o) => this.key(o) === this.key(p))) out.push(p)
    return out
  }

  async scan(req: CleanScanRequest): Promise<CleanScanResult> {
    if (this.abort) return { items: [], busy: true }
    const abort = new AbortController()
    this.abort = abort
    const signal = abort.signal
    const items: CleanItem[] = []
    const notes: Record<string, CleanNote> = {}
    const attached = new Map<string, Member[]>()
    try {
      const roots = await this.cleanRoots(req.roots)
      const running = this.deps.running()
      const self = this.deps.self()
      const open = await this.openList(req.open)
      const keep = this.deps.keep()
      const lock = (it: CleanItem): CleanItem => {
        const l = it.action === 'files' || it.action === 'bin' || it.action === 'command' ? undefined : lockOf(it, open, running, self, this.deps.platform)
        const { locked: own, ...rest } = it
        const final = l ?? own
        return final ? { ...rest, locked: final } : rest
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
          keep,
          now: this.now(),
          ...(this.deps.system ? { system: this.deps.system } : {}),
          ...(this.deps.sdk ? { sdk: this.deps.sdk } : {}),
          ...(this.deps.docker ? { docker: this.deps.docker } : {}),
          note: (n) => {
            notes[id] = n
          },
          attach: (p, list) => attached.set(this.key(p), list),
          keepMember: (p) => !lockOf({ path: p }, open, running, self, this.deps.platform),
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
        this.deps.emit({ category: id, state: 'done', count, bytes, ...(notes[id] ? { note: notes[id] } : {}) })
      }
      for (const [k, v] of this.last) {
        if (req.ids.includes(v.category)) {
          this.last.delete(k)
          this.members.delete(k)
        }
      }
      for (const it of items) {
        const k = this.key(it.path)
        this.last.set(k, it)
        const m = attached.get(k)
        if (m && it.action === 'files') this.members.set(k, m)
      }
      return { items, ...(Object.keys(notes).length ? { notes } : {}) }
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

  membersOf(p: string): Member[] {
    return this.members.get(this.key(p)) ?? []
  }

  private async liveLock(item: CleanItem): Promise<CleanLock | undefined> {
    if (item.category === 'avd' && this.deps.sdk) return (await avdBusy(item, this.deps.sdk)) ? 'app' : undefined
    const sys = this.deps.system
    if (!sys) return undefined
    if (item.category === 'browser') {
      const running = await runningBrowsers(sys)
      const spot = browserSpots(sys.roots).find((b) => b.id === item.kind)
      return spot && running.has(spot.id) ? 'browser' : undefined
    }
    if (item.category === 'appcache') return appCacheLock(path.basename(item.path), await runningApps(sys)) ? 'app' : undefined
    return undefined
  }

  private systemSafe(p: string): boolean {
    const sys = this.deps.system
    return !sys || !isForbidden(p, sys.roots, this.deps.keep())
  }

  private async realParent(p: string): Promise<string | null> {
    const lib = this.deps.platform === 'win32' ? path.win32 : path.posix
    const real = await fs.realpath(lib.dirname(p)).catch(() => null)
    return real ? this.key(real) : null
  }

  async remove(req: CleanDeleteRequest): Promise<DiskDeleteResult> {
    const permanent = req.permanent
    if (this.deleting || this.abort) return { done: [], failed: req.paths.map((p) => ({ path: p, reason: 'busy' })), freed: 0, permanent }
    this.deleting = true
    try {
      const failed: DiskDeleteResult['failed'] = []
      const targets: Target[] = []
      const running = this.deps.running()
      const self = this.deps.self()
      const open = await this.openList(req.open)
      for (const p of req.paths) {
        const item = this.known(p)
        if (!item || item.action === 'bin' || item.action === 'command' || (req.oneTap && !CLEAN_ONE_TAP.includes(item.category))) {
          failed.push({ path: p, reason: 'invalid' })
          continue
        }
        if (item.locked) {
          failed.push({ path: item.path, reason: `locked:${item.locked}` })
          continue
        }
        const st = await fs.lstat(item.path).catch(() => null)
        if (!st) {
          failed.push({ path: item.path, reason: 'ENOENT' })
          continue
        }
        const wantFile = item.file === true
        if (st.isSymbolicLink() || (wantFile ? !st.isFile() : !st.isDirectory())) {
          failed.push({ path: item.path, reason: 'changed' })
          continue
        }
        const live = (item.action === 'files' ? undefined : lockOf(item, open, running, self, this.deps.platform)) ?? (await this.liveLock(item))
        if (live) {
          failed.push({ path: item.path, reason: `locked:${live}` })
          continue
        }
        if (item.group === 'system' && item.action !== 'files' && !this.systemSafe(item.path)) {
          failed.push({ path: item.path, reason: 'protected' })
          continue
        }
        if (item.action === 'files') {
          const list = this.membersOf(item.path)
          let skipped = 0
          for (const m of list) {
            const ms = await fs.lstat(m.path).catch(() => null)
            const ok =
              !!ms &&
              !ms.isSymbolicLink() &&
              (m.file ? ms.isFile() : ms.isDirectory()) &&
              ms.mtimeMs <= m.mtime + 2000 &&
              isUnder(m.path, item.path, this.deps.platform) &&
              this.systemSafe(m.path) &&
              !lockOf({ path: m.path }, open, running, self, this.deps.platform) &&
              !(m.file && (await this.deps.system?.busy(m.path).catch(() => true)))
            if (ok) targets.push({ path: m.path, item, member: m })
            else skipped++
          }
          if (skipped) failed.push({ path: item.path, reason: `skipped:${skipped}` })
          continue
        }
        targets.push({ path: item.path, item })
      }
      if (!targets.length) return { done: [], failed, freed: 0, permanent, ...(failed.length ? {} : { cancelled: true }) }
      const parents = new Map<string, string | null>()
      for (const t of targets) parents.set(this.key(t.path), await this.realParent(t.path))
      const byKey = new Map(targets.map((t) => [this.key(t.path), t]))
      for (const t of targets) {
        const c = await canonicalOf(t.path, this.deps.platform).catch(() => null)
        if (!c || this.key(c.path) === this.key(t.path)) continue
        byKey.set(this.key(c.path), t)
        parents.set(this.key(c.path), parents.get(this.key(t.path)) ?? null)
      }
      let fresh = { open, running, self }
      const refresh = async (): Promise<void> => {
        fresh = { open: await this.openList(req.open), running: this.deps.running(), self: this.deps.self() }
      }
      const precheck = async (p: string): Promise<void> => {
        const t = byKey.get(this.key(p))
        if (!t) throw fail('invalid')
        const st = await fs.lstat(p).catch(() => null)
        if (!st) throw fail('ENOENT')
        const wantFile = t.member ? t.member.file : t.item.file === true
        if (st.isSymbolicLink() || (wantFile ? !st.isFile() : !st.isDirectory())) throw fail('changed')
        const before = parents.get(this.key(p))
        if (!before || (await this.realParent(p)) !== before) throw fail('changed')
        const l = lockOf(t.member ? { path: p } : t.item, fresh.open, fresh.running, fresh.self, this.deps.platform)
        if (l) throw fail(`locked:${l}`)
      }
      const trash = async (p: string): Promise<void> => {
        await precheck(p)
        await this.deps.trash(p)
      }
      const remove = async (p: string): Promise<void> => {
        await precheck(p)
        await this.deps.remove(p)
      }
      const sizeOf = async (p: string): Promise<number | null> => byKey.get(this.key(p))?.member?.size ?? this.known(p)?.size ?? null
      const forget = async (gone: string[]): Promise<void> => {
        const items = new Set<string>()
        for (const g of gone) {
          const t = byKey.get(this.key(g))
          if (!t) continue
          if (t.member) {
            const k = this.key(t.item.path)
            const rest = (this.members.get(k) ?? []).filter((m) => this.key(m.path) !== this.key(g))
            this.members.set(k, rest)
            if (!rest.length) {
              this.last.delete(k)
              this.members.delete(k)
            }
          } else {
            this.last.delete(this.key(g))
            items.add(g)
          }
        }
        if (items.size) await this.deps.forget?.([...items])
      }
      const allowTree = (await this.deps.allowTree?.().catch(() => [] as string[])) ?? []
      const guard = (paths: string[]) => async () => ({ platform: this.deps.platform, home: this.deps.home, keep: this.deps.keep(), roots: paths, ...(allowTree.length ? { allowTree } : {}) })
      const yes = async (): Promise<boolean> => {
        await refresh()
        return true
      }
      const base = { sizeOf, trash, remove, forget, record: this.deps.record, measure: measureRest }
      let results: DiskDeleteResult[]
      if (req.oneTap) {
        const perm = targets.filter((t) => CLEAN_PERMANENT_OK.includes(t.item.category))
        const soft = targets.filter((t) => !CLEAN_PERMANENT_OK.includes(t.item.category))
        const sizes = (list: Target[]): number => list.reduce((s, t) => s + (t.member?.size ?? t.item.size), 0)
        const ask: OneTapAsk = { bytes: sizes(targets), count: targets.length, permBytes: sizes(perm), permCount: perm.length, trashBytes: sizes(soft), trashCount: soft.length }
        const choice = this.deps.askOneTap ? await this.deps.askOneTap(ask) : 'cancel'
        if (choice === 'cancel') return { done: [], failed, freed: 0, permanent, cancelled: true }
        await refresh()
        const hard = choice === 'clean' ? perm.map((t) => t.path) : []
        const rest = choice === 'clean' ? soft.map((t) => t.path) : targets.map((t) => t.path)
        results = []
        if (hard.length) results.push(await runDelete(hard, true, { ...base, guard: guard(hard), confirm: yes, confirmPermanent: yes }))
        if (rest.length) results.push(await runDelete(rest, false, { ...base, guard: guard(rest), confirm: yes, confirmPermanent: yes }))
      } else {
        const all = targets.map((t) => t.path)
        results = [
          await runDelete(all, permanent, {
            ...base,
            guard: guard(all),
            confirm: async (a) => (await this.deps.confirm(a, false)) && (await yes()),
            confirmPermanent: async (a) => (await this.deps.confirm(a, true)) && (await yes()),
          }),
        ]
      }
      return this.fold(results, failed, permanent, byKey)
    } finally {
      this.deleting = false
    }
  }

  private fold(results: DiskDeleteResult[], failed: DiskDeleteResult['failed'], permanent: boolean, byKey: Map<string, Target>): DiskDeleteResult {
    const done = new Map<string, { path: string; size: number }>()
    const fails = new Map<string, { path: string; reason: string; n: number }>()
    let freed = 0
    let cancelled = results.length > 0
    let anyPermanent = false
    for (const r of results) {
      if (!r.cancelled) cancelled = false
      if (r.permanent && r.done.length) anyPermanent = true
      freed += r.freed
      for (const d of r.done) {
        const t = byKey.get(this.key(d.path))
        const owner = t?.member ? t.item.path : d.path
        const cur = done.get(this.key(owner)) ?? { path: owner, size: 0 }
        cur.size += d.size
        done.set(this.key(owner), cur)
      }
      for (const f of r.failed) {
        const t = byKey.get(this.key(f.path))
        if (t?.member) {
          const k = this.key(t.item.path)
          const cur = fails.get(k) ?? { path: t.item.path, reason: f.reason, n: 0 }
          cur.n++
          fails.set(k, cur)
        } else fails.set(this.key(f.path), { path: f.path, reason: f.reason, n: 0 })
      }
    }
    const extra = [...fails.values()].map((f) => ({ path: f.path, reason: f.n ? `some:${f.n}:${f.reason}` : f.reason }))
    const res: DiskDeleteResult = { done: [...done.values()], failed: [...failed, ...extra], freed, permanent: permanent || anyPermanent }
    return cancelled && !res.done.length ? { ...res, cancelled: true } : res
  }

  commandOf(item: CleanItem): { tool: 'docker' | 'xcrun'; args: readonly string[] } | null {
    if (item.action !== 'command') return null
    if (item.category === 'docker' && (DOCKER_KINDS as readonly string[]).includes(item.kind ?? '')) return { tool: 'docker', args: DOCKER_PRUNE[item.kind as DockerKind] }
    if (item.category === 'ios' && item.kind === 'simUnavailable') return { tool: 'xcrun', args: ['simctl', 'delete', 'unavailable'] }
    return null
  }

  async runCommand(p: string): Promise<CleanCommandResult> {
    if (this.deleting || this.abort) return { done: false, freed: 0, error: 'busy' }
    const item = this.known(p)
    const cmd = item ? this.commandOf(item) : null
    if (!item || !cmd) return { done: false, freed: 0, error: 'invalid' }
    const run = cmd.tool === 'docker' ? this.deps.docker?.run : this.deps.sdk?.simctl ? async (args: string[]): Promise<{ ok: boolean; code?: string; stdout: string }> => ({ ok: (await this.deps.sdk!.simctl!(args)) !== null, stdout: '' }) : undefined
    if (!run) return { done: false, freed: 0, error: 'invalid' }
    this.deleting = true
    try {
      const ask: CommandAsk = { kind: item.kind ?? '', bytes: item.size, count: item.files ?? 0, command: `${cmd.tool} ${cmd.args.join(' ')}`, warn: item.warn === true }
      if (!this.deps.askCommand || !(await this.deps.askCommand(ask))) return { done: false, freed: 0, cancelled: true }
      const res = await run([...cmd.args], DOCKER_PRUNE_MS)
      if (!res.ok) return { done: false, freed: 0, error: res.code ?? 'failed' }
      let freed = item.size
      if (cmd.tool === 'docker' && this.deps.docker) {
        const after = (await dockerRows(this.deps.docker).catch(() => null))?.find((r) => r.kind === item.kind)
        if (after) {
          freed = Math.max(0, item.size - after.reclaimable)
          this.last.set(this.key(item.path), { ...item, size: after.reclaimable, files: Math.max(0, after.total - after.active), detail: `${after.total}/${after.active}` })
        }
      } else this.last.set(this.key(item.path), { ...item, size: 0, files: 0 })
      await this.deps.record({ at: this.now(), action: 'delete', count: item.files ?? 1, freed, paths: [item.path], failed: 0 }).catch(() => undefined)
      return { done: true, freed }
    } finally {
      this.deleting = false
    }
  }

  async emptyBin(): Promise<CleanBinResult> {
    const sys = this.deps.system
    if (!sys || this.deleting || this.abort) return { done: false, freed: 0, error: 'busy' }
    const item = [...this.last.values()].find((i) => i.action === 'bin')
    if (!item) return { done: false, freed: 0, error: 'invalid' }
    this.deleting = true
    try {
      const info = await sys.bin.size().catch(() => null)
      const bytes = info?.bytes ?? item.size
      const count = info?.count ?? item.files ?? 0
      if (!this.deps.askBin || !(await this.deps.askBin({ bytes, count }))) return { done: false, freed: 0, cancelled: true }
      try {
        await sys.bin.empty()
      } catch (err) {
        return { done: false, freed: 0, error: String((err as { code?: unknown }).code ?? 'failed') }
      }
      this.last.set(this.key(item.path), { ...item, size: 0, files: 0 })
      await this.deps.record({ at: this.now(), action: 'delete', count, freed: bytes, paths: [item.path], failed: 0 }).catch(() => undefined)
      return { done: true, freed: bytes }
    } finally {
      this.deleting = false
    }
  }
}

