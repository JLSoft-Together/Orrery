import { promises as fs, watch as fsWatch, type FSWatcher } from 'node:fs'
import path from 'node:path'
import { randomBytes } from 'node:crypto'
import {
  ORGANIZE_KINDS,
  ORGANIZE_STABLE_MS,
  type OrganizeJunkResult,
  type OrganizeMode,
  type OrganizePlan,
  type OrganizeRunResult,
  type OrganizeSettings,
  type OrganizeSkip,
  type OrganizeSource,
  type OrganizeState,
  type OrganizeUndoResult,
} from '@ccm/shared'
import { canonicalOf, normalizePath, resolveDeletable, type CanonicalIo, type GuardContext } from '../diskMap/guard'
import { exists, moveCode, moveFile, realMoveIo, type MoveIo } from './move'
import { buildPlan, freeName, nameKey, realPlanIo, touchedAt, type PlanIo, type PlannedMove } from './plan'
import { RunStore, newRunId, type RunWriter } from './log'
import { cleanFolder, cleanPatch } from './rules'
import { scanJunk, type JunkDeps } from './junk'

export interface OrganizeDeps {
  dataDir: string
  platform: NodeJS.Platform
  home: string
  downloads: string
  desktop: string
  appDirs: string[]
  locale: () => string
  stableMs?: number
  now?: () => number
  broadcast?: (payload: unknown) => void
  junk?: Omit<JunkDeps, 'platform' | 'skipTree'>
  junkAllow?: string[]
  planIo?: PlanIo
  moveIo?: MoveIo
  guardIo?: CanonicalIo
  watch?: (dir: string, onChange: () => void, onError: () => void) => { close(): void }
  setTimer?: (fn: () => void, ms: number) => unknown
  clearTimer?: (t: unknown) => void
}

interface StoredSettings extends OrganizeSettings {
  since: Record<string, number>
}

interface PlanEntry {
  plan: OrganizePlan
  moves: Map<string, PlannedMove>
  at: number
}

const PLAN_TTL = 30 * 60_000
const MAX_PLANS = 5
const MAX_SOURCES = 12
const RETRY_MS = 30_000
const LOCKED = new Set(['EBUSY', 'EPERM', 'EACCES'])

const defaultWatch = (dir: string, onChange: () => void, onError: () => void): { close(): void } => {
  const w: FSWatcher = fsWatch(dir, { persistent: false }, () => onChange())
  w.on('error', () => onError())
  return w
}

export class OrganizeService {
  private settings: StoredSettings | null = null
  private plans = new Map<string, PlanEntry>()
  private queue: Promise<unknown> = Promise.resolve()
  private watchers = new Map<string, { handle: { close(): void } | null; timer: unknown; retry: unknown }>()
  private disposed = false
  readonly runs: RunStore
  readonly stableMs: number
  private readonly lib: typeof path.win32

  constructor(private readonly deps: OrganizeDeps) {
    this.runs = new RunStore(path.join(deps.dataDir, 'organize'))
    this.stableMs = deps.stableMs ?? ORGANIZE_STABLE_MS
    this.lib = deps.platform === 'win32' ? path.win32 : path.posix
  }

  private now(): number {
    return (this.deps.now ?? Date.now)()
  }

  private get settingsFile(): string {
    return path.join(this.deps.dataDir, 'organize', 'settings.json')
  }

  private serial<T>(fn: () => Promise<T>): Promise<T> {
    const run = this.queue.then(fn, fn)
    this.queue = run.catch(() => undefined)
    return run
  }

  private key(p: string): string {
    return nameKey(normalizePath(p, this.deps.platform), this.deps.platform)
  }

  private same(a: string, b: string): boolean {
    return this.key(a) === this.key(b)
  }

  private ctx(source: string): GuardContext {
    return { platform: this.deps.platform, home: this.deps.home, keep: [], roots: [source], keepTree: this.deps.appDirs.filter(Boolean) }
  }

  async load(): Promise<StoredSettings> {
    if (this.settings) return this.settings
    let raw: Record<string, unknown> = {}
    try {
      raw = JSON.parse(await fs.readFile(this.settingsFile, 'utf8')) as Record<string, unknown>
    } catch {
      raw = {}
    }
    const patch = cleanPatch(raw)
    const sources = Array.isArray(raw.sources) ? raw.sources.filter((s): s is string => typeof s === 'string' && this.lib.isAbsolute(s)).slice(0, MAX_SOURCES) : this.deps.downloads ? [this.deps.downloads] : []
    const since: Record<string, number> = {}
    if (raw.since && typeof raw.since === 'object') for (const [k, v] of Object.entries(raw.since as Record<string, unknown>)) if (typeof v === 'number' && Number.isFinite(v)) since[k] = v
    this.settings = {
      sources,
      source: patch.source && sources.some((s) => this.same(s, patch.source!)) ? patch.source : (sources[0] ?? ''),
      mode: patch.mode ?? 'type',
      kinds: patch.kinds ?? [...ORGANIZE_KINDS],
      rules: patch.rules ?? [],
      auto: patch.auto ?? false,
      since,
    }
    return this.settings
  }

