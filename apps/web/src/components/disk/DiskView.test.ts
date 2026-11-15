import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import diskV101 from '../../i18n/disk-v101'
import type { DiskChildren, DiskEntry, DiskStatus } from '../../lib/diskMap'

const flags = vi.hoisted(() => ({ supported: true }))
const api = vi.hoisted(() => ({
  status: vi.fn(),
  volumes: vi.fn(),
  scan: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  cancel: vi.fn(),
  children: vi.fn(),
  del: vi.fn(),
  history: vi.fn(),
  reveal: vi.fn(),
}))

vi.mock('../../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../../lib/diskMap')>()),
  get hasDiskMap() {
    return flags.supported
  },
  diskMapStatus: () => api.status(),
  diskMapVolumes: () => api.volumes(),
  diskMapScan: (...a: unknown[]) => api.scan(...a),
  diskMapPause: () => api.pause(),
  diskMapResume: () => api.resume(),
  diskMapCancel: () => api.cancel(),
  diskMapChildren: (...a: unknown[]) => api.children(...a),
  diskMapDelete: (...a: unknown[]) => api.del(...a),
  diskMapHistory: () => api.history(),
  diskMapReveal: (p: string) => api.reveal(p),
  onDiskMapProgress: () => () => {},
  onDiskMapDone: () => () => {},
}))

import DiskView from './DiskView.vue'
import { useDiskMapStore } from '../../stores/diskMap'

const entry = (name: string, size: number, dir = false): DiskEntry => ({ path: `C:\\r\\${name}`, name, size, dir, mtime: 1 })
const listing = (path: string, items: DiskEntry[]): DiskChildren => ({ path, size: items.reduce((s, i) => s + i.size, 0), total: items.length, items })
const status = (over: Partial<DiskStatus> = {}): DiskStatus => ({ scanning: false, paused: false, indexed: true, roots: ['C:\\r'], scannedAt: 1_700_000_000_000, entries: 1234, ...over })

const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en: diskV101.en, vi: diskV101.vi } })

let wrapper: VueWrapper | null = null

async function mountView(): Promise<VueWrapper> {
  const pinia = createPinia()
  setActivePinia(pinia)
  wrapper = mount(DiskView, { global: { plugins: [pinia, i18n] }, attachTo: document.body })
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  flags.supported = true
  localStorage.clear()
  HTMLCanvasElement.prototype.getContext = (() => null) as unknown as typeof HTMLCanvasElement.prototype.getContext
  for (const f of Object.values(api)) f.mockReset()
  api.status.mockResolvedValue(status())
  api.volumes.mockResolvedValue([{ mount: 'C:\\r', label: 'System', fs: 'NTFS', kind: 'fixed', network: false, total: 1000, free: 400 }])
  api.history.mockResolvedValue([])
  api.children.mockImplementation(async (p: string) => listing(p, [entry('big', 3000, true), entry('mid.zip', 2000), entry('small.txt', 100)]))
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
})

