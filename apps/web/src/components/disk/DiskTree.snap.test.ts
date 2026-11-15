import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import diskV101 from '../../i18n/disk-v101'
import type { DiskCompare, DiskDiffRow, DiskSnapshotMeta } from '../../lib/diskSnap'
import type { DiskEntry } from '../../lib/diskMap'

const api = vi.hoisted(() => ({ list: vi.fn(), compare: vi.fn() }))

vi.mock('../../lib/diskSnap', async (orig) => ({
  ...(await orig<typeof import('../../lib/diskSnap')>()),
  hasDiskSnap: true,
  hasDiskSchedule: false,
  diskSnapList: () => api.list(),
  diskSnapCompare: (...a: unknown[]) => api.compare(...a),
  diskScheduleGet: async () => null,
  onDiskSnapNew: () => () => {},
  onDiskSchedule: () => () => {},
}))

import DiskTree from './DiskTree.vue'
import { useDiskMapStore } from '../../stores/diskMap'
import { useDiskSnapStore } from '../../stores/diskSnap'

const GB = 1024 ** 3
const meta = (id: string, at: number): DiskSnapshotMeta => ({ id, at, bytes: 1, roots: ['C:\\r'] })
const row = (name: string, before: number, after: number, state: DiskDiffRow['state'] = 'changed'): DiskDiffRow => ({ path: `C:\\r\\${name}`, name, before, after, delta: after - before, state })
const entry = (name: string, size: number, dir = true): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir, mtime: 1 })

const items = [entry('big', 30 * GB), entry('shrunk', 5 * GB), entry('steady', 4 * GB), entry('fresh', 3 * GB), entry('movie.mkv', 2 * GB, false)]
const compare = (covered = true): DiskCompare => ({
  base: meta('s1', Date.UTC(2026, 10, 10, 12)),
  path: 'C:\\r',
  covered,
  rows: covered ? [row('big', 10 * GB, 30 * GB), row('shrunk', 9 * GB, 5 * GB), row('fresh', 0, 3 * GB, 'new')] : [],
  top: [row('big', 10 * GB, 30 * GB), row('fresh', 0, 3 * GB, 'new')],
})

const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en: diskV101.en, vi: diskV101.vi } })
let wrapper: VueWrapper | null = null

async function mountTree(): Promise<VueWrapper> {
  const pinia = createPinia()
  setActivePinia(pinia)
  const w = mount(DiskTree, { props: { items, total: 44 * GB, selected: new Set<string>(), loading: false }, global: { plugins: [pinia, i18n] }, attachTo: document.body })
  await flushPromises()
  wrapper = w
  return w
}

beforeEach(() => {
  api.list.mockReset().mockResolvedValue([meta('s2', Date.UTC(2026, 10, 14, 12)), meta('s1', Date.UTC(2026, 10, 10, 12))])
  api.compare.mockReset().mockImplementation(async () => compare())
})
afterEach(() => {
  wrapper?.unmount()
  wrapper = null
})

const cellOf = (w: VueWrapper, name: string): string => w.find(`[data-path="C:\\\\r\\\\${name}"] [data-disk-delta]`).text()