  private async save(): Promise<void> {
    if (!this.settings) return
    await fs.mkdir(path.dirname(this.settingsFile), { recursive: true })
    const tmp = `${this.settingsFile}.${randomBytes(3).toString('hex')}.tmp`
    await fs.writeFile(tmp, JSON.stringify(this.settings, null, 2), 'utf8')
    await fs.rename(tmp, this.settingsFile)
  }

  async checkSource(p: string): Promise<{ path: string } | { reason: string }> {
    const r = await resolveDeletable(p, this.ctx(p), this.deps.guardIo)
    if (r.reason) return { reason: r.reason }
    if (!r.canonical || r.canonical.missing) return { reason: 'missing' }
    if (r.canonical.link) return { reason: 'link' }
    try {
      const st = await fs.stat(r.canonical.path)
      if (!st.isDirectory()) return { reason: 'notdir' }
    } catch {
      return { reason: 'missing' }
    }
    return { path: r.canonical.path }
  }

  private labelOf(p: string): OrganizeSource['label'] {
    if (this.deps.downloads && this.same(p, this.deps.downloads)) return 'downloads'
    if (this.deps.desktop && this.same(p, this.deps.desktop)) return 'desktop'
    return 'custom'
  }

  async state(): Promise<OrganizeState> {
    const s = await this.load()
    const sources: OrganizeSource[] = []
    for (const p of s.sources) {
      const c = await this.checkSource(p)
      sources.push({ path: p, label: this.labelOf(p), ok: 'path' in c, ...('reason' in c ? { reason: c.reason } : {}) })
    }
    const { since: _since, ...settings } = s
    return { settings, sources, runs: await this.runs.list(50), watching: [...this.watchers.entries()].filter(([, w]) => w.handle).map(([p]) => p), stableMs: this.stableMs }
  }

  async patch(v: unknown): Promise<OrganizeState> {
    const s = await this.load()
    const p = cleanPatch(v)
    if (p.source !== undefined) {
      const hit = s.sources.find((x) => this.same(x, p.source!))
      if (hit) s.source = hit
    }
    if (p.mode) s.mode = p.mode
    if (p.kinds) s.kinds = p.kinds
    if (p.rules) s.rules = p.rules
    if (p.auto !== undefined && p.auto !== s.auto) {
      s.auto = p.auto
      if (p.auto) for (const src of s.sources) s.since[this.key(src)] = this.now()
    }
    await this.save()
    this.syncWatchers()
    return this.state()
  }

  async addSource(p: string | null): Promise<{ state: OrganizeState; reason?: string }> {
    const s = await this.load()
    if (!p) return { state: await this.state() }
    const c = await this.checkSource(p)
    if ('reason' in c) return { state: await this.state(), reason: c.reason }
    if (!s.sources.some((x) => this.same(x, c.path))) {
      if (s.sources.length >= MAX_SOURCES) return { state: await this.state(), reason: 'limit' }
      s.sources.push(c.path)
      s.since[this.key(c.path)] = this.now()
    }
    s.source = s.sources.find((x) => this.same(x, c.path)) ?? s.source
    await this.save()
    this.syncWatchers()
    return { state: await this.state() }
  }

  async removeSource(v: unknown): Promise<OrganizeState> {
    const s = await this.load()
    if (typeof v === 'string') {
      s.sources = s.sources.filter((x) => !this.same(x, v))
      delete s.since[this.key(v)]
      if (!s.sources.some((x) => this.same(x, s.source))) s.source = s.sources[0] ?? ''
      await this.save()
      this.syncWatchers()
    }
    return this.state()
  }

  private prune(): void {
    const now = this.now()
    for (const [id, e] of this.plans) if (now - e.at > PLAN_TTL) this.plans.delete(id)
    while (this.plans.size > MAX_PLANS) this.plans.delete(this.plans.keys().next().value!)
  }

