import { describe, expect, it, vi } from 'vitest'
import { cleanupWorktree, joinSetup, panesInside, type CleanupDeps } from './worktreeFlow'

describe('joinSetup', () => {
  it('chains with && for POSIX shells', () => {
    expect(joinSetup(['npm ci', 'npm run gen'], false)).toBe('npm ci && npm run gen')
    expect(joinSetup(['  a  ', '', 'b'], false)).toBe('a && b')
    expect(joinSetup([], false)).toBe('')
  })
  it('nests if ($?) for Windows PowerShell 5.1', () => {
    expect(joinSetup(['a'], true)).toBe('a')
    expect(joinSetup(['a', 'b', 'c'], true)).toBe('a; if ($?) { b; if ($?) { c } }')
  })
})

describe('panesInside', () => {
  const panes = [
    { key: 'a', cwd: 'C:\\w\\proj-wt\\feat' },
    { key: 'b', cwd: 'C:\\w\\proj', liveCwd: 'c:/w/proj-wt/feat/sub' },
    { key: 'c', path: 'C:\\w\\proj-wt\\feat\\x.ts' },
    { key: 'd', cwd: 'C:\\w\\proj-wt\\feat2' },
    { key: 'e', root: 'C:/w/proj-wt/feat/' },
    { key: 'f' },
  ]
  it('matches whole path components in cwd, liveCwd, file path and root', () => {
    expect(panesInside(panes, 'C:\\w\\proj-wt\\feat')).toEqual(['a', 'b', 'c', 'e'])
  })
  it('matches nothing for an empty dir', () => {
    expect(panesInside(panes, '')).toEqual([])
  })
})

function deps(over: Partial<CleanupDeps> = {}): CleanupDeps {
  return {
    check: vi.fn(async () => ({ ok: true as const, path: 'C:\\w\\wt', cleanup: [] })),
    run: vi.fn(async () => ({ ok: true as const, path: 'C:\\w\\wt', ran: 1 })),
    panes: () => [
      { key: 'p1', cwd: 'C:\\w\\wt' },
      { key: 'p2', cwd: 'C:\\w\\other' },
    ],
    closePanes: vi.fn(),
    remove: vi.fn(async () => ({ t: 'wt.removed', branch: 'deleted' as const })),
    wait: vi.fn(async () => undefined),
    ...over,
  }
}

describe('cleanupWorktree', () => {
  it('closes only the worktree panes, removes with the branch and reports it deleted', async () => {
    const d = deps()
    expect(await cleanupWorktree('C:\\w\\main', 'feat', d)).toEqual({ ok: true, path: 'C:\\w\\wt', branch: 'deleted', detail: undefined })
    expect(d.closePanes).toHaveBeenCalledWith(['p1'])
    expect(d.run).not.toHaveBeenCalled()
  })
  it('stops before touching anything when the check refuses (dirty)', async () => {
    const d = deps({ check: vi.fn(async () => ({ ok: false as const, error: 'dirty' as const })) })
    expect(await cleanupWorktree('r', 'feat', d)).toMatchObject({ ok: false, error: 'dirty', step: 'check' })
    expect(d.closePanes).not.toHaveBeenCalled()
    expect(d.remove).not.toHaveBeenCalled()
  })
  it('runs the cleanup commands first and stops on a failure or timeout', async () => {
    const check = vi.fn(async () => ({ ok: true as const, path: 'C:\\w\\wt', cleanup: ['docker compose down'] }))
    const timeout = deps({ check, run: vi.fn(async () => ({ ok: false as const, error: 'timeout' as const, detail: 'docker compose down' })) })
    expect(await cleanupWorktree('r', 'feat', timeout)).toMatchObject({ ok: false, error: 'timeout', step: 'cleanup', detail: 'docker compose down' })
    expect(timeout.closePanes).not.toHaveBeenCalled()
    expect(timeout.remove).not.toHaveBeenCalled()
    const ok = deps({ check })
    await cleanupWorktree('r', 'feat', ok)
    expect(ok.run).toHaveBeenCalledOnce()
    expect(ok.remove).toHaveBeenCalledOnce()
  })
  it('waits and retries while a closed terminal is still shutting down', async () => {
    const remove = vi
      .fn()
      .mockResolvedValueOnce({ t: 'wt.error', code: 'busy' })
      .mockResolvedValueOnce({ t: 'wt.error', code: 'busy' })
      .mockResolvedValueOnce({ t: 'wt.removed' })
    const d = deps({ remove })
    expect(await cleanupWorktree('r', 'feat', d)).toMatchObject({ ok: true, branch: 'none' })
    expect(remove).toHaveBeenCalledTimes(3)
    expect(d.wait).toHaveBeenCalledTimes(2)
  })
  it('gives up with busy after the retries', async () => {
    const remove = vi.fn(async () => ({ t: 'wt.error', code: 'busy' }))
    const d = deps({ remove })
    expect(await cleanupWorktree('r', 'feat', d)).toMatchObject({ ok: false, error: 'busy', step: 'remove' })
    expect(remove).toHaveBeenCalledTimes(7)
  })
  it('reports a kept branch with git detail and maps remove errors', async () => {
    const kept = deps({ remove: vi.fn(async () => ({ t: 'wt.removed', branch: 'kept' as const, branchDetail: 'not fully merged' })) })
    expect(await cleanupWorktree('r', 'feat', kept)).toMatchObject({ ok: true, branch: 'kept', detail: 'not fully merged' })
    const dirty = deps({ remove: vi.fn(async () => ({ t: 'wt.error', code: 'dirty' })) })
    expect(await cleanupWorktree('r', 'feat', dirty)).toMatchObject({ ok: false, error: 'dirty', step: 'remove' })
    const none = deps({ remove: vi.fn(async () => null) })
    expect(await cleanupWorktree('r', 'feat', none)).toMatchObject({ ok: false, error: 'failed' })
  })
})
