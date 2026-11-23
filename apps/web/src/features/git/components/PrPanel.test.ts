import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { i18n } from '../../../i18n'
import PrPanel from './PrPanel.vue'

const ghPr = vi.fn()
const ghCi = vi.fn()
const wtCleanup = vi.fn()
const openExternal = vi.fn()
const askCleanup = vi.fn()
let listener: ((e: unknown) => void) | null = null

vi.mock('../../../core/lib/preview', async (orig) => ({
  ...(await orig<Record<string, unknown>>()),
  hasGhPr: true,
  ghPr: (...a: unknown[]) => ghPr(...a),
  ghCi: (...a: unknown[]) => ghCi(...a),
  wtCleanup: (...a: unknown[]) => wtCleanup(...a),
  openExternal: (...a: unknown[]) => openExternal(...a),
  onGhEvent: (fn: (e: unknown) => void) => {
    listener = fn
    return () => {
      listener = null
    }
  },
}))
vi.mock('../lib/askCleanup', () => ({ askCleanupWorktree: (...a: unknown[]) => askCleanup(...a) }))

const SHA = 'a'.repeat(40)
const pr = (over: Record<string, unknown> = {}) => ({ number: 42, title: 'Add login', url: 'https://github.com/o/r/pull/42', state: 'open', draft: false, head: 'feat/login', headSha: SHA, base: 'main', mergeable: true, mergeableState: 'clean', ...over })
const run = (over: Record<string, unknown> = {}) => ({ name: 'build', state: 'success', url: 'https://github.com/o/r/runs/1', jobId: 1, startedAt: '2026-11-23T10:00:00Z', completedAt: '2026-11-23T10:01:20Z', ...over })

function open(props: Record<string, unknown> = {}) {
  return mount(PrPanel, {
    props: { root: 'C:\\w\\proj', branch: 'feat/login', headHash: SHA, subject: 'feat: login', upstream: 'origin/feat/login', ahead: 0, ...props },
    global: { plugins: [i18n], stubs: { Teleport: true } },
    attachTo: document.body,
  })
}

