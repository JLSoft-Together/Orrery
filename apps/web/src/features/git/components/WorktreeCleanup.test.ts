import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { i18n } from '../../../i18n'
import WorktreeCleanup from './WorktreeCleanup.vue'

const wtCleanup = vi.fn()
const request = vi.fn()
vi.mock('../../../core/lib/preview', () => ({ wtCleanup: (...a: unknown[]) => wtCleanup(...a) }))
vi.mock('../../workspace/lib/ptyClient', () => ({ PtyConnection: { request: (...a: unknown[]) => request(...a) } }))

function open() {
  return mount(WorktreeCleanup, {
    props: { root: 'C:\\w\\main', branch: 'feat/x' },
    global: { plugins: [i18n], stubs: { Teleport: true, Transition: false } },
    attachTo: document.body,
  })
}

describe('WorktreeCleanup', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    wtCleanup.mockReset()
    request.mockReset()
    i18n.global.locale.value = 'en'
  })

  it('asks Remove / Keep for the branch and lists cleanup commands', async () => {
    wtCleanup.mockResolvedValue({ ok: true, path: 'C:\\w\\wt', cleanup: ['docker compose down'] })
    const w = open()
    await flushPromises()
    expect(w.text()).toContain('Remove worktree and branch feat/x?')
    expect(w.text()).toContain('docker compose down')
    expect(w.find('[data-wt-cleanup-remove]').exists()).toBe(true)
    await w.find('[data-wt-cleanup-keep]').trigger('click')
    expect(w.emitted('done')).toEqual([['kept']])
  })

  it('removes: cleanup, then wt.remove with deleteBranch, then reports done', async () => {
    wtCleanup.mockImplementation(async (_r: string, _b: string, phase: string) => (phase === 'check' ? { ok: true, path: 'C:\\w\\wt', cleanup: [] } : { ok: true, path: 'C:\\w\\wt' }))
    request.mockResolvedValue({ t: 'wt.removed', path: 'C:\\w\\wt', branch: 'deleted' })
    const w = open()
    await flushPromises()
    await w.find('[data-wt-cleanup-remove]').trigger('click')
    await flushPromises()
    expect(request).toHaveBeenCalledWith({ t: 'wt.remove', cwd: 'C:\\w\\main', path: 'C:\\w\\wt', deleteBranch: true }, 130_000)
    expect(w.find('[data-wt-cleanup-result]').text()).toContain('Removed the worktree and branch feat/x.')
    await w.find('[data-wt-cleanup-close]').trigger('click')
    expect(w.emitted('done')).toEqual([['removed']])
  })

  it('shows a dirty worktree as blocked and offers only Close', async () => {
    wtCleanup.mockResolvedValue({ ok: false, error: 'dirty' })
    const w = open()
    await flushPromises()
    expect(w.find('[data-wt-cleanup-remove]').exists()).toBe(false)
    expect(w.find('[data-wt-cleanup-result]').text()).toContain('uncommitted changes')
    expect(request).not.toHaveBeenCalled()
  })

  it('tells the owner when git kept the branch because it is not merged', async () => {
    wtCleanup.mockResolvedValue({ ok: true, path: 'C:\\w\\wt', cleanup: [] })
    request.mockResolvedValue({ t: 'wt.removed', path: 'C:\\w\\wt', branch: 'kept', branchDetail: 'not fully merged' })
    const w = open()
    await flushPromises()
    await w.find('[data-wt-cleanup-remove]').trigger('click')
    await flushPromises()
    expect(w.find('[data-wt-cleanup-result]').text()).toContain('Branch feat/x stays')
  })
})
