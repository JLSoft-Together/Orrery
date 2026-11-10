import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerPreviewIpc } from '../src/preview/ipc'
import type { PreviewServer } from '../src/preview/server'
import type { RootWatcher } from '../src/preview/watch'
import { findGit } from '../src/pty/git'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-pgit-')))
const GIT = findGit()

const ENV = {
  GIT_AUTHOR_NAME: 'Twin',
  GIT_AUTHOR_EMAIL: 'twin@example.com',
  GIT_COMMITTER_NAME: 'Twin',
  GIT_COMMITTER_EMAIL: 'twin@example.com',
  GIT_AUTHOR_DATE: '2026-01-02T03:04:05Z',
  GIT_COMMITTER_DATE: '2026-01-02T03:04:05Z',
  GIT_CONFIG_NOSYSTEM: '1',
  GIT_CONFIG_GLOBAL: path.join(tmp, 'gitconfig'),
}

const PNG = Buffer.from('89504e470d0a1a0a0000000d4948445200000001000000010806000000', 'hex')

function git(cwd: string, ...args: string[]): string {
  return execFileSync(GIT!, args, { cwd, env: { ...process.env, ...ENV }, encoding: 'utf8' })
}

function makeSide(name: string): { root: string; repo: string; remote: string; other: string } {
  const root = path.join(tmp, name)
  const repo = path.join(root, 'repo')
  const remote = path.join(root, 'remote.git')
  const other = path.join(root, 'other')
  mkdirSync(repo, { recursive: true })
  mkdirSync(other, { recursive: true })
  git(root, 'init', '-q', '--bare', '-b', 'main', remote)
  git(repo, 'init', '-q', '-b', 'main')
  writeFileSync(path.join(repo, 'a.txt'), 'one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n')
  writeFileSync(path.join(repo, 'b.txt'), 'bee\n')
  writeFileSync(path.join(repo, 'img.png'), PNG)
  git(repo, 'add', '-A')
  git(repo, 'commit', '-q', '-m', 'first')
  writeFileSync(path.join(repo, 'b.txt'), 'bee\nsecond line\n')
  git(repo, 'mv', 'img.png', 'pic.png')
  git(repo, 'commit', '-q', '-a', '-m', 'second change')
  git(repo, 'tag', 'v1')
  git(repo, 'remote', 'add', 'origin', remote)
  writeFileSync(path.join(repo, 'a.txt'), 'one\nTWO\nthree\nfour\nfive\nsix\nseven\neight\nnine\nTEN\n')
  writeFileSync(path.join(repo, 'new file.txt'), 'n1\nn2')
  writeFileSync(path.join(repo, 'bin.dat'), Buffer.from([1, 0, 2, 3]))
  return { root, repo, remote, other }
}

function side(roots: string[], dataDir: string, twin?: Twin) {
  const ipc = fakeIpc()
  const confirms: unknown[] = []
  const trashed: string[] = []
  registerPreviewIpc({
    ipcMain: ipc.ipcMain,
    isAppFrame: () => true,
    server: { tokensUnder: () => [] } as unknown as PreviewServer,
    watcher: { listen: () => undefined } as unknown as RootWatcher,
    shell: {} as never,
    dialog: {} as never,
    getWindow: () => null,
    getRoots: () => roots,
    confirmAccess: async () => false,
    confirmGit: async (req) => {
      confirms.push(req)
      return true
    },
    confirmRun: async (req) => {
      confirms.push(req)
      return true
    },
    trash: async (p) => {
      trashed.push(p)
      rmSync(p, { force: true, recursive: true })
    },
    github: {
      envFor: async () => ({ CCM_TWIN_TOKEN: 'x' }),
      resolve: async (root: string, url: string) => ({ github: true, root, url }),
      setChoice: (_root: string, choice: unknown) => choice === 'a',
    } as never,
    dataDir,
    desk: twin?.router,
  })
  return { ...ipc, confirms, trashed, ev: { sender: { id: 7 } } }
}

const SKIP = !HAVE_DESK || !GIT

