import { constants, promises as fs } from 'node:fs'

export interface MoveIo {
  rename: (from: string, to: string) => Promise<void>
  copyFile: (from: string, to: string, mode: number) => Promise<void>
  lstat: (p: string) => Promise<{ size: number; mtimeMs: number; atimeMs: number; isFile(): boolean }>
  utimes: (p: string, atime: number, mtime: number) => Promise<void>
  unlink: (p: string) => Promise<void>
}

export const realMoveIo: MoveIo = {
  rename: (a, b) => fs.rename(a, b),
  copyFile: (a, b, m) => fs.copyFile(a, b, m),
  lstat: (p) => fs.lstat(p),
  utimes: (p, a, m) => fs.utimes(p, a / 1000, m / 1000),
  unlink: (p) => fs.unlink(p),
}

export class MoveError extends Error {
  constructor(readonly code: string) {
    super(code)
  }
}

const codeOf = (err: unknown): string => {
  const c = (err as { code?: unknown } | null)?.code
  return typeof c === 'string' && c ? c : err instanceof MoveError ? err.code : 'failed'
}

export async function exists(io: Pick<MoveIo, 'lstat'>, p: string): Promise<boolean> {
  try {
    await io.lstat(p)
    return true
  } catch (err) {
    if (codeOf(err) === 'ENOENT') return false
    throw err
  }
}

export async function moveFile(from: string, to: string, io: MoveIo = realMoveIo): Promise<'rename' | 'copy'> {
  if (await exists(io, to)) throw new MoveError('EEXIST')
  const src = await io.lstat(from)
  if (!src.isFile()) throw new MoveError('notfile')
  try {
    await io.rename(from, to)
    return 'rename'
  } catch (err) {
    if (codeOf(err) !== 'EXDEV') throw err
  }
  await io.copyFile(from, to, constants.COPYFILE_EXCL)
  try {
    const dst = await io.lstat(to)
    if (dst.size !== src.size) throw new MoveError('verify')
    await io.utimes(to, src.atimeMs, src.mtimeMs).catch(() => undefined)
    const again = await io.lstat(from)
    if (again.size !== src.size || again.mtimeMs !== src.mtimeMs) throw new MoveError('changed')
  } catch (err) {
    await io.unlink(to).catch(() => undefined)
    throw err
  }
  try {
    await io.unlink(from)
  } catch (err) {
    await io.unlink(to).catch(() => undefined)
    throw err
  }
  return 'copy'
}

export { codeOf as moveCode }
