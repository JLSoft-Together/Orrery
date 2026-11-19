import { randomUUID } from 'node:crypto'
import fs from 'node:fs/promises'
import path from 'node:path'
import {
  MOVE_BACK_SUFFIX,
  MOVE_DIR,
  MOVE_SUFFIX,
  moveNeed,
  type DiskHistoryEntry,
  type DiskMoveRecord,
  type MoveBlock,
  type MoveCandidate,
  type MoveKind,
  type MovePending,
  type MovePhase,
  type MovePlan,
  type MovePlanResult,
  type MoveProgress,
  type MoveResult,
  type MoveState,
  type MoveVolume,
} from '@ccm/shared'
import { isUnder, normalizePath, resolveDeletable } from '../diskMap/guard'
import type { ProcInfo } from '../diskClean/sdk'
import { isOpen, MoveJournal, pendingOf, recordOf, type JournalEntry, type MoveEntry } from './journal'
import { codeOf, copyTree, MoveCancelled, removeTree, retry, verifyCopy, walkTree, type Fault } from './tree'

export interface MoveAsk {
  kind: 'move' | 'back' | 'permanent' | 'restore' | 'finish'
  source: string
  target: string
  bytes: number
}

export interface VolumeInfo {
  mount: string
  label: string
  network: boolean
  total: number
  free: number
}

export interface Suggestion {
  kind: MoveKind
  path: string
  guide?: boolean
}

export interface DiskMoveDeps {
  platform: NodeJS.Platform
  home: string
  dataDir: string
  keep: () => string[]
  self: () => string[]
  volumes: () => Promise<VolumeInfo[]>
  suggest: () => Suggestion[]
  holders: () => Promise<ProcInfo[]>
  confirm: (ask: MoveAsk) => Promise<boolean>
  trash: (p: string) => Promise<void>
  record: (e: DiskHistoryEntry) => Promise<void>
  emit: (p: MoveProgress) => void
  indexSize?: (p: string) => Promise<number | null>
  cleanKnown?: (p: string) => string | null
  pick?: () => Promise<string | null>
  freeOf?: (p: string) => Promise<number | null>
  devOf?: (p: string) => Promise<number | null>
  fault?: Fault
  now?: () => number
}

export const TOOL = /^(java|javaw|gradle|gradlew|adb|emulator|qemu-system[\w-]*|docker|com\.docker\.[\w.-]+|dockerd|node|dart|dartaotruntime|flutter|studio|studio64|idea|idea64|kotlin-daemon|pub)(\.exe)?$/i