describe('DiskView', () => {
  it('shows a notice and no controls when the bridge is missing', async () => {
    flags.supported = false
    const w = await mountView()
    expect(w.find('[data-disk-unsupported]').exists()).toBe(true)
    expect(w.find('[data-disk-scanbar]').exists()).toBe(false)
  })

  it('lists volumes, opens the first root and renders the tree, crumbs and legend', async () => {
    const w = await mountView()
    expect(w.findAll('[data-disk-volume]')).toHaveLength(1)
    expect(w.findAll('[data-disk-row]').map((r) => r.attributes('data-path'))).toEqual(['C:\\r\\big', 'C:\\r\\mid.zip', 'C:\\r\\small.txt'])
    expect(w.find('[data-disk-crumbs]').text()).toContain('r')
    expect(w.find('[data-disk-treemap]').exists()).toBe(true)
    expect(w.findAll('[data-disk-legend] li')).toHaveLength(7)
    expect(w.find('[data-disk-total]').text()).toBe('4.98 KB')
  })

  it('switches to the sunburst and remembers it', async () => {
    const w = await mountView()
    await w.find('[data-disk-chart] [role=radio]:last-child').trigger('click')
    expect(w.find('[data-disk-sunburst]').exists()).toBe(true)
    expect(localStorage.getItem('ccm.diskChart')).toBe('sunburst')
  })

  it('enters a folder from the tree and goes back up', async () => {
    const w = await mountView()
    await w.find('[data-disk-row][data-path="C:\\\\r\\\\big"] button').trigger('click')
    await flushPromises()
    expect(api.children).toHaveBeenCalledWith('C:\\r\\big', 2000)
    expect(useDiskMapStore().path).toBe('C:\\r\\big')
    await w.find('[data-disk-up]').trigger('click')
    await flushPromises()
    expect(useDiskMapStore().path).toBe('C:\\r')
  })

  it('shows the selection total and sends a trash request with the selected paths', async () => {
    api.del.mockResolvedValue({ done: [{ path: 'C:\\r\\mid.zip', size: 2000 }], failed: [], freed: 2000, permanent: false })
    const w = await mountView()
    await w.find('[data-disk-row][data-path="C:\\\\r\\\\mid.zip"] input').trigger('click')
    expect(w.find('[data-disk-selection]').text()).toContain('1 selected')
    await w.find('[data-disk-trash]').trigger('click')
    await flushPromises()
    expect(api.del).toHaveBeenCalledWith(['C:\\r\\mid.zip'], false)
    expect(w.find('[data-disk-report]').text()).toContain('Freed')
  })

  it('asks for a permanent delete only through the secondary button and lists partial failures', async () => {
    api.del.mockResolvedValue({ done: [], failed: [{ path: 'C:\\r\\big', reason: 'in use' }], freed: 0, permanent: true })
    const w = await mountView()
    await w.find('[data-disk-row][data-path="C:\\\\r\\\\big"] input').trigger('click')
    await w.find('[data-disk-delete]').trigger('click')
    await flushPromises()
    expect(api.del).toHaveBeenCalledWith(['C:\\r\\big'], true)
    expect(w.find('[data-disk-report]').text()).toContain('1 failed')
    await w.find('[data-disk-report-more]').trigger('click')
    expect(w.find('[data-disk-report]').text()).toContain('in use')
  })

  it('keeps the selection bar hidden until something is selected', async () => {
    const w = await mountView()
    expect(w.find('[data-disk-selection]').exists()).toBe(false)
  })

  it('says so for a folder of 0 bytes and for a missing listing', async () => {
    api.children.mockImplementation(async (p: string) => listing(p, [entry('empty', 0, true)]))
    const w = await mountView()
    expect(w.find('[data-disk-zero]').exists()).toBe(true)
    api.children.mockResolvedValue(null)
    await useDiskMapStore().open('C:\\r\\gone')
    await flushPromises()
    expect(w.find('[data-disk-empty]').exists()).toBe(true)
  })

  it('shows progress with pause and cancel while scanning', async () => {
    api.status.mockResolvedValue(status({ scanning: true, indexed: false, roots: [] }))
    const w = await mountView()
    const store = useDiskMapStore()
    store.progress = { files: 1500, bytes: 5 * 1024 ** 3, current: 'C:\\r\\deep\\file.bin', elapsedMs: 65_000 }
    await flushPromises()
    const p = w.find('[data-disk-progress]')
    expect(p.text()).toContain('1,500 files')
    expect(p.text()).toContain('5 GB')
    expect(p.text()).toContain('1:05')
    expect(w.find('[data-disk-pause]').exists()).toBe(true)
    expect(w.find('[data-disk-cancel]').exists()).toBe(true)
    expect(w.find('[data-disk-scan]').exists()).toBe(false)
  })

  it('starts a scan for the ticked drives at the chosen level', async () => {
    api.status.mockResolvedValue(status({ indexed: false, roots: [] }))
    api.scan.mockResolvedValue(true)
    const w = await mountView()
    expect(w.find('[data-disk-start]').text()).toContain('No index yet')
    await w.find('[data-disk-root] input').setValue(true)
    await w.find('[data-disk-level] [role=radio]:first-child').trigger('click')
    await w.find('[data-disk-scan]').trigger('click')
    await flushPromises()
    expect(api.scan).toHaveBeenCalledWith(['C:\\r'], 'light')
  })

  it('shows the clean-up history and the total freed', async () => {
    api.history.mockResolvedValue([
      { at: 1_700_000_000_000, action: 'trash', count: 3, freed: 1024 ** 3, paths: ['C:\\r\\a'], failed: 0 },
      { at: 1_700_000_100_000, action: 'delete', count: 1, freed: 1024 ** 3, paths: ['C:\\r\\b'], failed: 1 },
    ])
    const w = await mountView()
    expect(w.find('[data-disk-freed]').text()).toContain('2 GB')
    expect(w.findAll('[data-disk-history] li')).toHaveLength(2)
  })

  it('shows an empty history message', async () => {
    const w = await mountView()
    expect(w.find('[data-disk-history-empty]').exists()).toBe(true)
  })
})
