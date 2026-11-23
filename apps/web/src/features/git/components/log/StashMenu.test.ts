import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { i18n } from '../../../../i18n'
import StashMenu from './StashMenu.vue'

const stashList = vi.fn()
const stashRun = vi.fn()

vi.mock('../../../../core/lib/preview', async (orig) => ({
  ...(await orig<Record<string, unknown>>()),
  hasStash: true,
  stashList: (...a: unknown[]) => stashList(...a),
  stashRun: (...a: unknown[]) => stashRun(...a),
}))

const entry = (over: Record<string, unknown> = {}) => ({ hash: 'a'.repeat(40), index: 0, time: 1_700_000_000, untracked: false, branch: 'main', message: 'my work', ...over })

function open(props: Record<string, unknown> = {}) {
  return mount(StashMenu, { props: { root: 'C:\w\proj', active: true, ...props }, attachTo: document.body, global: { plugins: [i18n] } })
}

describe('StashMenu', () => {
  beforeEach(() => {
    stashList.mockReset()
    stashRun.mockReset()
    i18n.global.locale.value = 'en'
  })

  it('shows the count and lists stashes when opened', async () => {
    stashList.mockResolvedValue({ ok: true, stashes: [entry(), entry({ hash: 'b'.repeat(40), index: 1, message: 'older', untracked: true })] })
    const w = open()
    await flushPromises()
    expect(w.find('[data-stash-count]').text()).toBe('2')
    await w.find('[data-stash-menu-button]').trigger('click')
    await flushPromises()
    const rows = w.findAll('[data-stash-row]')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.text()).toContain('my work')
    expect(rows[1]!.text()).toContain('untracked')
  })

  it('says so when there are no stashes', async () => {
    stashList.mockResolvedValue({ ok: true, stashes: [] })
    const w = open()
    await flushPromises()
    await w.find('[data-stash-menu-button]').trigger('click')
    await flushPromises()
    expect(w.find('[data-stash-empty]').text()).toBe('No stashes')
    expect(w.find('[data-stash-count]').exists()).toBe(false)
  })

  it('runs pop by hash, tells the parent and reloads the list', async () => {
    const e = entry()
    stashList.mockResolvedValueOnce({ ok: true, stashes: [e] }).mockResolvedValueOnce({ ok: true, stashes: [e] }).mockResolvedValue({ ok: true, stashes: [] })
    stashRun.mockResolvedValue({ ok: true })
    const w = open()
    await flushPromises()
    await w.find('[data-stash-menu-button]').trigger('click')
    await flushPromises()
    await w.find('[data-stash-pop]').trigger('click')
    await flushPromises()
    expect(stashRun).toHaveBeenCalledWith('C:\w\proj', { op: 'pop', hash: e.hash })
    expect(w.emitted('changed')).toHaveLength(1)
    expect(w.find('[data-stash-note]').text()).toContain('Popped my work')
    expect(w.findAll('[data-stash-row]')).toHaveLength(0)
  })

  it('shows a conflict without treating it as success and keeps the row', async () => {
    stashList.mockResolvedValue({ ok: true, stashes: [entry()] })
    stashRun.mockResolvedValue({ ok: false, error: 'conflict' })
    const w = open()
    await flushPromises()
    await w.find('[data-stash-menu-button]').trigger('click')
    await flushPromises()
    await w.find('[data-stash-apply]').trigger('click')
    await flushPromises()
    expect(w.emitted('changed')).toBeUndefined()
    expect(w.find('[data-stash-note]').text()).toContain('conflicts')
    expect(w.findAll('[data-stash-row]')).toHaveLength(1)
  })

  it('does not report a cancelled drop as an error', async () => {
    stashList.mockResolvedValue({ ok: true, stashes: [entry()] })
    stashRun.mockResolvedValue({ ok: false, error: 'cancelled' })
    const w = open()
    await flushPromises()
    await w.find('[data-stash-menu-button]').trigger('click')
    await flushPromises()
    await w.find('[data-stash-drop]').trigger('click')
    await flushPromises()
    expect(stashRun).toHaveBeenCalledWith('C:\w\proj', { op: 'drop', hash: entry().hash })
    expect(w.find('[data-stash-note]').exists()).toBe(false)
    expect(w.emitted('changed')).toBeUndefined()
  })
})