describe('DiskTree comparison', () => {
  it('shows no comparison UI before a snapshot exists', async () => {
    api.list.mockResolvedValue([])
    const w = await mountTree()
    expect(w.find('[data-disk-compare]').exists()).toBe(false)
    expect(w.find('[data-disk-delta]').exists()).toBe(false)
  })

  it('lists the snapshots in the picker and keeps the normal columns until one is chosen', async () => {
    const w = await mountTree()
    const opts = w.findAll('[data-disk-compare-base] option')
    expect(opts.map((o) => o.text())[0]).toBe('No comparison')
    expect(opts[1]!.text()).toMatch(/(11\/14|14\/11)\/2026/)
    expect(opts[2]!.text()).toMatch(/(11\/10|10\/11)\/2026/)
    expect(opts).toHaveLength(3)
    expect(w.find('[data-disk-delta]').exists()).toBe(false)
    expect(w.find('[data-disk-row] .rounded-full.bg-fill').exists()).toBe(true)
  })

  it('picking a snapshot adds the change column with signs, arrows and words', async () => {
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    expect(api.compare).toHaveBeenCalledWith('s1', '')
    expect(cellOf(w, 'big')).toContain('+20 GB')
    expect(w.find('[data-path="C:\\\\r\\\\big"] [data-disk-delta]').attributes('data-delta')).toBe('up')
    expect(cellOf(w, 'shrunk')).toContain('-4 GB')
    expect(w.find('[data-path="C:\\\\r\\\\shrunk"] [data-disk-delta]').attributes('data-delta')).toBe('down')
    expect(cellOf(w, 'fresh')).toContain('New')
    expect(cellOf(w, 'fresh')).toContain('+3 GB')
    expect(cellOf(w, 'steady')).toBe('0')
    expect(w.find('[data-path="C:\\\\r\\\\movie.mkv"] [data-disk-delta]').text()).toBe('')
    expect(w.find('[data-path="C:\\\\r\\\\big"] [data-disk-delta]').attributes('title')).toBe('Grew by 20 GB')
  })

  it('folders outside the comparison show a dash with the reason', async () => {
    api.compare.mockImplementation(async () => compare(false))
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    const cell = w.find('[data-path="C:\\\\r\\\\big"] [data-disk-delta]')
    expect(cell.text()).toBe('-')
    expect(cell.attributes('title')).toContain('deeper than 4 levels')
  })

  it('shows a message when the comparison cannot be made', async () => {
    api.compare.mockResolvedValue(null)
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    expect(w.find('[data-disk-compare-failed]').exists()).toBe(true)
    expect(w.find('[data-disk-delta]').exists()).toBe(false)
  })

  it('the header button sorts the folders by change', async () => {
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    const store = useDiskMapStore()
    store.children = { path: 'C:\\r', size: 44 * GB, total: items.length, items }
    expect(store.items.map((i) => i.name)).toEqual(['big', 'shrunk', 'steady', 'fresh', 'movie.mkv'])
    await w.find('[data-disk-delta-sort]').trigger('click')
    expect(store.items.map((i) => i.name)).toEqual(['big', 'fresh', 'steady', 'shrunk', 'movie.mkv'])
    expect(useDiskSnapStore().sortDelta).toBe(true)
    expect(w.find('[data-disk-delta-sort]').attributes('aria-pressed')).toBe('true')
  })

  it('the growth list shows the top folders and opens one', async () => {
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    await w.find('[data-disk-growth-toggle]').trigger('click')
    const rows = w.findAll('[data-disk-growth-row]')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.text()).toContain('big')
    expect(rows[0]!.text()).toContain('+20 GB')
    expect(rows[1]!.text()).toContain('New')
    expect(w.find('[data-disk-growth]').text()).toMatch(/Top 2 folders that grew since (11\/10|10\/11)\/2026/)
    await rows[0]!.trigger('click')
    expect(w.emitted('open')?.[0]).toEqual(['C:\\r\\big'])
    expect(w.find('[data-disk-growth]').exists()).toBe(false)
  })

  it('an empty growth list says so', async () => {
    api.compare.mockImplementation(async () => ({ ...compare(), top: [] }))
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    await w.find('[data-disk-growth-toggle]').trigger('click')
    expect(w.find('[data-disk-growth-empty]').text()).toContain('No folder grew')
  })

  it('choosing "No comparison" brings the normal list back', async () => {
    const w = await mountTree()
    await w.find('[data-disk-compare-base]').setValue('s1')
    await flushPromises()
    await w.find('[data-disk-compare-base]').setValue('')
    await flushPromises()
    expect(w.find('[data-disk-delta]').exists()).toBe(false)
    expect(w.find('[data-disk-growth-toggle]').exists()).toBe(false)
  })
})