  async plan(v: unknown): Promise<OrganizePlan> {
    const s = await this.load()
    const want = typeof (v as { source?: unknown } | null)?.source === 'string' ? (v as { source: string }).source : s.source
    const listed = s.sources.find((x) => this.same(x, want))
    const id = randomBytes(8).toString('hex')
    const empty: OrganizePlan = { id, source: want, mode: s.mode, items: [], skipped: [] }
    if (!listed) return { ...empty, error: 'unknown' }
    const c = await this.checkSource(listed)
    if ('reason' in c) return { ...empty, source: listed, error: c.reason }
    let outcome
    try {
      outcome = await buildPlan(c.path, { mode: s.mode, kinds: s.kinds, rules: s.rules, locale: this.deps.locale(), platform: this.deps.platform, now: this.now(), stableMs: this.stableMs }, this.deps.planIo ?? realPlanIo)
    } catch {
      return { ...empty, source: listed, error: 'unreadable' }
    }
    const moves = new Map<string, PlannedMove>()
    const items = outcome.moves.map((m, i) => {
      const itemId = String(i)
      moves.set(itemId, m)
      return { id: itemId, name: m.name, folder: m.folder, target: m.target, size: m.size, mtime: m.mtime, kind: m.kind, ...(m.rule !== undefined ? { rule: m.rule } : {}) }
    })
    const plan: OrganizePlan = { id, source: c.path, mode: s.mode, items, skipped: outcome.skipped }
    this.plans.set(id, { plan, moves, at: this.now() })
    this.prune()
    return plan
  }

  async run(v: unknown): Promise<OrganizeRunResult> {
    const o = v && typeof v === 'object' ? (v as { plan?: unknown; exclude?: unknown }) : {}
    const fail = (error: string): OrganizeRunResult => ({ run: null, moved: 0, bytes: 0, failed: [], error })
    if (typeof o.plan !== 'string') return fail('unknown')
    const entry = this.plans.get(o.plan)
    if (!entry) return fail('expired')
    this.plans.delete(o.plan)
    const exclude = new Set(Array.isArray(o.exclude) ? o.exclude.filter((x): x is string => typeof x === 'string') : [])
    const moves = [...entry.moves.entries()].filter(([id]) => !exclude.has(id)).map(([, m]) => m)
    if (!moves.length) return fail('empty')
    return this.serial(async () => {
      const c = await this.checkSource(entry.plan.source)
      if ('reason' in c || !this.same(c.path, entry.plan.source)) return fail('reason' in c ? c.reason : 'changed')
      const r = await this.execute(c.path, moves, entry.plan.mode, false)
      return { run: r.run, moved: r.moved, bytes: r.bytes, failed: r.failed }
    })
  }

  private async dirOk(source: string, dir: string): Promise<'ok' | 'missing' | 'blocked'> {
    const io = this.deps.moveIo ?? realMoveIo
    let st: Awaited<ReturnType<MoveIo['lstat']>> & { isDirectory?: () => boolean; isSymbolicLink?: () => boolean }
    try {
      st = (await io.lstat(dir)) as typeof st
    } catch (err) {
      return moveCode(err) === 'ENOENT' ? 'missing' : 'blocked'
    }
    if (st.isSymbolicLink?.() || !st.isDirectory?.()) return 'blocked'
    const can = await canonicalOf(dir, this.deps.platform, this.deps.guardIo)
    if (!can || can.link || can.missing || !this.same(this.lib.dirname(can.path), source)) return 'blocked'
    return 'ok'
  }

  private async fromOk(source: string, from: string): Promise<boolean> {
    const r = await resolveDeletable(from, this.ctx(source), this.deps.guardIo)
    return !r.reason && !!r.canonical && !r.canonical.link && !r.canonical.missing && this.same(this.lib.dirname(r.canonical.path), source)
  }

