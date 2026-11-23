import { dirKey } from '../../../core/stores/favorites'
import type { WtCleanup } from '../../../core/lib/preview'

export const isWindowsHost = typeof navigator !== 'undefined' && /Win/i.test(navigator.platform || navigator.userAgent)

export function joinSetup(commands: string[], windows: boolean = isWindowsHost): string {
  const list = commands.map((c) => c.trim()).filter(Boolean)
  if (!list.length) return ''
  if (!windows) return list.join(' && ')
  return list.reduceRight<string>((rest, cmd) => (rest ? `${cmd}; if ($?) { ${rest} }` : cmd), '')
}

export function isInside(child: string, parent: string): boolean {
  const c = dirKey(child)
  const p = dirKey(parent)
  return !!p && (c === p || c.startsWith(`${p}\\`))
}

export interface PaneLike {
  key: string
  cwd?: string
  liveCwd?: string
  path?: string
  root?: string
}

export function panesInside(panes: readonly PaneLike[], dir: string): string[] {
  return panes.filter((p) => [p.cwd, p.liveCwd, p.path, p.root].some((x) => !!x && isInside(x, dir))).map((p) => p.key)
}

export type CleanupError = 'denied' | 'not-repo' | 'unknown' | 'main' | 'dirty' | 'busy' | 'git' | 'timeout' | 'cleanup-failed' | 'cancelled' | 'failed' | 'unavailable'

export type CleanupOutcome =
  | { ok: true; path: string; branch: 'deleted' | 'kept' | 'none'; detail?: string }
  | { ok: false; error: CleanupError; detail?: string; step: 'check' | 'cleanup' | 'remove' }

export interface RemoveReply {
  t: string
  code?: string
  detail?: string
  branch?: 'deleted' | 'kept'
  branchDetail?: string
}

export interface CleanupDeps {
  check: (root: string, branch: string) => Promise<WtCleanup>
  run: (root: string, branch: string) => Promise<WtCleanup>
  panes: () => readonly PaneLike[]
  closePanes: (keys: string[]) => void
  remove: (root: string, path: string) => Promise<RemoveReply | null>
  wait: (ms: number) => Promise<void>
}

const BUSY_RETRIES = 6
const BUSY_WAIT_MS = 400

const REMOVE_CODES: Record<string, CleanupError> = { dirty: 'dirty', main: 'main', busy: 'busy', unknown: 'unknown', notRepo: 'not-repo', noGit: 'git', git: 'git' }

export async function cleanupWorktree(root: string, branch: string, deps: CleanupDeps): Promise<CleanupOutcome> {
  const checked = await deps.check(root, branch)
  if (!checked.ok) return { ok: false, error: checked.error, detail: checked.detail, step: 'check' }
  const path = checked.path
  if (checked.cleanup?.length) {
    const ran = await deps.run(root, branch)
    if (!ran.ok) return { ok: false, error: ran.error, detail: ran.detail, step: 'cleanup' }
  }
  const keys = panesInside(deps.panes(), path)
  if (keys.length) deps.closePanes(keys)
  let reply: RemoveReply | null = null
  for (let i = 0; i <= BUSY_RETRIES; i++) {
    reply = await deps.remove(root, path)
    if (reply?.t !== 'wt.error' || reply.code !== 'busy') break
    await deps.wait(BUSY_WAIT_MS)
  }
  if (reply?.t === 'wt.removed') return { ok: true, path, branch: reply.branch ?? 'none', detail: reply.branchDetail }
  const code = reply?.t === 'wt.error' ? (REMOVE_CODES[reply.code ?? ''] ?? 'failed') : 'failed'
  return { ok: false, error: code, detail: reply?.detail, step: 'remove' }
}
