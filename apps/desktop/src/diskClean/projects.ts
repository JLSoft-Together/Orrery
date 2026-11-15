import fs from 'node:fs/promises'
import path from 'node:path'
import { buildDirKind } from '@ccm/shared'
import { Cancelled } from '../agentData/fsTree'

export type JunkKind = 'node' | 'pnpm' | 'gradle' | 'flutter' | 'rust' | 'maven' | 'web' | 'python'

export interface JunkHit {
  path: string
  name: string
  kind: JunkKind
  owner: string
  root: string
}

export const MARKERS = [
  'package.json',
  'build.gradle',
  'build.gradle.kts',
  'settings.gradle',
  'settings.gradle.kts',
  'gradlew',
  'pubspec.yaml',
  'cargo.toml',
  'pom.xml',
  'pyproject.toml',
  'setup.py',
  'requirements.txt',
]

export const JUNK_NAMES: ReadonlySet<string> = new Set([
  'node_modules',
  '.pnpm-store',
  'build',
  '.gradle',
  '.dart_tool',
  'dist',
  'out',
  'target',
  '.next',
  '.nuxt',
  '.turbo',
  '.parcel-cache',
  '.venv',
])

const LOCKFILES = ['pnpm-lock.yaml', 'yarn.lock', 'package-lock.json', 'bun.lockb', 'bun.lock', 'npm-shrinkwrap.json']

const SKIP = new Set(['.git', '.hg', '.svn', '.idea', '.vscode', '.cxx', '.externalNativeBuild', '__pycache__', '.pytest_cache', 'Pods', '.symlinks'])
export const WALK_DEPTH = 6
const MAX_DIRS = 20_000

const has = (s: ReadonlySet<string>, ...names: string[]): boolean => names.some((n) => s.has(n))

export function junkKind(name: string, siblings: ReadonlySet<string>): JunkKind | null {
  if (!MARKERS.some((m) => siblings.has(m))) return null
  switch (name) {
    case 'node_modules':
      return has(siblings, 'package.json') ? 'node' : null
    case '.pnpm-store':
      return has(siblings, 'package.json', 'pnpm-lock.yaml') ? 'pnpm' : null
    case '.dart_tool':
      return has(siblings, 'pubspec.yaml') ? 'flutter' : null
    case '.next':
    case '.nuxt':
    case '.turbo':
    case '.parcel-cache':
      return has(siblings, 'package.json') ? 'web' : null
    case '.venv':
      return has(siblings, 'pyproject.toml', 'setup.py', 'requirements.txt') ? 'python' : null
    case 'build':
    case '.gradle':
    case 'dist':
    case 'out':
    case 'target': {
      const k = buildDirKind(name, siblings, false)
      return k === 'gradle' || k === 'flutter' || k === 'rust' || k === 'maven' || k === 'web' ? k : null
    }
    default:
      return null
  }
}

export async function findJunk(root: string, signal?: AbortSignal, depthMax = WALK_DEPTH): Promise<JunkHit[]> {
  const out: JunkHit[] = []
  const queue: Array<{ dir: string; depth: number }> = [{ dir: root, depth: 0 }]
  let seen = 0
  while (queue.length && seen < MAX_DIRS) {
    if (signal?.aborted) throw new Cancelled()
    const { dir, depth } = queue.shift()!
    seen++
    const names = await fs.readdir(dir).catch(() => [] as string[])
    const siblings = new Set(names.map((n) => n.toLowerCase()))
    const stats = await Promise.all(names.map((n) => fs.lstat(path.join(dir, n)).catch(() => null)))
    for (let i = 0; i < names.length; i++) {
      const name = names[i]!
      const st = stats[i]
      if (!st || st.isSymbolicLink() || !st.isDirectory()) continue
      const child = path.join(dir, name)
      if (JUNK_NAMES.has(name)) {
        const kind = junkKind(name, siblings)
        if (kind) out.push({ path: child, name, kind, owner: dir, root })
        continue
      }
      if (SKIP.has(name) || depth + 1 >= depthMax) continue
      queue.push({ dir: child, depth: depth + 1 })
    }
  }
  return out
}

async function mtime(p: string): Promise<number> {
  const st = await fs.stat(p).catch(() => null)
  return st ? st.mtimeMs : 0
}

export async function repoOf(owner: string, root: string): Promise<{ dir: string; git: string; isDir: boolean } | null> {
  let dir = owner
  const top = path.resolve(root)
  for (let i = 0; i < 12; i++) {
    const git = path.join(dir, '.git')
    const st = await fs.stat(git).catch(() => null)
    if (st) return { dir, git, isDir: st.isDirectory() }
    if (path.resolve(dir) === top) break
    const up = path.dirname(dir)
    if (up === dir) break
    dir = up
  }
  return null
}

export async function projectOf(owner: string, root: string): Promise<string> {
  return (await repoOf(owner, root))?.dir ?? owner
}

export async function projectTouched(owner: string, root: string): Promise<number | undefined> {
  const names = await fs.readdir(owner).catch(() => [] as string[])
  const marks = names.filter((n) => MARKERS.includes(n.toLowerCase()) || LOCKFILES.includes(n.toLowerCase()))
  let best = Math.max(0, ...(await Promise.all(marks.map((n) => mtime(path.join(owner, n))))))
  const repo = await repoOf(owner, root)
  if (repo?.isDir) best = Math.max(best, await mtime(path.join(repo.git, 'HEAD')), await mtime(path.join(repo.git, 'index')))
  else if (repo) best = Math.max(best, await mtime(repo.git))
  return best > 0 ? best : undefined
}

export async function nodeManager(owner: string, root: string): Promise<'pnpm' | 'yarn' | 'bun' | 'npm'> {
  let dir = owner
  const top = path.resolve(root)
  for (let i = 0; i < 12; i++) {
    const names = new Set((await fs.readdir(dir).catch(() => [] as string[])).map((n) => n.toLowerCase()))
    if (names.has('pnpm-lock.yaml')) return 'pnpm'
    if (names.has('yarn.lock')) return 'yarn'
    if (names.has('bun.lockb') || names.has('bun.lock')) return 'bun'
    if (names.has('package-lock.json') || names.has('npm-shrinkwrap.json')) return 'npm'
    if (path.resolve(dir) === top) break
    const up = path.dirname(dir)
    if (up === dir) break
    dir = up
  }
  return 'npm'
}

export async function restoreOf(hit: Pick<JunkHit, 'kind' | 'owner' | 'root'>): Promise<string> {
  switch (hit.kind) {
    case 'node': {
      const m = await nodeManager(hit.owner, hit.root)
      return m === 'yarn' ? 'yarn' : `${m} install`
    }
    case 'pnpm':
      return 'pnpm install'
    case 'web': {
      const m = await nodeManager(hit.owner, hit.root)
      return m === 'npm' ? 'npm run build' : `${m} run build`
    }
    case 'gradle':
      return './gradlew build'
    case 'flutter':
      return 'flutter pub get'
    case 'rust':
      return 'cargo build'
    case 'maven':
      return 'mvn package'
    case 'python':
      return 'python -m venv .venv'
  }
}