  private async execute(source: string, moves: PlannedMove[], mode: OrganizeMode, auto: boolean): Promise<{ run: string | null; moved: number; bytes: number; failed: OrganizeSkip[]; retryAt: number | null }> {
    const io = this.deps.moveIo ?? realMoveIo
    const planIo = this.deps.planIo ?? realPlanIo
    const failed: OrganizeSkip[] = []
    let writer: RunWriter | null = null
    let runId: string | null = null
    let moved = 0
    let bytes = 0
    let retryAt: number | null = null
    const taken = new Set<string>()
    const created: string[] = []
    const ensureWriter = (): RunWriter => {
      if (!writer) {
        runId = newRunId(this.now())
        writer = this.runs.writer({ id: runId, at: this.now(), source, mode, auto })
      }
      return writer
    }
    try {
      for (const m of moves) {
        const from = this.lib.join(source, m.name)
        let st: Awaited<ReturnType<PlanIo['lstat']>>
        try {
          st = await planIo.lstat(from)
        } catch {
          failed.push({ name: m.name, reason: 'missing' })
          continue
        }
        if (!st.isFile() || st.size !== m.size || st.mtimeMs !== m.mtime) {
          failed.push({ name: m.name, reason: 'changed' })
          continue
        }
        if (this.now() - touchedAt(st) < this.stableMs) {
          failed.push({ name: m.name, reason: 'recent' })
          continue
        }
        if (!cleanFolder(m.folder) || !(await this.fromOk(source, from))) {
          failed.push({ name: m.name, reason: 'guard' })
          continue
        }
        const dir = this.lib.join(source, m.folder)
        let state = await this.dirOk(source, dir)
        if (state === 'missing') {
          try {
            await fs.mkdir(dir)
            created.push(dir)
            await ensureWriter().write({ t: 'dir', path: dir }, true)
          } catch (err) {
            if (moveCode(err) !== 'EEXIST') {
              failed.push({ name: m.name, reason: 'blocked' })
              continue
            }
          }
          state = await this.dirOk(source, dir)
        }
        if (state !== 'ok') {
          failed.push({ name: m.name, reason: 'blocked' })
          continue
        }
        let target = m.target
        if (taken.has(this.key(this.lib.join(dir, target))) || (await exists(io, this.lib.join(dir, target)))) {
          const next = await freeName(dir, m.name, taken, this.deps.platform, planIo)
          if (!next) {
            failed.push({ name: m.name, reason: 'blocked' })
            continue
          }
          target = next
        }
        taken.add(this.key(this.lib.join(dir, target)))
        const to = this.lib.join(dir, target)
        const w = ensureWriter()
        await w.write({ t: 'intent', from, to }, true)
        try {
          await moveFile(from, to, io)
          const after = await io.lstat(to)
          await w.write({ t: 'move', from, to, size: after.size, mtime: after.mtimeMs })
          moved++
          bytes += after.size
        } catch (err) {
          const code = moveCode(err)
          await w.write({ t: 'fail', to })
          failed.push({ name: m.name, reason: LOCKED.has(code) ? 'locked' : code })
          if (LOCKED.has(code)) retryAt = this.now() + this.stableMs
        }
      }
    } finally {
      for (const d of created) await fs.rmdir(d).catch(() => undefined)
      await (writer as RunWriter | null)?.close()
    }
    if (moved && auto) this.deps.broadcast?.({ run: runId, moved })
    return { run: runId, moved, bytes, failed, retryAt }
  }

  async undo(v: unknown): Promise<OrganizeUndoResult> {
    const id = typeof (v as { run?: unknown } | null)?.run === 'string' ? (v as { run: string }).run : ''
    return this.serial(async () => {
      const rec = await this.runs.read(id)
      if (!rec) return { restored: 0, conflicts: [], dirs: 0, error: 'missing' }
      if (rec.undo) return { restored: 0, conflicts: [], dirs: 0, error: 'done' }
      const source = rec.header.source
      const c = await this.checkSource(source)
      if ('reason' in c || !this.same(c.path, source)) return { restored: 0, conflicts: [], dirs: 0, error: 'reason' in c ? c.reason : 'changed' }
      const io = this.deps.moveIo ?? realMoveIo
      const w = this.runs.appender(rec.header.id)
      const conflicts: OrganizeSkip[] = []
      let restored = 0
      let dirs = 0
      try {
        for (const m of [...rec.moves].reverse()) {
          if (rec.backs.has(m.to)) continue
          const name = this.lib.basename(m.from)
          const toDir = this.lib.dirname(m.to)
          if (!this.same(this.lib.dirname(m.from), source) || !this.same(this.lib.dirname(toDir), source)) {
            conflicts.push({ name, reason: 'guard' })
            continue
          }
          let st: Awaited<ReturnType<MoveIo['lstat']>>
          try {
            st = await io.lstat(m.to)
          } catch {
            if (!m.done) continue
            conflicts.push({ name, reason: 'missing' })
            continue
          }
          if (m.done && (!st.isFile() || st.size !== m.size || Math.abs(st.mtimeMs - (m.mtime ?? -1)) >= 1)) {
            conflicts.push({ name, reason: 'changed' })
            continue
          }
          if (!m.done && !st.isFile()) continue
          if (await exists(io, m.from).catch(() => true)) {
            if (!m.done) continue
            conflicts.push({ name, reason: 'occupied' })
            continue
          }
          if ((await this.dirOk(source, toDir)) !== 'ok') {
            conflicts.push({ name, reason: 'guard' })
            continue
          }
          try {
            await moveFile(m.to, m.from, io)
            await w.write({ t: 'back', from: m.from, to: m.to }, true)
            restored++
          } catch (err) {
            const code = moveCode(err)
            conflicts.push({ name, reason: LOCKED.has(code) ? 'locked' : code })
          }
        }
        for (const d of [...rec.dirs].reverse()) {
          if (!this.same(this.lib.dirname(d), source)) continue
          try {
            if ((await fs.readdir(d)).length) continue
            if ((await this.dirOk(source, d)) !== 'ok') continue
            await fs.rmdir(d)
            dirs++
          } catch {
            continue
          }
        }
        await w.write({ t: 'undo', at: this.now(), restored, conflicts: conflicts.length }, true)
      } finally {
        await w.close()
      }
      return { restored, conflicts, dirs }
    })
  }

