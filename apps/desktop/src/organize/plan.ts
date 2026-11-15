import { promises as fs } from 'node:fs'
import path from 'node:path'
import type { OrganizeKind, OrganizeMode, OrganizeRule, OrganizeSkip } from '@ccm/shared'
import { classify, isHiddenName, isPartialName, PARTIAL_EXT, withSuffix } from './rules'

export interface PlanIo {
  readdir: (dir: string) => Promise<Array<{ name: string; isFile(): boolean; isDirectory(): boolean; isSymbolicLink(): boolean }>>
  lstat: (p: string) => Promise<{ size: number; mtimeMs: number; ctimeMs: number; birthtimeMs: number; isFile(): boolean; isDirectory(): boolean }>
}

export const realPlanIo: PlanIo = {
  readdir: (dir) => fs.readdir(dir, { withFileTypes: true }),
  lstat: (p) => fs.lstat(p),
}

export interface PlanOptions {
  mode: OrganizeMode
  kinds: OrganizeKind[]
  rules: OrganizeRule[]
  locale: string
  platform: NodeJS.Platform
  now: number
  stableMs: number
  since?: number
  max?: number
}

export interface PlannedMove {
  name: string
  folder: string
  target: string
  size: number
  mtime: number
  kind: OrganizeKind
  touched: number
  rule?: number
}

export interface PlanOutcome {
  moves: PlannedMove[]
  skipped: OrganizeSkip[]
  waitUntil: number | null
  pendingMin: number | null
}

export const touchedAt = (st: { mtimeMs: number; ctimeMs: number; birthtimeMs: number }): number => Math.max(st.mtimeMs, st.ctimeMs || 0, st.birthtimeMs || 0)

export const nameKey = (name: string, platform: NodeJS.Platform): string => {
  const n = name.normalize('NFC')
  return platform === 'linux' ? n : n.toLowerCase()
}

export async function freeName(dir: string, name: string, taken: Set<string>, platform: NodeJS.Platform, io: Pick<PlanIo, 'lstat'>): Promise<string | null> {
  for (let n = 1; n < 10_000; n++) {
    const candidate = withSuffix(name, n)
    const key = nameKey(path.join(dir, candidate), platform)
    if (taken.has(key)) continue
    const used = await io.lstat(path.join(dir, candidate)).then(
      () => true,
      (err: unknown) => (err as { code?: unknown } | null)?.code !== 'ENOENT',
    )
    if (!used) {
      taken.add(key)
      return candidate
    }
  }
  return null
}

export async function buildPlan(source: string, opts: PlanOptions, io: PlanIo = realPlanIo): Promise<PlanOutcome> {
  const out: PlanOutcome = { moves: [], skipped: [], waitUntil: null, pendingMin: null }
  const entries = await io.readdir(source)
  const names = new Set(entries.map((e) => nameKey(e.name, opts.platform)))
  const taken = new Set<string>()
  const folderState = new Map<string, boolean>()
  const pending = (t: number): void => {
    out.pendingMin = out.pendingMin === null ? t : Math.min(out.pendingMin, t)
  }
  const max = opts.max ?? 5000
  for (const e of entries) {
    if (out.moves.length >= max) {
      out.skipped.push({ name: e.name, reason: 'limit' })
      continue
    }
    if (e.isDirectory()) continue
    if (e.isSymbolicLink()) {
      out.skipped.push({ name: e.name, reason: 'link' })
      continue
    }
    if (!e.isFile()) continue
    if (isHiddenName(e.name)) {
      out.skipped.push({ name: e.name, reason: 'hidden' })
      continue
    }
    const full = path.join(source, e.name)
    let st: Awaited<ReturnType<PlanIo['lstat']>>
    try {
      st = await io.lstat(full)
    } catch {
      out.skipped.push({ name: e.name, reason: 'unreadable' })
      continue
    }
    if (!st.isFile()) continue
    const t = touchedAt(st)
    if (opts.since !== undefined && t <= opts.since) continue
    if (isPartialName(e.name) || PARTIAL_EXT.some((x) => names.has(nameKey(`${e.name}.${x}`, opts.platform)))) {
      out.skipped.push({ name: e.name, reason: 'downloading' })
      pending(t)
      continue
    }
    if (opts.now - t < opts.stableMs) {
      out.skipped.push({ name: e.name, reason: 'recent' })
      const ready = t + opts.stableMs
      out.waitUntil = out.waitUntil === null ? ready : Math.min(out.waitUntil, ready)
      pending(t)
      continue
    }
    const c = classify(e.name, st.size, st.mtimeMs, opts)
    if (!c) {
      out.skipped.push({ name: e.name, reason: 'kind' })
      continue
    }
    const dir = path.join(source, c.folder)
    if (!folderState.has(c.folder)) {
      const ok = await io.lstat(dir).then(
        (s) => s.isDirectory(),
        (err: unknown) => (err as { code?: unknown } | null)?.code === 'ENOENT',
      )
      folderState.set(c.folder, ok)
    }
    if (!folderState.get(c.folder)) {
      out.skipped.push({ name: e.name, reason: 'blocked' })
      continue
    }
    const target = await freeName(dir, e.name, taken, opts.platform, io)
    if (!target) {
      out.skipped.push({ name: e.name, reason: 'blocked' })
      continue
    }
    out.moves.push({ name: e.name, folder: c.folder, target, size: st.size, mtime: st.mtimeMs, kind: c.kind, touched: t, ...(c.rule !== undefined ? { rule: c.rule } : {}) })
  }
  return out
}
