import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import type { RunHistoryEntry } from '@ccm/shared'
import { i18n } from '../../../i18n'

const api = vi.hoisted(() => ({ list: vi.fn(), read: vi.fn(), pin: vi.fn(), del: vi.fn() }))

vi.mock('../../../core/lib/desktop', () => ({
  desktop: {
    platform: 'win32',
    runHistoryList: (root: string) => api.list(root),
    runHistoryRead: (root: string, id: string, offset: number, limit: number) => api.read(root, id, offset, limit),
    runHistoryPin: (root: string, id: string, pinned: boolean) => api.pin(root, id, pinned),
    runHistoryDelete: (root: string, id: string) => api.del(root, id),
  },
}))
vi.mock('../stores/run', () => ({ useRunStore: () => ({ focusedSession: () => null }) }))

import RunHistoryDialog from './RunHistoryDialog.vue'

const entry = (id: string, over: Partial<RunHistoryEntry> = {}): RunHistoryEntry => ({
  id,
  project: '/p',
  target: 'app',
  config: 'c',
  device: null,
  start: Date.now() - 60_000,
  end: Date.now() - 50_000,
  exitCode: 0,
  lines: 3,
  bytes: 300,
  pinned: false,
  truncated: false,
  ...over,
})

let w: VueWrapper | null = null
const t = (k: string, p?: Record<string, unknown>): string => i18n.global.t(k, p ?? {})

beforeEach(() => {
  i18n.global.locale.value = 'en'
  api.list.mockReset()
  api.read.mockReset()
  api.pin.mockReset().mockResolvedValue(true)
  api.del.mockReset().mockResolvedValue(true)
})
afterEach(() => {
  w?.unmount()
  w = null
})

const mountIt = async (): Promise<VueWrapper> => {
  w = mount(RunHistoryDialog, { props: { open: true, root: '/p' }, global: { plugins: [i18n], stubs: { teleport: true } } })
  await flushPromises()
  return w
}

describe('RunHistoryDialog', () => {
  it('shows the empty state', async () => {
    api.list.mockResolvedValue([])
    const el = await mountIt()
    expect(el.get('[data-history-empty]').text()).toBe(t('runHistory.empty'))
    expect(el.find('[data-history-compare]').exists()).toBe(false)
  })

  it('lists runs with exit chips, duration and pinned mark', async () => {
    api.list.mockResolvedValue([entry('b', { exitCode: 1 }), entry('a', { pinned: true }), entry('r', { end: null, exitCode: null })])
    const el = await mountIt()
    const rows = el.findAll('[data-history-row]')
    expect(rows.map((r) => r.attributes('data-history-row'))).toEqual(['b', 'a', 'r'])
    expect(el.get('[data-history-row="b"] [data-history-exit]').attributes('data-history-exit')).toBe('error')
    expect(el.get('[data-history-row="b"] [data-history-exit]').text()).toBe(t('runHistory.exit', { code: 1 }))
    expect(el.get('[data-history-row="a"] [data-history-exit]').attributes('data-history-exit')).toBe('ok')
    expect(el.find('[data-history-row="a"] [data-history-pinned]').exists()).toBe(true)
    expect(el.get('[data-history-row="r"] [data-history-exit]').attributes('data-history-exit')).toBe('running')
    expect(el.get('[data-history-row="r"] [data-history-delete]').attributes('disabled')).toBeDefined()
    expect(el.get('[data-history-row="b"]').text()).toContain('10 s')
  })

  it('pin and delete call main and reload the list', async () => {
    api.list.mockResolvedValue([entry('b'), entry('a')])
    const el = await mountIt()
    await el.get('[data-history-row="a"] [data-history-pin]').trigger('click')
    await flushPromises()
    expect(api.pin).toHaveBeenCalledWith('/p', 'a', true)
    await el.get('[data-history-row="b"] [data-history-delete]').trigger('click')
    await flushPromises()
    expect(api.del).toHaveBeenCalledWith('/p', 'b')
    expect(api.list).toHaveBeenCalledTimes(3)
  })

  it('compares the pinned baseline with the newest run', async () => {
    api.list.mockResolvedValue([entry('new', { exitCode: 0, lines: 2 }), entry('mid'), entry('base', { pinned: true, exitCode: 1, lines: 2 })])
    api.read.mockImplementation(async (_r: string, id: string) => ({
      id,
      offset: 0,
      total: 2,
      lines: id === 'base' ? [{ stream: 'app', text: 'same', at: 1 }, { stream: 'app', text: 'error: boom', at: 2 }] : [{ stream: 'app', text: 'same', at: 1 }, { stream: 'app', text: 'ok done', at: 2 }],
    }))
    const el = await mountIt()
    await el.get('[data-history-compare]').trigger('click')
    await new Promise((r) => setTimeout(r, 40))
    await flushPromises()
    expect(api.read.mock.calls.map((c) => c[1]).sort()).toEqual(['base', 'new'])
    expect(el.get('[data-compare-exit]').text()).toContain(t('runHistory.compare.exit', { old: 1, now: 0 }))
    expect(el.get('[data-compare-side="old"]').text()).toContain('error: boom')
    expect(el.get('[data-compare-side="new"]').text()).toContain('ok done')
    expect(el.get('[data-compare-side="old"]').text()).not.toContain('same')
  })
})
