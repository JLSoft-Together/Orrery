/** A GitHub account added with a personal access token (v58). The token itself never leaves main. */
export interface GhAccount {
  id: string
  login: string
  name: string | null
  /** Classic token scopes (`X-OAuth-Scopes`); null for a fine-grained token, which reports none. */
  scopes: string[] | null
  addedAt: number
  isDefault: boolean
}

export type GhAccountError = 'invalid' | 'network' | 'no-keychain' | 'not-found' | 'cancelled' | 'denied' | 'failed'

export type GhAccountsResult = { ok: true; accounts: GhAccount[] } | { ok: false; error: GhAccountError; detail?: string }

/** Per repo: `auto` picks the account that can push (default first), `system` = Git Credential Manager, else an account id. */
export type GhRepoChoice = string

export type GhAccess = 'push' | 'read' | 'none' | 'unknown'

/** Which account git network commands of one repo + remote use. `github: false` = not an https github.com remote. */
export type GhRepoAccount =
  | { github: false }
  | { github: true; owner: string; repo: string; choice: GhRepoChoice; account: { id: string; login: string } | null; access: GhAccess }

/** New classic token page with the scopes git push needs (`workflow` for repos with GitHub Actions files). */
export const GH_TOKEN_URL = 'https://github.com/settings/tokens/new?scopes=repo,workflow,read:org&description=Orrery'

/** `owner/repo` of a github.com remote URL; `https` false for SSH forms, where a token does not apply. */
export function githubRepoOf(url: string): { owner: string; repo: string; https: boolean } | null {
  const s = url.trim()
  const m =
    /^https:\/\/(?:[^@/\s]+@)?github\.com(?::443)?\/([\w.-]+)\/([\w.-]+?)(?:\.git)?\/?$/i.exec(s) ??
    /^(?:ssh:\/\/)?(?:[\w.-]+@)?(?:ssh\.)?github\.com[:/](?:443\/)?([\w.-]+)\/([\w.-]+?)(?:\.git)?\/?$/i.exec(s)
  if (!m) return null
  return { owner: m[1]!, repo: m[2]!, https: /^https:/i.test(s) }
}

export type GhError =
  | 'denied'
  | 'invalid'
  | 'cancelled'
  | 'network'
  | 'timeout'
  | 'no-remote'
  | 'no-keychain'
  | 'gh.notGithub'
  | 'gh.noAccount'
  | 'gh.noAccess'
  | 'gh.auth'
  | 'gh.scope'
  | 'gh.rateLimit'
  | 'gh.notFound'
  | 'gh.notMergeable'
  | 'gh.conflict'
  | 'gh.invalid'
  | 'gh.watchMax'
  | 'gh.logGone'
  | 'gh.forkHead'
  | 'gh.http'

export interface GhFail {
  ok: false
  error: GhError
  detail?: string
  permission?: string
}

export type GhResult<T> = ({ ok: true } & T) | GhFail

export type GhPrState = 'open' | 'closed' | 'merged'

export interface GhPr {
  number: number
  title: string
  url: string
  state: GhPrState
  draft: boolean
  head: string
  headSha: string
  base: string
  mergeable: boolean | null
  mergeableState: string | null
}

export type GhMergeMethod = 'merge' | 'squash' | 'rebase'

export type GhWatchAdded = 'added' | 'updated' | 'exists'

export type GhPrRequest =
  | { op: 'find'; root: string; remote?: string; head: string }
  | { op: 'base'; root: string; remote?: string }
  | { op: 'create'; root: string; remote?: string; base: string; head: string; title: string; body: string }
  | { op: 'merge'; root: string; remote?: string; number: number; method: GhMergeMethod; sha: string; base: string; head?: string; deleteBranch?: boolean }
  | { op: 'deleteBranch'; root: string; remote?: string; branch: string }

export type GhCiState = 'pending' | 'success' | 'failure' | 'neutral' | 'none'

export interface GhCiRun {
  name: string | null
  state: Exclude<GhCiState, 'none'>
  url: string | null
  jobId: number | null
  startedAt: string | null
  completedAt: string | null
}

export interface GhCiStatus {
  state: GhCiState
  runs: GhCiRun[]
  sha: string
}

export interface GhCiWatch {
  root: string
  branch: string
  sha: string
  pr: number | null
  since: number
}

export type GhCiRequest =
  | { op: 'status'; root: string; remote?: string; sha: string }
  | { op: 'log'; root: string; remote?: string; jobId: number; lines?: number }
  | { op: 'watch'; root: string; remote?: string; branch: string; sha: string; pr?: number }
  | { op: 'unwatch'; root: string; branch: string }
  | { op: 'list' }

export interface GhCiLog {
  lines: string[]
  cut: boolean
}

export interface GhCiDoneEvent {
  type: 'ci.done'
  root: string
  branch: string
  sha: string
  pr: number | null
  state: GhCiState
  failed: string[]
  url: string
  timedOut?: true
}