export function holderOf(source: string, procs: readonly ProcInfo[], platform: NodeJS.Platform): string | null {
  const fold = (s: string): string => {
    const n = s.replace(/\\/g, '/').replace(/\/+$/, '')
    return platform === 'linux' ? n : n.toLowerCase()
  }
  const needle = fold(source)
  for (const p of procs) {
    if (!TOOL.test(p.name)) continue
    const cmd = fold(p.cmd)
    const at = cmd.indexOf(needle)
    if (at < 0) continue
    const next = cmd.charAt(at + needle.length)
    if (!next || /[/\s"';:]/.test(next)) return p.name
  }
  return null
}

const fail = (code: string): Error => Object.assign(new Error(code), { code })

interface Candidate extends MoveCandidate {
  real: string
}

interface Plan extends MovePlan {
  at: number
}

const statfsFree = async (p: string): Promise<number | null> => {
  try {
    const s = await fs.statfs(p)
    return Number(s.bavail) * Number(s.bsize)
  } catch {
    return null
  }
}

const devOfDefault = async (p: string): Promise<number | null> => {
  try {
    return (await fs.stat(p)).dev
  } catch {
    return null
  }
}

export class DiskMoveService {
  readonly journal: MoveJournal
  private readonly candidates = new Map<string, Candidate>()
  private readonly plans = new Map<string, Plan>()
  private abort: AbortController | null = null
  private running: string | null = null

  constructor(private readonly deps: DiskMoveDeps) {
    this.journal = new MoveJournal(path.join(deps.dataDir, 'disk', 'moves.jsonl'))
  }

  private get lib(): typeof path.win32 {
    return this.deps.platform === 'win32' ? path.win32 : path.posix
  }

  private now(): number {
    return (this.deps.now ?? Date.now)()
  }

  private key(p: string): string {
    const n = normalizePath(p, this.deps.platform)
    return this.deps.platform === 'linux' ? n : n.toLowerCase()
  }

  private emit(id: string, phase: MovePhase, bytes = 0, total = 0, files = 0): void {
    this.deps.emit({ id, phase, bytes, total, files })
  }

  private async block(p: string): Promise<{ block?: MoveBlock; holder?: string }> {
    const st = await fs.lstat(p).catch(() => null)
    if (!st) return { block: 'missing' }
    if (st.isSymbolicLink()) {
      const moved = (await this.journal.entries()).some((e) => e.kind === 'move' && (e.stage === 'done' || e.stage === 'cleaned') && this.key(e.source) === this.key(p))
      return { block: moved ? 'moved' : 'link' }
    }
    if (!st.isDirectory()) return { block: 'notDir' }
    const pf = this.deps.platform
    const { reason } = await resolveDeletable(p, { platform: pf, home: this.deps.home, keep: this.deps.keep(), roots: [p] })
    if (reason) return { block: 'guard' }
    if (this.deps.self().some((s) => s && (isUnder(p, s, pf) || isUnder(s, p, pf))) || isUnder(p, this.deps.dataDir, pf) || isUnder(this.deps.dataDir, p, pf)) return { block: 'self' }
    if (await fs.lstat(p + MOVE_SUFFIX).catch(() => null)) return { block: 'pending' }
    if (await fs.lstat(p + MOVE_BACK_SUFFIX).catch(() => null)) return { block: 'pending' }
    return {}
  }

  private async addCandidate(kind: MoveKind, p: string, guide = false): Promise<Candidate> {
    const abs = normalizePath(p, this.deps.platform)
    const existing = [...this.candidates.values()].find((c) => this.key(c.path) === this.key(abs))
    const id = existing?.id ?? randomUUID()
    const b = guide ? {} : await this.block(abs)
    const size = b.block ? undefined : ((await this.deps.indexSize?.(abs).catch(() => null)) ?? undefined)
    const c: Candidate = { id, path: abs, kind: existing && existing.kind !== 'picked' ? existing.kind : kind, real: abs, ...(guide ? { guide: true } : {}), ...b, ...(typeof size === 'number' ? { size } : {}) }
    this.candidates.set(id, c)
    return c
  }

  private view(c: Candidate): MoveCandidate {
    const { real: _real, ...rest } = c
    return rest
  }

  async list(): Promise<MoveCandidate[]> {
    for (const s of this.deps.suggest()) if (s.path) await this.addCandidate(s.kind, s.path, s.guide === true)
    for (const c of [...this.candidates.values()]) if (c.kind === 'picked' || c.kind === 'clean') await this.addCandidate(c.kind, c.path)
    return [...this.candidates.values()].map((c) => this.view(c))
  }

  async pick(): Promise<MoveCandidate | null> {
    const p = await this.deps.pick?.()
    if (!p || !this.lib.isAbsolute(p)) return null
    return this.view(await this.addCandidate('picked', p))
  }

  async fromClean(p: string, open = false): Promise<MoveCandidate | null> {
    const known = open ? (this.lib.isAbsolute(p) ? p : null) : this.deps.cleanKnown?.(p)
    if (!known) return null
    return this.view(await this.addCandidate('clean', known))
  }

  async volumes(candidateId: string): Promise<MoveVolume[]> {
    const c = this.candidates.get(candidateId)
    if (!c) return []
    const srcDev = await (this.deps.devOf ?? devOfDefault)(this.lib.dirname(c.path))
    const out: MoveVolume[] = []
    for (const v of await this.deps.volumes().catch(() => [] as VolumeInfo[])) {
      if (!v.mount) continue
      const dev = await (this.deps.devOf ?? devOfDefault)(v.mount)
      const free = (await (this.deps.freeOf ?? statfsFree)(v.mount)) ?? v.free
      const same = dev !== null && dev === srcDev
      const inside = isUnder(v.mount, c.path, this.deps.platform)
      const reason = dev === null ? 'missing' : v.network ? 'network' : same || inside ? 'same' : typeof c.size === 'number' && free < moveNeed(c.size) ? 'space' : undefined
      out.push({ mount: v.mount, label: v.label, free, total: v.total, ok: !reason, ...(reason ? { reason } : {}) })
    }
    return out
  }

  cancel(): boolean {
    if (!this.abort) return false
    this.abort.abort()
    return true
  }

  private async targetFor(source: string, mount: string): Promise<string> {
    const base = this.lib.join(mount, MOVE_DIR)
    const name = this.lib.basename(source)
    for (let i = 1; i < 100; i++) {
      const t = this.lib.join(base, i === 1 ? name : `${name}-${i}`)
      if (!(await fs.lstat(t).catch(() => null))) return t
    }
    throw fail('target')
  }

  async lockOf(source: string): Promise<string | null> {
    const procs = await this.deps.holders().catch(() => [] as ProcInfo[])
    const h = holderOf(source, procs, this.deps.platform)
    if (h) return h
    const probe = source + '.orrery-probe'
    try {
      await fs.rename(source, probe)
    } catch (err) {
      const c = codeOf(err)
      return c === 'EBUSY' || c === 'EPERM' || c === 'EACCES' ? 'probe' : null
    }
    await retry(() => fs.rename(probe, source))
    return null
  }

  async plan(candidateId: string, mount: string): Promise<MovePlanResult> {
    if (this.running || this.abort) return { error: 'busy' }
    const c = this.candidates.get(candidateId)
    if (!c || c.guide) return { error: 'invalid' }
    const vol = (await this.volumes(candidateId)).find((v) => this.key(v.mount) === this.key(mount))
    if (!vol) return { error: 'invalid' }
    if (vol.reason && vol.reason !== 'space') return { error: vol.reason }
    const b = await this.block(c.path)
    if (b.block) return { error: b.block }
    const holder = await this.lockOf(c.path)
    if (holder) return { error: 'locked', holder }
    const abort = new AbortController()
    this.abort = abort
    try {
      this.emit(candidateId, 'check')
      const st = await walkTree(c.path, { signal: abort.signal })
      c.size = st.bytes
      const free = (await (this.deps.freeOf ?? statfsFree)(vol.mount)) ?? vol.free
      const need = moveNeed(st.bytes)
      if (free < need) return { error: 'space' }
      const target = await this.targetFor(c.path, vol.mount)
      if (isUnder(target, c.path, this.deps.platform) || isUnder(c.path, target, this.deps.platform)) return { error: 'same' }
      const plan: Plan = { id: randomUUID(), candidate: c.id, source: c.path, target, bytes: st.bytes, files: st.files, volume: vol.mount, need, free, at: this.now() }
      this.plans.set(plan.id, plan)
      const { at: _at, ...view } = plan
      return { plan: view }
    } catch (err) {
      return { error: err instanceof MoveCancelled ? 'cancelled' : codeOf(err) }
    } finally {
      this.abort = null
    }
  }

  private async log(e: JournalEntry): Promise<void> {
    await this.journal.append(e)
  }

  private async dropOld(source: string, old: string, bytes: number): Promise<string | undefined> {
    const { reason } = await resolveDeletable(old, { platform: this.deps.platform, home: this.deps.home, keep: this.deps.keep(), roots: [old] })
    if (reason) return old
    await this.deps.fault?.('cleanup', old)
    try {
      await this.deps.trash(old)
      await this.deps.record({ at: this.now(), action: 'trash', count: 1, freed: bytes, paths: [old], failed: 0 }).catch(() => undefined)
      return undefined
    } catch {
      if (!(await this.deps.confirm({ kind: 'permanent', source, target: old, bytes }))) return old
      try {
        await removeTree(old)
        await this.deps.record({ at: this.now(), action: 'delete', count: 1, freed: bytes, paths: [old], failed: 0 }).catch(() => undefined)
        return undefined
      } catch {
        return old
      }
    }
  }

  async run(planId: string): Promise<MoveResult> {
    if (this.running || this.abort) return { ok: false, error: 'busy' }
    const plan = this.plans.get(planId)
    if (!plan) return { ok: false, error: 'invalid' }
    this.plans.delete(planId)
    const { source, target } = plan
    const b = await this.block(source)
    if (b.block) return { ok: false, error: b.block }
    if (!(await this.deps.confirm({ kind: 'move', source, target, bytes: plan.bytes }))) return { ok: false, cancelled: true }
    const holder = await this.lockOf(source)
    if (holder) return { ok: false, error: 'locked', holder }
    const free = (await (this.deps.freeOf ?? statfsFree)(plan.volume)) ?? 0
    if (free < moveNeed(plan.bytes)) return { ok: false, error: 'space' }
    if (await fs.lstat(target).catch(() => null)) return { ok: false, error: 'target' }
    const id = randomUUID()
    const moving = source + MOVE_SUFFIX
    const abort = new AbortController()
    this.abort = abort
    this.running = id
    let stage: 'copy' | 'renamed' | 'linked' = 'copy'
    let made = false
    try {
      await this.log({ id, stage: 'copying', at: this.now(), kind: 'move', source, target, bytes: plan.bytes, files: plan.files })
      await fs.mkdir(this.lib.dirname(target), { recursive: true })
      let last = 0
      made = true
      await copyTree(source, target, {
        platform: this.deps.platform,
        signal: abort.signal,
        fault: this.deps.fault,
        onBytes: (bytes, files) => {
          const t = Date.now()
          if (t - last < 200) return
          last = t
          this.emit(id, 'copy', bytes, plan.bytes, files)
        },
      })
      this.emit(id, 'verify', plan.bytes, plan.bytes, plan.files)
      const v = await verifyCopy(source, target, abort.signal, this.deps.fault)
      if (!v.ok) throw fail(`verify:${v.reason}`)
      this.emit(id, 'switch', plan.bytes, plan.bytes, plan.files)
      await this.deps.fault?.('rename', source)
      try {
        await fs.rename(source, moving)
      } catch (err) {
        const c = codeOf(err)
        throw fail(c === 'EBUSY' || c === 'EPERM' || c === 'EACCES' ? 'locked' : c)
      }
      stage = 'renamed'
      await this.log({ id, stage: 'renamed', at: this.now() })
      await this.deps.fault?.('symlink', source)
      await fs.symlink(target, source, this.deps.platform === 'win32' ? 'junction' : 'dir')
      stage = 'linked'
      await this.deps.fault?.('linkCheck', source)
      const real = await fs.realpath(source)
      const realTarget = await fs.realpath(target)
      if (this.key(real) !== this.key(realTarget)) throw fail('linkCheck')
      await fs.readdir(source)
      await this.log({ id, stage: 'linked', at: this.now() })
      this.emit(id, 'cleanup', plan.bytes, plan.bytes, plan.files)
      const leftover = await this.dropOld(source, moving, plan.bytes)
      await this.log({ id, stage: 'done', at: this.now(), ...(leftover ? { leftover } : {}) })
      this.emit(id, 'done', plan.bytes, plan.bytes, plan.files)
      await this.deps.record({ at: this.now(), action: 'move', count: 1, freed: plan.bytes, paths: [source, target], failed: 0 }).catch(() => undefined)
      const record: DiskMoveRecord = { id, source, target, at: this.now(), bytes: plan.bytes, files: plan.files, state: 'moved', ...(leftover ? { leftover } : {}) }
      return { ok: true, record, ...(leftover ? { leftover } : {}) }
    } catch (err) {
      const error = err instanceof MoveCancelled ? 'cancelled' : codeOf(err)
      this.emit(id, 'rollback')
      const back = await this.rollbackMove(stage, source, moving, target, made)
      await this.log({ id, stage: back ? 'rolledBack' : 'failed', at: this.now(), error })
      return { ok: false, error, rolledBack: back, ...(error === 'cancelled' ? { cancelled: true } : {}) }
    } finally {
      this.abort = null
      this.running = null
    }
  }

  private async rollbackMove(stage: 'copy' | 'renamed' | 'linked', source: string, moving: string, target: string, made: boolean): Promise<boolean> {
    try {
      if (stage === 'linked' || stage === 'renamed') {
        if (!(await fs.lstat(moving).catch(() => null))) return false
        const st = await fs.lstat(source).catch(() => null)
        if (st?.isSymbolicLink()) await retry(() => fs.unlink(source))
        else if (st) return false
        await retry(() => fs.rename(moving, source))
      }
      if (made) await removeTree(target)
      return true
    } catch {
      return false
    }
  }

  private async findMove(id: string): Promise<MoveEntry | null> {
    return (await this.journal.entries()).find((e) => e.id === id && e.kind === 'move' && (e.stage === 'done' || e.stage === 'cleaned')) ?? null
  }

  async back(moveId: string): Promise<MoveResult> {
    if (this.running || this.abort) return { ok: false, error: 'busy' }
    const m = await this.findMove(moveId)
    if (!m) return { ok: false, error: 'invalid' }
    const { source, target } = m
    if (m.leftover && (await fs.lstat(m.leftover).catch(() => null))) return { ok: false, error: 'leftover' }
    const st = await fs.lstat(source).catch(() => null)
    if (!st?.isSymbolicLink()) return { ok: false, error: 'notLink' }
    const tst = await fs.lstat(target).catch(() => null)
    if (!tst?.isDirectory()) return { ok: false, error: 'targetMissing' }
    const real = await fs.realpath(source).catch(() => null)
    if (!real || this.key(real) !== this.key(await fs.realpath(target))) return { ok: false, error: 'notLink' }
    const tmp = source + MOVE_BACK_SUFFIX
    if (await fs.lstat(tmp).catch(() => null)) return { ok: false, error: 'pending' }
    const holder = (await this.lockOf(target)) ?? holderOf(source, await this.deps.holders().catch(() => []), this.deps.platform)
    if (holder) return { ok: false, error: 'locked', holder }
    const abort = new AbortController()
    this.abort = abort
    let size: { bytes: number; files: number }
    try {
      size = await walkTree(target, { signal: abort.signal })
    } catch (err) {
      this.abort = null
      return { ok: false, error: err instanceof MoveCancelled ? 'cancelled' : codeOf(err) }
    }
    this.abort = null
    const free = (await (this.deps.freeOf ?? statfsFree)(this.lib.dirname(source))) ?? 0
    if (free < moveNeed(size.bytes)) return { ok: false, error: 'space' }
    if (!(await this.deps.confirm({ kind: 'back', source, target, bytes: size.bytes }))) return { ok: false, cancelled: true }
    const id = randomUUID()
    this.abort = new AbortController()
    this.running = id
    let unlinked = false
    try {
      await this.log({ id, stage: 'copying', at: this.now(), kind: 'back', source, target, bytes: size.bytes, files: size.files })
      let last = 0
      await copyTree(target, tmp, {
        platform: this.deps.platform,
        signal: this.abort.signal,
        fault: this.deps.fault,
        onBytes: (bytes, files) => {
          const t = Date.now()
          if (t - last < 200) return
          last = t
          this.emit(id, 'copy', bytes, size.bytes, files)
        },
      })
      this.emit(id, 'verify', size.bytes, size.bytes, size.files)
      const v = await verifyCopy(target, tmp, this.abort.signal, this.deps.fault)
      if (!v.ok) throw fail(`verify:${v.reason}`)
      this.emit(id, 'switch', size.bytes, size.bytes, size.files)
      await this.deps.fault?.('unlink', source)
      await retry(() => fs.unlink(source))
      unlinked = true
      await this.log({ id, stage: 'renamed', at: this.now() })
      await this.deps.fault?.('rename', tmp)
      await retry(() => fs.rename(tmp, source))
      unlinked = false
      await this.log({ id: m.id, stage: 'returned', at: this.now() })
      this.emit(id, 'cleanup', size.bytes, size.bytes, size.files)
      let leftover: string | undefined
      try {
        await this.deps.fault?.('cleanup', target)
        await removeTree(target)
        await this.deps.record({ at: this.now(), action: 'delete', count: 1, freed: size.bytes, paths: [target], failed: 0 }).catch(() => undefined)
      } catch {
        leftover = target
      }
      await this.log({ id, stage: 'done', at: this.now(), ...(leftover ? { leftover } : {}) })
      this.emit(id, 'done', size.bytes, size.bytes, size.files)
      return { ok: true, record: { id, source, target, at: this.now(), bytes: size.bytes, files: size.files, state: 'back', ...(leftover ? { leftover } : {}) }, ...(leftover ? { leftover } : {}) }
    } catch (err) {
      const error = err instanceof MoveCancelled ? 'cancelled' : codeOf(err)
      this.emit(id, 'rollback')
      let ok = true
      try {
        if (unlinked && !(await fs.lstat(source).catch(() => null))) await fs.symlink(target, source, this.deps.platform === 'win32' ? 'junction' : 'dir')
        await removeTree(tmp)
      } catch {
        ok = false
      }
      await this.log({ id, stage: ok ? 'rolledBack' : 'failed', at: this.now(), error })
      return { ok: false, error, rolledBack: ok, ...(error === 'cancelled' ? { cancelled: true } : {}) }
    } finally {
      this.abort = null
      this.running = null
    }
  }

  private async pendingStage(e: MoveEntry): Promise<MovePending | null> {
    const lst = async (p: string) => fs.lstat(p).catch(() => null)
    const src = await lst(e.source)
    if (e.kind === 'move') {
      const moving = await lst(e.source + MOVE_SUFFIX)
      if (moving && src?.isSymbolicLink()) return pendingOf(e, 'linked')
      if (moving && !src) return pendingOf(e, 'renamed')
      if (!moving && src?.isDirectory() && !src.isSymbolicLink() && (await lst(e.target)) && e.stage !== 'done' && e.stage !== 'cleaned') return pendingOf(e, 'copying')
      return null
    }
    const tmp = await lst(e.source + MOVE_BACK_SUFFIX)
    if (tmp && src?.isSymbolicLink()) return pendingOf(e, 'copying')
    if (tmp && !src) return pendingOf(e, 'renamed')
    if (!tmp && e.leftover && (await lst(e.leftover)) && src?.isDirectory() && !src.isSymbolicLink()) return pendingOf(e, 'linked')
    return null
  }

  async state(): Promise<MoveState> {
    const entries = await this.journal.entries()
    const moves: DiskMoveRecord[] = []
    const pending: MovePending[] = []
    for (const e of entries) {
      if (e.id === this.running) continue
      if (e.kind === 'move' && (e.stage === 'done' || e.stage === 'cleaned') && (await fs.lstat(e.source).catch(() => null))?.isSymbolicLink()) {
        const r = recordOf(e)
        moves.push(r.leftover && !(await fs.lstat(r.leftover).catch(() => null)) ? { ...r, leftover: undefined } : r)
      }
      if (isOpen(e)) {
        const p = await this.pendingStage(e)
        if (p) pending.push(p)
      }
    }
    return { moves: moves.sort((a, b) => b.at - a.at), pending, running: this.running }
  }

  async recover(pendingId: string, action: 'restore' | 'finish'): Promise<MoveResult> {
    if (this.running || this.abort) return { ok: false, error: 'busy' }
    const e = (await this.journal.entries()).find((x) => x.id === pendingId)
    const p = e && isOpen(e) ? await this.pendingStage(e) : null
    if (!e || !p) return { ok: false, error: 'invalid' }
    if (!(await this.deps.confirm({ kind: action, source: e.source, target: e.target, bytes: e.bytes }))) return { ok: false, cancelled: true }
    this.running = e.id
    try {
      const moving = e.source + MOVE_SUFFIX
      const tmp = e.source + MOVE_BACK_SUFFIX
      if (e.kind === 'move') {
        if (action === 'finish') {
          if (p.stage !== 'linked') return { ok: false, error: 'invalid' }
          const leftover = await this.dropOld(e.source, moving, e.bytes)
          if (leftover) return { ok: false, error: 'leftover', leftover }
          await this.log({ id: e.id, stage: 'cleaned', at: this.now(), leftover: '' })
          return { ok: true }
        }
        const ok = await this.rollbackMove(p.stage === 'copying' ? 'copy' : 'linked', e.source, moving, e.target, true)
        await this.log({ id: e.id, stage: ok ? 'rolledBack' : 'failed', at: this.now() })
        return ok ? { ok: true, rolledBack: true } : { ok: false, error: 'restore' }
      }
      if (p.stage === 'linked') {
        if (action !== 'finish') return { ok: false, error: 'invalid' }
        await removeTree(e.target)
        await this.log({ id: e.id, stage: 'cleaned', at: this.now(), leftover: '' })
        return { ok: true }
      }
      if (p.stage === 'renamed' && action === 'finish') {
        await retry(() => fs.rename(tmp, e.source))
        await this.log({ id: e.id, stage: 'done', at: this.now(), leftover: e.target })
        return { ok: true }
      }
      if (p.stage === 'renamed') await fs.symlink(e.target, e.source, this.deps.platform === 'win32' ? 'junction' : 'dir')
      await removeTree(tmp)
      await this.log({ id: e.id, stage: 'rolledBack', at: this.now() })
      return { ok: true, rolledBack: true }
    } catch (err) {
      return { ok: false, error: codeOf(err) }
    } finally {
      this.running = null
    }
  }
}
