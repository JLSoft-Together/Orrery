import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import diskV101 from '../i18n/disk-v101'
import diskV77 from '../i18n/disk-v77'
import type { DiskSchedule } from '../lib/diskSnap'

const api = vi.hoisted(() => ({
  sget: vi.fn(),
  sset: vi.fn(),
  schedCb: null as null | ((s: unknown) => void),
  diskSet: vi.fn(),
  diskState: vi.fn(),
}))

vi.mock('../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMap')>()),
  hasDiskMap: true,
  diskMapStatus: async () => ({ scanning: false, paused: false, indexed: true, roots: ['C:\\r'], entries: 10 }),
  diskMapVolumes: async () => [],
  diskMapHistory: async () => [],
  diskMapChildren: async () => null,
  onDiskMapProgress: () => () => {},
  onDiskMapDone: () => () => {},
}))
vi.mock('../lib/diskSnap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskSnap')>()),
  hasDiskSnap: true,
  hasDiskSchedule: true,
  diskSnapList: async () => [],
  diskScheduleGet: () => api.sget(),
  diskScheduleSet: (...a: unknown[]) => api.sset(...a),
  onDiskSnapNew: () => () => {},
  onDiskSchedule: (cb: (s: unknown) => void) => {
    api.schedCb = cb
    return () => {}
  },
}))
vi.mock('../lib/buildClean', async (orig) => ({
  ...(await orig<typeof import('../lib/buildClean')>()),
  hasDisk: true,
  diskState: () => api.diskState(),
  diskSet: (...a: unknown[]) => api.diskSet(...a),
  diskRefresh: async () => null,
  onDisk: () => () => {},
}))

import SettingsDiskMap from './SettingsDiskMap.vue'

const sched = (over: Partial<DiskSchedule> = {}): DiskSchedule => ({ freq: 'daily', telegram: true, last: null, running: false, paused: false, ...over })
const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en: { ...diskV101.en, ...diskV77.en }, vi: { ...diskV101.vi, ...diskV77.vi } } })
let wrapper: VueWrapper | null = null

async function mountSettings(): Promise<VueWrapper> {
  const pinia = createPinia()
  setActivePinia(pinia)
  const w = mount(SettingsDiskMap, { global: { plugins: [pinia, i18n], stubs: { DiskVolumeRow: true } }, attachTo: document.body })
  await flushPromises()
  wrapper = w
  return w
}

beforeEach(() => {
  api.sget.mockReset().mockResolvedValue(sched())
  api.sset.mockReset().mockImplementation(async (patch: object) => sched(patch))
  api.diskSet.mockReset().mockImplementation(async (gb: number) => ({ warnGb: gb, at: 1, drives: [] }))
  api.diskState.mockReset().mockResolvedValue({ warnGb: 10, at: 1, drives: [] })
})
afterEach(() => {
  wrapper?.unmount()
  wrapper = null
})

const radio = (w: VueWrapper, root: string, label: string) => w.findAll(`${root} [role="radio"]`).find((b) => b.text() === label)!

describe('Settings > Data > Disk', () => {
  it('shows the background scan frequency with daily selected and no scan yet', async () => {
    const w = await mountSettings()
    expect(w.find('[data-disk-schedule]').exists()).toBe(true)
    expect(radio(w, '[data-disk-schedule-freq]', 'Daily').attributes('aria-checked')).toBe('true')
    expect(w.find('[data-disk-schedule-last]').text()).toBe('No background scan yet')
  })

  it('changing the frequency goes to main', async () => {
    const w = await mountSettings()
    await radio(w, '[data-disk-schedule-freq]', 'Weekly').trigger('click')
    await flushPromises()
    expect(api.sset).toHaveBeenCalledWith({ freq: 'weekly' })
    expect(radio(w, '[data-disk-schedule-freq]', 'Weekly').attributes('aria-checked')).toBe('true')
    await radio(w, '[data-disk-schedule-freq]', 'Off').trigger('click')
    expect(api.sset).toHaveBeenLastCalledWith({ freq: 'off' })
  })

  it('shows the last background scan, a failure, and live progress', async () => {
    api.sget.mockResolvedValue(sched({ last: { at: Date.UTC(2026, 10, 14, 12), ok: true } }))
    const w = await mountSettings()
    expect(w.find('[data-disk-schedule-last]').text()).toMatch(/^Last background scan: .*2026/)
    api.schedCb?.(sched({ last: { at: Date.UTC(2026, 10, 14, 12), ok: false } }))
    await flushPromises()
    expect(w.find('[data-disk-schedule-last]').text()).toMatch(/failed/)
    api.schedCb?.(sched({ running: true }))
    await flushPromises()
    expect(w.find('[data-disk-schedule-last]').text()).toBe('Scanning in the background')
    api.schedCb?.(sched({ running: true, paused: true }))
    await flushPromises()
    expect(w.find('[data-disk-schedule-last]').text()).toBe('Background scan paused while the PC is in use')
  })

  it('the Telegram switch writes the setting', async () => {
    const w = await mountSettings()
    const sw = w.find('[data-disk-schedule-telegram] [role="switch"], [data-disk-schedule-telegram][role="switch"]')
    expect(sw.attributes('aria-checked')).toBe('true')
    await sw.trigger('click')
    await flushPromises()
    expect(api.sset).toHaveBeenCalledWith({ telegram: false })
  })

  it('keeps the low disk warning setting in the same section and writes through the old channel', async () => {
    const w = await mountSettings()
    expect(w.find('[data-setting-row="disk"] [data-disk-settings]').exists()).toBe(true)
    const five = radio(w, '[data-disk-threshold]', '5 GB')
    expect(radio(w, '[data-disk-threshold]', '10 GB').attributes('aria-checked')).toBe('true')
    await five.trigger('click')
    await flushPromises()
    expect(api.diskSet).toHaveBeenCalledWith(5)
  })

  it('hides the schedule when the bridge has no schedule answer', async () => {
    api.sget.mockResolvedValue(null)
    const w = await mountSettings()
    expect(w.find('[data-disk-schedule]').exists()).toBe(false)
    expect(w.find('[data-disk-schedule-telegram]').exists()).toBe(false)
    expect(w.find('[data-disk-settings]').exists()).toBe(true)
  })
})
