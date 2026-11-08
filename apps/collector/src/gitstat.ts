import { execFile } from 'node:child_process'
import type { GitDiffStat } from '@ccm/shared'
import { findOnPath } from './win-exe'

export function parseShortstat(out: string): { files: number; insertions: number; deletions: number } {
  const n = (re: RegExp) => Number(out.match(re)?.[1] ?? 0)
  return { files: n(/(\d+) files? changed/), insertions: n(/(\d+) insertions?\(\+\)/), deletions: n(/(\d+) deletions?\(-\)/) }
}

let gitPath: string | null | undefined

export const NOT_A_REPO = Symbol('not-a-repo')

type GitResult = string | null | typeof NOT_A_REPO

function git(cwd: string, args: string[]): Promise<GitResult> {
  gitPath ??= process.platform === 'win32' ? findOnPath('git.exe') : 'git'
  const file = gitPath
  if (!file) return Promise.resolve(null)
  return new Promise((resolve) => {
    execFile(
      file,
      ['-C', cwd, ...args],
      // Optional locks off: a status read must never contend with the session's own git commands for index.lock.
      { timeout: 5_000, windowsHide: true, maxBuffer: 1_000_000, env: { ...process.env, GIT_OPTIONAL_LOCKS: '0', GIT_TERMINAL_PROMPT: '0' } },
      (err, stdout, stderr) => {
        if (!err) return resolve(String(stdout))
        const notRepo = (err as { code?: unknown }).code === 128 && /not a git repository/i.test(String(stderr))
        resolve(notRepo ? NOT_A_REPO : null)
      },
    )
  })
}

async function runDiffStat(cwd: string, now: number): Promise<GitDiffStat | null | typeof NOT_A_REPO> {
  const shortstat = await git(cwd, ['diff', '--shortstat', '--histogram', 'HEAD'])
  if (shortstat === null || shortstat === NOT_A_REPO) return shortstat
  const others = await git(cwd, ['ls-files', '--others', '--exclude-standard', '--directory', '--no-empty-directory'])
  const untracked = typeof others === 'string' ? others.split('\n').filter((l) => l.trim()).length : 0
  return { ...parseShortstat(shortstat), untracked, at: new Date(now).toISOString() }
}

export interface DiffStatReaderOptions {
  notRepoTtlMs?: number
  now?: () => number
}

export function createDiffStatReader(
  run: (cwd: string, now: number) => Promise<GitDiffStat | null | typeof NOT_A_REPO>,
  o: DiffStatReaderOptions = {},
): (cwd: string, now?: number) => Promise<GitDiffStat | null> {
  const notRepoTtl = o.notRepoTtlMs ?? 300_000
  const clock = o.now ?? Date.now
  type Slot = { cur: Promise<GitDiffStat | null>; next?: Promise<GitDiffStat | null> }
  const slots = new Map<string, Slot>()
  const notRepo = new Map<string, number>()
  const launch = (key: string, cwd: string, now: number): Promise<GitDiffStat | null> => {
    const at = notRepo.get(key)
    if (at !== undefined && clock() - at < notRepoTtl) return Promise.resolve(null)
    const cur = run(cwd, now)
      .catch(() => null)
      .then((value) => {
        if (value === NOT_A_REPO) {
          notRepo.set(key, clock())
          return null
        }
        notRepo.delete(key)
        return value
      })
    const slot: Slot = { cur }
    slots.set(key, slot)
    void cur.finally(() => {
      if (slots.get(key) === slot) slots.delete(key)
    })
    return cur
  }
  return (cwd, now = clock()) => {
    const key = process.platform === 'win32' ? cwd.toLowerCase() : cwd
    const slot = slots.get(key)
    if (!slot) return launch(key, cwd, now)
    slot.next ??= slot.cur.then(() => launch(key, cwd, clock()))
    return slot.next
  }
}

/** Uncommitted tracked changes vs HEAD plus untracked entries (folders count once); null outside a repo. */
export const readDiffStat = createDiffStatReader(runDiffStat)
