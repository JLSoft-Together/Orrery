import fs from 'node:fs/promises'
import path from 'node:path'
import type { CleanGroup, CleanItem } from '@ccm/shared'
import { cacheSpots, realDir } from './caches'
import { projectOf, projectTouched, restoreOf, type JunkHit } from './projects'

export interface CleanCtx {
  roots: string[]
  home: string
  platform: NodeJS.Platform
  env: NodeJS.ProcessEnv
  signal: AbortSignal
  junk: () => Promise<JunkHit[]>
  size: (p: string) => Promise<number>
  emit: (items: CleanItem[]) => void
}

export interface CleanProvider {
  id: string
  group: CleanGroup
  preselect: boolean
  platforms?: NodeJS.Platform[]
  scan: (ctx: CleanCtx) => Promise<CleanItem[]>
}

async function fromHits(ctx: CleanCtx, category: string, hits: JunkHit[]): Promise<CleanItem[]> {
  const out: CleanItem[] = []
  for (const h of hits) {
    if (ctx.signal.aborted) break
    const [size, lastTouched, restore, project] = await Promise.all([ctx.size(h.path), projectTouched(h.owner, h.root), restoreOf(h), projectOf(h.owner, h.root)])
    const item: CleanItem = { path: h.path, size, group: 'dev', category, kind: h.kind, project, restore, ...(lastTouched ? { lastTouched } : {}) }
    out.push(item)
    ctx.emit([item])
  }
  return out
}

export const nodeModulesProvider: CleanProvider = {
  id: 'node_modules',
  group: 'dev',
  preselect: true,
  scan: async (ctx) => fromHits(ctx, 'node_modules', (await ctx.junk()).filter((h) => h.kind === 'node' || h.kind === 'pnpm')),
}

export const buildProvider: CleanProvider = {
  id: 'build',
  group: 'dev',
  preselect: true,
  scan: async (ctx) => fromHits(ctx, 'build', (await ctx.junk()).filter((h) => h.kind !== 'node' && h.kind !== 'pnpm')),
}

async function subdirs(dir: string): Promise<Array<{ path: string; mtime: number }>> {
  const names = await fs.readdir(dir).catch(() => [] as string[])
  const out: Array<{ path: string; mtime: number }> = []
  for (const n of names) {
    const p = path.join(dir, n)
    const st = await fs.lstat(p).catch(() => null)
    if (st && st.isDirectory() && !st.isSymbolicLink()) out.push({ path: p, mtime: st.mtimeMs })
  }
  return out
}

export const xcodeProvider: CleanProvider = {
  id: 'xcode',
  group: 'dev',
  preselect: true,
  platforms: ['darwin'],
  scan: async (ctx) => {
    const base = path.join(ctx.home, 'Library', 'Developer', 'Xcode')
    const out: CleanItem[] = []
    const push = async (p: string, mtime: number, kind: string, restore?: string): Promise<void> => {
      if (ctx.signal.aborted) return
      const item: CleanItem = { path: p, size: await ctx.size(p), group: 'dev', category: 'xcode', kind, lastTouched: mtime, ...(restore ? { restore } : {}) }
      out.push(item)
      ctx.emit([item])
    }
    for (const d of await subdirs(path.join(base, 'DerivedData'))) await push(d.path, d.mtime, 'derived', 'xcodebuild')
    for (const day of await subdirs(path.join(base, 'Archives'))) {
      for (const a of await subdirs(day.path)) if (a.path.endsWith('.xcarchive')) await push(a.path, a.mtime, 'archive')
    }
    return out
  },
}

export const cacheProvider: CleanProvider = {
  id: 'cache',
  group: 'dev',
  preselect: false,
  scan: async (ctx) => {
    const out: CleanItem[] = []
    const seen = new Set<string>()
    for (const spot of cacheSpots({ home: ctx.home, platform: ctx.platform, env: ctx.env })) {
      if (ctx.signal.aborted) break
      const real = await realDir(spot.path)
      if (!real) continue
      const key = ctx.platform === 'win32' || ctx.platform === 'darwin' ? real.toLowerCase() : real
      if (seen.has(key)) continue
      seen.add(key)
      const item: CleanItem = { path: real, size: await ctx.size(real), group: 'dev', category: 'cache', kind: spot.kind }
      out.push(item)
      ctx.emit([item])
    }
    return out
  },
}

export const DEV_PROVIDERS: CleanProvider[] = [nodeModulesProvider, buildProvider, xcodeProvider, cacheProvider]