  async junk(): Promise<OrganizeJunkResult> {
    if (!this.deps.junk) return { indexed: false, items: [] }
    return scanJunk({ ...this.deps.junk, platform: this.deps.platform, skipTree: this.deps.appDirs.filter(Boolean), allowTree: (this.deps.junkAllow ?? []).filter(Boolean) })
  }

  private timer(fn: () => void, ms: number): unknown {
    return (this.deps.setTimer ?? ((f, t) => setTimeout(f, t).unref()))(fn, Math.max(0, ms))
  }

  private clear(t: unknown): void {
    if (t) (this.deps.clearTimer ?? ((x) => clearTimeout(x as NodeJS.Timeout)))(t)
  }

  start(): Promise<void> {
    return this.load().then(() => this.syncWatchers())
  }

  syncWatchers(): void {
    if (this.disposed || !this.settings) return
    const want = this.settings.auto ? this.settings.sources : []
    for (const [p, w] of this.watchers) {
      if (!want.some((x) => this.same(x, p))) {
        w.handle?.close()
        this.clear(w.timer)
        this.clear(w.retry)
        this.watchers.delete(p)
      }
    }
    for (const p of want) if (![...this.watchers.keys()].some((k) => this.same(k, p))) this.open(p)
  }

  private open(source: string): void {
    const slot = this.watchers.get(source) ?? { handle: null, timer: null, retry: null }
    this.watchers.set(source, slot)
    this.clear(slot.retry)
    slot.retry = null
    const fail = (): void => {
      slot.handle?.close()
      slot.handle = null
      this.clear(slot.retry)
      if (!this.disposed && this.watchers.get(source) === slot) slot.retry = this.timer(() => this.open(source), Math.max(RETRY_MS, this.stableMs))
    }
    try {
      slot.handle = (this.deps.watch ?? defaultWatch)(source, () => this.schedule(source, this.stableMs + 500), fail)
    } catch {
      fail()
      return
    }
    this.schedule(source, 0)
  }

  private schedule(source: string, ms: number): void {
    const slot = this.watchers.get(source)
    if (!slot || this.disposed) return
    this.clear(slot.timer)
    slot.timer = this.timer(() => {
      slot.timer = null
      void this.pass(source)
    }, ms)
  }

  pass(source: string): Promise<number> {
    return this.serial(async () => {
      const s = await this.load()
      if (this.disposed || !s.auto || !s.sources.some((x) => this.same(x, source))) return 0
      const c = await this.checkSource(source)
      if ('reason' in c) return 0
      const k = this.key(source)
      const since = s.since[k] ?? this.now()
      const start = this.now()
      let outcome
      try {
        outcome = await buildPlan(c.path, { mode: s.mode, kinds: s.kinds, rules: s.rules, locale: this.deps.locale(), platform: this.deps.platform, now: start, stableMs: this.stableMs, since }, this.deps.planIo ?? realPlanIo)
      } catch {
        return 0
      }
      const r = outcome.moves.length ? await this.execute(c.path, outcome.moves, s.mode, true) : { moved: 0, failed: [] as OrganizeSkip[], retryAt: null as number | null }
      let next = start - this.stableMs
      if (outcome.pendingMin !== null) next = Math.min(next, outcome.pendingMin - 1)
      for (const m of outcome.moves) if (r.failed.some((f) => f.name === m.name && f.reason !== 'missing')) next = Math.min(next, m.touched - 1)
      if (next > since) {
        s.since[k] = next
        await this.save().catch(() => undefined)
      }
      const wakes = [outcome.waitUntil, r.retryAt].filter((x): x is number => x !== null)
      if (wakes.length) this.schedule(source, Math.min(...wakes) - this.now() + 250)
      return r.moved
    })
  }

  dispose(): void {
    this.disposed = true
    for (const w of this.watchers.values()) {
      w.handle?.close()
      this.clear(w.timer)
      this.clear(w.retry)
    }
    this.watchers.clear()
  }
}
