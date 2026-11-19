import { createHash } from 'node:crypto'
import { createReadStream, constants as FS, type Dirent } from 'node:fs'
import fs from 'node:fs/promises'
import path from 'node:path'

export type Fault = (step: string, p?: string) => void | Promise<void>

export interface TreeStat {
  files: number
  bytes: number
  dirs: number
  links: number
}

export class MoveCancelled extends Error {
  constructor() {
    super('cancelled')
  }
}

const fail = (code: string, p?: string): Error => Object.assign(new Error(p ? `${code}: ${p}` : code), { code })

export const codeOf = (err: unknown): string => {
  const c = (err as { code?: unknown } | null)?.code
  return typeof c === 'string' && c ? c : 'failed'
}

export interface WalkOpts {
  signal?: AbortSignal
  onFile?: (p: string, size: number) => void
}

export async function walkTree(root: string, opts: WalkOpts = {}): Promise<TreeStat> {
  const stat: TreeStat = { files: 0, bytes: 0, dirs: 0, links: 0 }
  const stack = [root]
  while (stack.length) {
    if (opts.signal?.aborted) throw new MoveCancelled()
    const dir = stack.pop()!
    let entries: Dirent[]
    try {
      entries = await fs.readdir(dir, { withFileTypes: true })
    } catch (err) {
      throw fail(codeOf(err), dir)
    }
    stat.dirs++
    for (const e of entries) {
      const p = path.join(dir, e.name)
      if (e.isSymbolicLink()) stat.links++
      else if (e.isDirectory()) stack.push(p)
      else if (e.isFile()) {
        const st = await fs.lstat(p)
        if (st.isSymbolicLink()) {
          stat.links++
          continue
        }
        stat.files++
        stat.bytes += st.size
        opts.onFile?.(p, st.size)
      }
    }
  }
  return stat
}

export interface CopyOpts {
  platform: NodeJS.Platform
  signal?: AbortSignal
  fault?: Fault
  onBytes?: (bytes: number, files: number) => void
}

async function copyLink(src: string, dst: string, platform: NodeJS.Platform): Promise<void> {
  const target = await fs.readlink(src)
  if (platform !== 'win32') return fs.symlink(target, dst)
  const isDir = await fs
    .stat(src)
    .then((s) => s.isDirectory())
    .catch(() => false)
  const abs = path.win32.isAbsolute(target)
  await fs.symlink(target, dst, isDir ? (abs ? 'junction' : 'dir') : 'file')
}

export async function copyTree(src: string, dst: string, opts: CopyOpts): Promise<TreeStat> {
  const stat: TreeStat = { files: 0, bytes: 0, dirs: 0, links: 0 }
  const dirTimes: Array<{ path: string; atime: Date; mtime: Date }> = []
  await opts.fault?.('mkdir', dst)
  await fs.mkdir(dst)
  const stack: Array<[string, string]> = [[src, dst]]
  while (stack.length) {
    if (opts.signal?.aborted) throw new MoveCancelled()
    const [s, d] = stack.pop()!
    const sst = await fs.lstat(s)
    dirTimes.push({ path: d, atime: sst.atime, mtime: sst.mtime })
    stat.dirs++
    for (const e of await fs.readdir(s, { withFileTypes: true })) {
      if (opts.signal?.aborted) throw new MoveCancelled()
      const sp = path.join(s, e.name)
      const dp = path.join(d, e.name)
      const st = await fs.lstat(sp)
      if (st.isSymbolicLink()) {
        await opts.fault?.('link', sp)
        await copyLink(sp, dp, opts.platform)
        stat.links++
      } else if (st.isDirectory()) {
        await fs.mkdir(dp)
        stack.push([sp, dp])
      } else if (st.isFile()) {
        await opts.fault?.('copyFile', sp)
        await fs.copyFile(sp, dp, FS.COPYFILE_EXCL)
        await fs.utimes(dp, st.atime, st.mtime)
        stat.files++
        stat.bytes += st.size
        opts.onBytes?.(stat.bytes, stat.files)
      }
    }
  }
  for (let i = dirTimes.length - 1; i >= 0; i--) {
    const t = dirTimes[i]!
    await fs.utimes(t.path, t.atime, t.mtime).catch(() => undefined)
  }
  return stat
}

export const SAMPLE_COUNT = 6
const SAMPLE_MAX = 512 * 1024 * 1024

export async function hashFile(p: string): Promise<string> {
  const h = createHash('sha256')
  await new Promise<void>((resolve, reject) => {
    const s = createReadStream(p)
    s.on('data', (c) => h.update(c))
    s.on('error', reject)
    s.on('end', () => resolve())
  })
  return h.digest('hex')
}

export function pickSamples(files: ReadonlyArray<{ rel: string; size: number }>, n = SAMPLE_COUNT): string[] {
  const ok = files.filter((f) => f.size <= SAMPLE_MAX)
  if (!ok.length) return []
  const out = new Set<string>()
  for (const f of [...ok].sort((a, b) => b.size - a.size).slice(0, Math.ceil(n / 2))) out.add(f.rel)
  const step = Math.max(1, Math.floor(ok.length / Math.max(1, n - out.size)))
  for (let i = 0; i < ok.length && out.size < n; i += step) out.add(ok[i]!.rel)
  return [...out]
}

export interface VerifyResult {
  ok: boolean
  reason?: 'count' | 'bytes' | 'links' | 'hash'
  source: TreeStat
  copy: TreeStat
}

export async function verifyCopy(src: string, dst: string, signal?: AbortSignal, fault?: Fault): Promise<VerifyResult> {
  const files: Array<{ rel: string; size: number }> = []
  const source = await walkTree(src, { signal, onFile: (p, size) => files.push({ rel: path.relative(src, p), size }) })
  const copy = await walkTree(dst, { signal })
  await fault?.('verify', dst)
  if (source.files !== copy.files) return { ok: false, reason: 'count', source, copy }
  if (source.bytes !== copy.bytes) return { ok: false, reason: 'bytes', source, copy }
  if (source.links !== copy.links) return { ok: false, reason: 'links', source, copy }
  for (const rel of pickSamples(files)) {
    if (signal?.aborted) throw new MoveCancelled()
    const [a, b] = await Promise.all([hashFile(path.join(src, rel)), hashFile(path.join(dst, rel)).catch(() => '')])
    if (a !== b) return { ok: false, reason: 'hash', source, copy }
  }
  return { ok: true, source, copy }
}

export async function removeTree(p: string): Promise<void> {
  const st = await fs.lstat(p).catch(() => null)
  if (!st) return
  if (st.isSymbolicLink()) return fs.unlink(p)
  await fs.rm(p, { recursive: true, force: true, maxRetries: 3, retryDelay: 200 })
}

export async function retry<T>(fn: () => Promise<T>, times = 20, delayMs = 100): Promise<T> {
  let last: unknown
  for (let i = 0; i < times; i++) {
    try {
      return await fn()
    } catch (err) {
      last = err
      const c = codeOf(err)
      if (c !== 'EBUSY' && c !== 'EPERM' && c !== 'EACCES' && c !== 'ENOTEMPTY') break
      await new Promise((r) => setTimeout(r, delayMs))
    }
  }
  throw last
}
