import { baseRev, safeRepoPath } from './changes'
import { execGit } from './gitExec'
import { resolveCommit } from './log'

export type HeadImage =
  | { ok: true; dataUrl: string; size: number }
  | { ok: false; error: 'denied' | 'unsupported' | 'missing' | 'too-large' | 'failed' }

export type ImageSide = 'old' | 'new'

export const MAX_IMAGE_BYTES = 20 * 1024 * 1024

const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
  svg: 'image/svg+xml',
  bmp: 'image/bmp',
  ico: 'image/x-icon',
}

export const imageMime = (rel: string): string | null => MIME[/\.([a-z0-9]+)$/i.exec(rel)?.[1]?.toLowerCase() ?? ''] ?? null

async function blobImage(root: string, spec: string, mime: string): Promise<HeadImage> {
  const kind = await execGit(root, ['cat-file', '-t', spec], { maxBytes: 64, timeoutMs: 10_000 })
  if (kind.code !== 0 || kind.stdout.trim() !== 'blob') return { ok: false, error: 'missing' }
  const size = await execGit(root, ['cat-file', '-s', spec], { maxBytes: 64, timeoutMs: 10_000 })
  const bytes = Number(size.stdout.trim())
  if (size.code !== 0 || !Number.isFinite(bytes)) return { ok: false, error: 'failed' }
  if (bytes > MAX_IMAGE_BYTES) return { ok: false, error: 'too-large' }
  const blob = await execGit(root, ['cat-file', 'blob', spec], { binary: true, maxBytes: MAX_IMAGE_BYTES, timeoutMs: 20_000 })
  if (blob.code !== 0 || blob.truncated || !blob.bytes) return { ok: false, error: 'failed' }
  return { ok: true, dataUrl: `data:${mime};base64,${blob.bytes.toString('base64')}`, size: blob.bytes.length }
}

/** HEAD's copy of an image in `root` (already a known repo toplevel) as a data URL; `rel` relative to the repo. */
export async function headImage(root: string, relInput: unknown): Promise<HeadImage> {
  const safe = safeRepoPath(root, relInput)
  if (!safe) return { ok: false, error: 'denied' }
  const mime = imageMime(safe.rel)
  if (!mime) return { ok: false, error: 'unsupported' }
  if ((await baseRev(root)) !== 'HEAD') return { ok: false, error: 'missing' }
  return blobImage(root, `HEAD:${safe.rel}`, mime)
}

export async function commitImage(root: string, hashInput: unknown, relInput: unknown, sideInput: unknown, origInput?: unknown): Promise<HeadImage> {
  const safe = safeRepoPath(root, relInput)
  const hasOrig = origInput !== undefined && origInput !== null
  const orig = hasOrig ? safeRepoPath(root, origInput) : null
  if (!safe || (hasOrig && !orig) || (sideInput !== 'old' && sideInput !== 'new')) return { ok: false, error: 'denied' }
  const rel = sideInput === 'old' && orig ? orig.rel : safe.rel
  const mime = imageMime(rel)
  if (!mime) return { ok: false, error: 'unsupported' }
  const full = await resolveCommit(root, hashInput)
  if (full === 'denied' || full === 'missing') return { ok: false, error: full }
  let rev = full
  if (sideInput === 'old') {
    const parents = (await execGit(root, ['rev-list', '--parents', '-n', '1', full], { maxBytes: 16_384 })).stdout.trim().split(' ').slice(1)
    if (!parents[0]) return { ok: false, error: 'missing' }
    rev = parents[0]
  }
  return blobImage(root, `${rev}:${rel}`, mime)
}