describe.skipIf(SKIP)('desk previewGit twin (TS vs orrery-core desk)', () => {
  let twin: Twin
  let n: ReturnType<typeof makeSide>
  let r: ReturnType<typeof makeSide>
  let node: ReturnType<typeof side>
  let rust: ReturnType<typeof side>
  const saved: Record<string, string | undefined> = {}

  beforeAll(async () => {
    writeFileSync(ENV.GIT_CONFIG_GLOBAL, '')
    for (const [k, v] of Object.entries(ENV)) {
      saved[k] = process.env[k]
      process.env[k] = v
    }
    n = makeSide('n')
    r = makeSide('r')
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
    node = side([n.repo], path.join(tmp, 'data-n'))
    rust = side([r.repo], path.join(tmp, 'data'), twin)
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]
      else process.env[k] = v
    }
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = async (channel: string, mk: (s: ReturnType<typeof makeSide>) => unknown[]): Promise<unknown> => {
    const a = await node.call(channel, node.ev, ...mk(n))
    const b = await rust.call(channel, rust.ev, ...mk(r))
    expect(b, channel).toEqual(swapRoot(a, n.root, r.root))
    return b
  }

  it('reads status, diffs, images and file diffs the same', async () => {
    expect(twin.host.channels).toContain('ccm:changes-status')
    const st = (await same('ccm:changes-status', (s) => [s.repo])) as { ok: boolean; entries: unknown[] }
    expect(st.ok).toBe(true)
    expect(st.entries.length).toBeGreaterThan(2)
    await same('ccm:changes-diff', (s) => [{ root: s.repo, path: 'a.txt' }])
    await same('ccm:changes-diff', (s) => [{ root: s.repo, path: 'a.txt', opts: { context: 'full', whitespace: 'all' } }])
    await same('ccm:changes-diff', (s) => [{ root: s.repo, path: 'new file.txt' }])
    await same('ccm:changes-diff', (s) => [{ root: s.repo, path: 'bin.dat' }])
    await same('ccm:changes-diff', (s) => [{ root: s.repo, path: '../x' }])
    await same('ccm:changes-diff', (s) => [{ root: s.other, path: 'a.txt' }])
    await same('ccm:git-show-image', (s) => [{ root: s.repo, path: 'pic.png' }])
    await same('ccm:git-show-image', (s) => [{ root: s.repo, path: 'a.txt' }])
    await same('ccm:git-show-image', (s) => [{ root: s.repo, path: 'none.png' }])
    await same('ccm:changes-file', (s) => [path.join(s.repo, 'a.txt')])
    await same('ccm:changes-file', (s) => [path.join(s.repo, 'new file.txt')])
    await same('ccm:changes-file', (s) => [path.join(s.other, 'nope.txt')])
  })

  it('reads the log the same', async () => {
    await same('ccm:log-refs', (s) => [s.repo])
    const page = (await same('ccm:log-page', (s) => [{ root: s.repo, query: {} }])) as { commits: { hash: string }[] }
    const [head, first] = page.commits.map((c) => c.hash)
    await same('ccm:log-page', () => [{ root: '', query: {} }])
    await same('ccm:log-page', (s) => [{ root: s.repo, query: { text: first!.slice(0, 8) } }])
    await same('ccm:log-page', (s) => [{ root: s.repo, query: { author: 'twin', paths: ['b.txt'], since: 1, skip: 0 } }])
    await same('ccm:log-page', (s) => [{ root: s.repo, query: { branch: 'refs/tags/v1', text: 'Second', regex: true, matchCase: false } }])
    await same('ccm:log-commit', (s) => [{ root: s.repo, hash: head }])
    await same('ccm:log-commit', (s) => [{ root: s.repo, hash: first }])
    await same('ccm:log-commit', (s) => [{ root: s.repo, hash: 'zz' }])
    await same('ccm:log-contains', (s) => [{ root: s.repo, hash: first }])
    await same('ccm:log-diff', (s) => [{ root: s.repo, hash: head, path: 'b.txt' }])
    await same('ccm:log-diff', (s) => [{ root: s.repo, hash: head, path: 'pic.png', origPath: 'img.png' }])
    await same('ccm:log-diff', (s) => [{ root: s.repo, hash: first, path: 'a.txt', opts: { context: 'full' } }])
    await same('ccm:log-diff', (s) => [{ root: s.repo, hash: first, path: 'a.txt', origPath: null }])
    await same('ccm:git-blame', (s) => [path.join(s.repo, 'b.txt')])
    await same('ccm:git-blame', (s) => [path.join(s.repo, 'new file.txt')])
    await same('ccm:git-merge-state', (s) => [s.repo])
    await same('ccm:git-merge-abort', (s) => [s.repo])
    await same('ccm:git-can-amend', (s) => [s.repo])
  })

  it('stages, commits and discards the same', async () => {
    await same('ccm:git-stage', (s) => [{ root: s.repo, paths: ['b.txt', '../x'] }])
    await same('ccm:git-stage', (s) => [{ root: s.repo, paths: ['a.txt'], hunk: '@@ -1,2 +1,2 @@\n one\n-two\n+TWO\n' }])
    await same('ccm:changes-status', (s) => [s.repo])
    await same('ccm:git-unstage', (s) => [{ root: s.repo, paths: 'a.txt' }])
    await same('ccm:git-stage', (s) => [{ root: s.repo, paths: ['new file.txt'] }])
    await same('ccm:git-commit', (s) => [{ root: s.repo, message: '   ' }])
    expect(await same('ccm:git-commit', (s) => [{ root: s.repo, message: 'third\r\n\r\nbody' }])).toMatchObject({ ok: true, subject: 'third' })
    await same('ccm:git-commit', (s) => [{ root: s.repo, message: 'nothing staged' }])
    await same('ccm:git-commit', (s) => [{ root: s.repo, message: 'third amended', amend: true }])
    await same('ccm:git-commit-cancel', (s) => [s.repo])
    await same('ccm:changes-discard', (s) => [{ root: s.repo, path: 'a.txt', hunk: '@@ -9,2 +9,2 @@\n nine\n-ten\n+TEN\n' }])
    await same('ccm:changes-discard', (s) => [{ root: s.repo, path: 'a.txt' }])
    await same('ccm:changes-discard', (s) => [{ root: s.repo, path: 'bin.dat' }])
    await same('ccm:changes-discard', (s) => [{ root: s.repo, path: 'gone.txt' }])
    await same('ccm:changes-status', (s) => [s.repo])
    expect(swapRoot(node.trashed, n.root, r.root)).toEqual(rust.trashed)
    expect(swapRoot(node.confirms, n.root, r.root)).toEqual(rust.confirms)
  })

  it('creates, pushes, fetches and deletes branches the same', async () => {
    await same('ccm:git-branch-create', (s) => [{ root: s.repo, name: 'feat/x', base: 'HEAD', checkout: false }])
    await same('ccm:git-branch-create', (s) => [{ root: s.repo, name: 'bad..name', base: 'HEAD' }])
    await same('ccm:git-branch-create', (s) => [{ root: s.repo, name: 'feat/x', base: 'refs/tags/v1' }])
    await same('ccm:git-push-preview', (s) => [{ root: s.repo, ref: 'refs/heads/main' }])
    expect(await same('ccm:git-push', (s) => [{ root: s.repo, ref: 'refs/heads/main', remote: 'origin', branch: 'main', setUpstream: true }])).toEqual({ ok: true })
    await same('ccm:git-push', (s) => [{ root: s.repo, ref: 'refs/heads/feat/x', remote: 'origin', branch: 'feat/x' }])
    await same('ccm:git-push', (s) => [{ root: s.repo, ref: 'refs/heads/main', remote: 'nope', branch: 'main' }])
    await same('ccm:git-push-preview', (s) => [{ root: s.repo, ref: 'refs/heads/feat/x', remote: 'origin', branch: 'other' }])
    await same('ccm:git-fetch', (s) => [s.repo])
    await same('ccm:git-branch-update', (s) => [{ root: s.repo, ref: 'refs/heads/main' }])
    await same('ccm:git-branch-update', (s) => [{ root: s.repo, ref: 'refs/heads/feat/x' }])
    await same('ccm:log-refs', (s) => [s.repo])
    await same('ccm:gh-repo-account', (s) => [{ root: s.repo, remote: 'origin' }])
    await same('ccm:gh-repo-account', (s) => [{ root: s.repo, remote: '-x' }])
    await same('ccm:gh-repo-account-set', (s) => [{ root: s.repo, choice: 'a' }])
    await same('ccm:git-branch-delete', (s) => [{ root: s.repo, ref: 'refs/heads/main' }])
    await same('ccm:git-branch-delete', (s) => [{ root: s.repo, ref: 'refs/remotes/origin/feat/x' }])
    await same('ccm:git-branch-delete', (s) => [{ root: s.repo, ref: 'refs/heads/feat/x' }])
    await same('ccm:log-refs', (s) => [s.repo])
    await same('ccm:git-can-amend', (s) => [s.repo])
    expect(swapRoot(node.confirms, n.root, r.root)).toEqual(rust.confirms)
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  }, 60_000)

  it('compares worktrees, dry-merges and keeps checkpoints the same', async () => {
    for (const s of [n, r]) {
      git(s.repo, 'worktree', 'add', '-q', '-b', 'side', path.join(s.root, 'wt'), 'main')
      writeFileSync(path.join(s.root, 'wt', 'b.txt'), 'bee\nfrom side\n')
      writeFileSync(path.join(s.root, 'wt', 'c.txt'), 'see\n')
      git(path.join(s.root, 'wt'), 'add', '-A')
      git(path.join(s.root, 'wt'), 'commit', '-q', '-m', 'side work')
    }
    await same('ccm:wt-compare', (s) => [{ dir: s.repo, branch: 'side' }])
    await same('ccm:wt-compare', (s) => [{ dir: s.repo, branch: 'main' }])
    await same('ccm:wt-compare', (s) => [{ dir: path.join(s.root, 'wt'), branch: 'side' }])
    await same('ccm:wt-diff', (s) => [{ dir: s.repo, a: 'main', b: 'side' }])
    await same('ccm:wt-file-diff', (s) => [{ dir: s.repo, a: 'main', b: 'side', path: 'b.txt' }])
    await same('ccm:overlap-files', (s) => [s.repo])
    await same('ccm:overlap-dryrun', (s) => [{ dir: s.repo, a: 'main', b: 'side' }])
    await same('ccm:overlap-dryrun', (s) => [{ dir: s.repo, a: 'main', b: 'main' }])
    await same('ccm:wt-take', (s) => [{ dir: s.repo, from: 'side', path: 'c.txt' }])
    await same('ccm:wt-merge', (s) => [{ dir: s.repo, branch: 'side', mode: 'squash' }])
    for (const s of [n, r]) git(s.repo, 'reset', '-q', '--hard', 'HEAD')
    await same('ccm:wt-merge', (s) => [{ dir: s.repo, branch: 'side', mode: 'ff' }])
    await same('ccm:wt-merge', (s) => [{ dir: s.repo, branch: 'side', mode: 'squash', keep: true }])
    await same('ccm:wt-merge', (s) => [{ dir: s.repo, branch: 'side', mode: 'rebase' }])

    await same('ccm:cp-state', (s) => [s.repo])
    await same('ccm:cp-take', (s) => [{ root: s.repo, label: 'off' }])
    await same('ccm:cp-set', (s) => [{ root: s.repo, on: true }])
    const t1 = (await same('ccm:cp-take', (s) => [{ root: s.repo, label: 'one\nline' }])) as { id: string }
    await same('ccm:cp-take', (s) => [{ root: s.repo }])
    for (const s of [n, r]) writeFileSync(path.join(s.repo, 'a.txt'), 'one\nchanged\nthree\n')
    for (const s of [n, r]) writeFileSync(path.join(s.repo, 'later.txt'), 'later\n')
    const t2 = (await same('ccm:cp-take', (s) => [{ root: s.repo, label: 'two' }])) as { id: string }
    expect([t1.id, t2.id].every((id) => /^[0-9a-f]{40,64}$/.test(id))).toBe(true)
    await same('ccm:cp-state', (s) => [s.repo])
    await same('ccm:cp-diff', (s) => [{ root: s.repo, id: t1.id }])
    await same('ccm:cp-diff', (s) => [{ root: s.repo, id: t1.id, to: t2.id }])
    await same('ccm:cp-diff', (s) => [{ root: s.repo, id: 'f'.repeat(40) }])
    await same('ccm:cp-file-diff', (s) => [{ root: s.repo, from: t1.id, to: t2.id, path: 'a.txt' }])
    await same('ccm:cp-revert-hunk', (s) => [{ root: s.repo, path: 'a.txt', patch: '@@ -1,3 +1,3 @@\n one\n-changed\n+nope\n three\n' }])
    await same('ccm:cp-restore', (s) => [{ root: s.repo, id: t1.id, paths: ['a.txt', 'later.txt'] }])
    await same('ccm:changes-status', (s) => [s.repo])
    await same('ccm:cp-set', (s) => [{ root: s.repo, on: false }])
    await same('ccm:cp-state', (s) => [s.repo])
    expect(swapRoot(node.confirms, n.root, r.root)).toEqual(rust.confirms)
    expect(swapRoot(node.trashed, n.root, r.root)).toEqual(rust.trashed)
    expect(twin.logs).toEqual([])
  }, 90_000)
})
