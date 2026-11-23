import type { StashEntry, StashError, UndoCommitError } from '../../../core/lib/preview'

const pad = (n: number): string => String(n).padStart(2, '0')

export function defaultStashName(branch: string | null, now: Date): string {
  const stamp = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}`
  return `${branch || 'HEAD'} ${stamp}`
}

export function stashTitle(s: StashEntry): string {
  return s.message.trim() || `stash@{${s.index}}`
}

const STASH_ERRORS: ReadonlySet<StashError> = new Set(['nothing', 'no-commit', 'conflict', 'dirty', 'gone', 'timeout', 'denied', 'unavailable'])

export function stashErrorKey(error: StashError): string {
  return `v112stash.error.${STASH_ERRORS.has(error) ? error : 'failed'}`
}

const UNDO_ERRORS: ReadonlySet<UndoCommitError> = new Set(['no-commit', 'stale', 'root-commit', 'merge', 'pushed', 'timeout', 'denied', 'unavailable'])

export function undoErrorKey(error: UndoCommitError): string {
  return `v112stash.undoError.${UNDO_ERRORS.has(error) ? error : 'failed'}`
}

export function canOfferUndo(commit: { hash: string; parents: string[] } | undefined, headHash: string | null, ahead: number, hasUpstream: boolean): boolean {
  if (!commit || !headHash || commit.hash !== headHash) return false
  if (commit.parents.length !== 1) return false
  return !hasUpstream || ahead > 0
}