describe('PrPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    for (const m of [ghPr, ghCi, wtCleanup, openExternal, askCleanup]) m.mockReset()
    i18n.global.locale.value = 'en'
    listener = null
  })

  it('renders nothing when the repo is not on GitHub or has no account', async () => {
    ghPr.mockResolvedValue({ ok: false, error: 'gh.notGithub' })
    const w = open({ root: 'C:\\w\\a' })
    await flushPromises()
    expect(w.find('[data-pr-panel]').exists()).toBe(false)
    ghPr.mockResolvedValue({ ok: false, error: 'gh.noAccount' })
    const x = open({ root: 'C:\\w\\b' })
    await flushPromises()
    expect(x.find('[data-pr-panel]').exists()).toBe(false)
  })

  it('shows the missing permission with a Settings link on gh.scope', async () => {
    ghPr.mockResolvedValue({ ok: false, error: 'gh.scope', permission: 'pull_requests' })
    const w = open({ root: 'C:\\w\\c' })
    await flushPromises()
    expect(w.find('[data-pr-scope]').text()).toContain('The token is missing the pull_requests permission.')
    expect(w.find('[data-pr-scope] button').text()).toContain('Settings')
  })

  it('offers Create PR when there is no PR and the branch is pushed', async () => {
    ghPr.mockImplementation(async (r: { op: string }) => (r.op === 'find' ? { ok: true, pr: null } : { ok: true, default: 'main', branches: ['main'] }))
    const w = open({ root: 'C:\\w\\d' })
    await flushPromises()
    expect(w.find('[data-pr-state="noPr"]').exists()).toBe(true)
    expect(w.find('[data-pr-create-open]').exists()).toBe(true)
    expect(w.find('[data-pr-push]').exists()).toBe(false)
  })

  it('asks for a push first when the branch is not on the remote', async () => {
    ghPr.mockImplementation(async (r: { op: string }) => (r.op === 'find' ? { ok: true, pr: null } : { ok: true, default: 'main', branches: ['main'] }))
    const w = open({ root: 'C:\\w\\e', upstream: null })
    await flushPromises()
    expect(w.find('[data-pr-create-open]').exists()).toBe(false)
    await w.find('[data-pr-push]').trigger('click')
    expect(w.emitted('push')).toHaveLength(1)
  })

  it('shows a running PR with icon plus text and its checks', async () => {
    ghPr.mockResolvedValue({ ok: true, pr: pr() })
    ghCi.mockImplementation(async (r: { op: string }) => (r.op === 'status' ? { ok: true, state: 'pending', sha: SHA, runs: [run({ state: 'pending', completedAt: null }), run({ name: 'lint', jobId: 2 })] } : { ok: true, watch: 'added' }))
    const w = open({ root: 'C:\\w\\f' })
    await flushPromises()
    expect(w.find('[data-pr-ci]').attributes('data-pr-ci-state')).toBe('pending')
    expect(w.find('[data-pr-ci]').text()).toContain('CI running')
    expect(ghCi).toHaveBeenCalledWith(expect.objectContaining({ op: 'watch', branch: 'feat/login' }))
    await w.find('[data-pr-toggle]').trigger('click')
    expect(w.findAll('[data-pr-run]').map((n) => n.attributes('data-pr-run'))).toEqual(['pending', 'success'])
  })

  it('opens the last log lines of a failed job and sends them to the one agent pane without submitting', async () => {
    ghPr.mockResolvedValue({ ok: true, pr: pr() })
    ghCi.mockImplementation(async (r: { op: string }) => {
      if (r.op === 'status') return { ok: true, state: 'failure', sha: SHA, runs: [run({ name: 'test', state: 'failure', jobId: 9 })] }
      if (r.op === 'log') return { ok: true, lines: ['FAIL a.test.ts', 'expected 200'], cut: false }
      return { ok: true }
    })
    const w = open({ root: 'C:\\w\\g' })
    await flushPromises()
    await w.find('[data-pr-toggle]').trigger('click')
    await w.find('[data-pr-log-toggle]').trigger('click')
    await flushPromises()
    expect(ghCi).toHaveBeenCalledWith({ op: 'log', root: 'C:\\w\\g', jobId: 9, lines: 60 })
    expect(w.find('[data-pr-log-text]').text()).toContain('expected 200')
    expect(w.find('[data-pr-ci]').attributes('data-pr-ci-state')).toBe('failure')
  })

  it('refreshes on a ci.done event of the same branch only', async () => {
    ghPr.mockResolvedValue({ ok: true, pr: pr() })
    ghCi.mockResolvedValue({ ok: true, state: 'pending', sha: SHA, runs: [] })
    const w = open({ root: 'C:\\w\\h' })
    await flushPromises()
    const before = ghPr.mock.calls.length
    listener?.({ type: 'ci.done', root: 'C:\\w\\h', branch: 'other', state: 'success', failed: [], sha: SHA, pr: 42, url: '' })
    await flushPromises()
    expect(ghPr.mock.calls.length).toBe(before)
    listener?.({ type: 'ci.done', root: 'c:/w/h', branch: 'feat/login', state: 'success', failed: [], sha: SHA, pr: 42, url: '' })
    await flushPromises()
    expect(ghPr.mock.calls.length).toBeGreaterThan(before)
    w.unmount()
  })

  it('hides Merge for a merged PR and shows it for an open one', async () => {
    ghPr.mockResolvedValue({ ok: true, pr: pr() })
    ghCi.mockResolvedValue({ ok: true, state: 'success', sha: SHA, runs: [] })
    const a = open({ root: 'C:\\w\\i' })
    await flushPromises()
    expect(a.find('[data-pr-merge-open]').exists()).toBe(true)
    ghPr.mockResolvedValue({ ok: true, pr: pr({ state: 'merged' }) })
    const b = open({ root: 'C:\\w\\j' })
    await flushPromises()
    expect(b.find('[data-pr-merge-open]').exists()).toBe(false)
    expect(b.find('[data-pr-state-chip]').text()).toBe('Merged')
  })

  it('merges through the dialog, keeps the merged PR on screen and offers worktree cleanup', async () => {
    ghPr.mockImplementation(async (r: { op: string }) => (r.op === 'merge' ? { ok: true, merged: true, sha: 'c'.repeat(40), branchDeleted: true } : { ok: true, pr: pr() }))
    ghCi.mockResolvedValue({ ok: true, state: 'success', sha: SHA, runs: [] })
    wtCleanup.mockResolvedValue({ ok: true, path: 'C:\w\wt', cleanup: [] })
    askCleanup.mockResolvedValue('kept')
    const w = open({ root: 'C:\w\k' })
    await flushPromises()
    await w.find('[data-pr-merge-open]').trigger('click')
    await w.find('[data-pr-method="squash"]').setValue(true)
    await w.find('[data-pr-merge-submit]').trigger('submit')
    await flushPromises()
    expect(ghPr).toHaveBeenCalledWith({ op: 'merge', root: 'C:\w\k', number: 42, method: 'squash', sha: SHA, base: 'main', head: 'feat/login', deleteBranch: true })
    expect(w.find('[data-pr-state-chip]').text()).toBe('Merged')
    expect(w.find('[data-pr-merge-open]').exists()).toBe(false)
    expect(askCleanup).toHaveBeenCalledWith('C:\w\k', 'feat/login')
  })

  it('does not ask about a worktree when the branch is not one', async () => {
    ghPr.mockImplementation(async (r: { op: string }) => (r.op === 'merge' ? { ok: true, merged: true, sha: 'c'.repeat(40), branchDeleted: false } : { ok: true, pr: pr() }))
    ghCi.mockResolvedValue({ ok: true, state: 'success', sha: SHA, runs: [] })
    wtCleanup.mockResolvedValue({ ok: false, error: 'unknown' })
    const w = open({ root: 'C:\w\l' })
    await flushPromises()
    await w.find('[data-pr-merge-open]').trigger('click')
    await w.find('[data-pr-merge-submit]').trigger('submit')
    await flushPromises()
    expect(askCleanup).not.toHaveBeenCalled()
  })
})
