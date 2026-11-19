import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import type { DiskDone, DiskStatus, DiskVolume } from '../lib/diskMap'
import { fastScanDrive, fastScanError } from '../lib/diskMap'
import diskV101 from '../i18n/disk-v101'

const api = vi.hoisted(() => ({
  status: vi.fn(),
  volumes: vi.fn(),
  fast: vi.fn(),
  scan: vi.fn(),
  done: null as ((d: DiskDone | null) => void) | null,
}))

vi.mock('../lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMap')>()),
  hasDiskMap: true,
  hasFastScan: true,
  diskMapStatus: () => api.status(),
  diskMapVolumes: () => api.volumes(),
  diskMapHistory: async () => [],
  diskMapChildren: async () => null,
  diskMapScan: (...a: unknown[]) => api.scan(...a),
  diskMapFastScan: (...a: unknown[]) => api.fast(...a),
  onDiskMapProgress: () => () => {},
  onDiskMapDone: (cb: (d: DiskDone | null) => void) => {
    api.done = cb
    return () => {}
  },
}))

import { useDiskMapStore } from './diskMap'
import DiskScanBar from '../components/disk/DiskScanBar.vue'

const vol = (mount: string, fs: string, extra: Partial<DiskVolume> = {}): DiskVolume => ({ mount, label: '', fs, kind: 'fixed', network: false, total: 100, free: 50, ...extra })
const idle = (scanning = false): DiskStatus => ({ scanning, paused: false, indexed: false, roots: [] })

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  for (const f of [api.status, api.volumes, api.fast, api.scan]) f.mockReset()
  api.done = null
  api.status.mockResolvedValue(idle())
  api.volumes.mockResolvedValue([vol('C:\\', 'NTFS'), vol('D:\\', 'exFAT', { kind: 'removable' }), vol('\\\\nas\\share', 'NTFS', { kind: 'network', network: true })])
})

describe('fast scan helpers', () => {
  it('offers only local NTFS drive roots', () => {
    expect(fastScanDrive(vol('C:\\', 'NTFS'))).toBe('C')
    expect(fastScanDrive(vol('e:', 'NTFS', { kind: 'removable' }))).toBe('E')
    expect(fastScanDrive(vol('D:\\', 'ReFS'))).toBeNull()
    expect(fastScanDrive(vol('F:\\', 'FAT32'))).toBeNull()
    expect(fastScanDrive(vol('Z:\\', 'NTFS', { kind: 'network', network: true }))).toBeNull()
    expect(fastScanDrive(vol('G:\\', 'NTFS', { kind: 'cdrom' }))).toBeNull()
    expect(fastScanDrive(vol('C:\\mnt\\disk', 'NTFS'))).toBeNull()
    expect(fastScanDrive(vol('/', 'apfs'))).toBeNull()
  })

  it('maps core errors to messages', () => {
    expect(fastScanError('elevation-declined')).toBe('declined')
    expect(fastScanError('unsupported')).toBe('unsupported')
    expect(fastScanError('unavailable')).toBe('unsupported')
    expect(fastScanError('busy')).toBe('busy')
    expect(fastScanError('helper exited')).toBe('failed')
    expect(fastScanError(undefined)).toBe('failed')
  })
})

describe('fast scan store', () => {
  it('needs exactly one NTFS drive selected', async () => {
    const s = useDiskMapStore()
    await s.init()
    expect(s.fastDrive).toBeNull()
    s.toggleRoot('C:\\')
    expect(s.fastDrive).toBe('C')
    s.toggleRoot('D:\\')
    expect(s.fastDrive).toBeNull()
    s.toggleRoot('C:\\')
    expect(s.fastDrive).toBeNull()
    s.toggleRoot('D:\\')
    s.toggleRoot('\\\\nas\\share')
    expect(s.fastDrive).toBeNull()
    expect(await s.fastScan()).toBe(false)
    expect(api.fast).not.toHaveBeenCalled()
  })

  it('starts, then clears the running flag on done', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggleRoot('C:\\')
    api.fast.mockResolvedValue({ started: true })
    api.status.mockResolvedValue(idle(true))
    expect(await s.fastScan()).toBe(true)
    expect(api.fast).toHaveBeenCalledWith('C')
    expect(s.fastRunning).toBe(true)
    api.status.mockResolvedValue(idle())
    api.done!({ ok: true, mft: true, entries: 3 })
    expect(s.fastRunning).toBe(false)
    expect(s.fastError).toBeNull()
  })

  it('reports a declined UAC prompt and a refused start', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggleRoot('C:\\')
    api.fast.mockResolvedValue({ started: true })
    await s.fastScan()
    api.done!({ ok: false, error: 'elevation-declined', mft: true })
    expect(s.fastError).toBe('declined')
    api.fast.mockResolvedValue({ started: false, error: 'busy' })
    expect(await s.fastScan()).toBe(false)
    expect(s.fastError).toBe('busy')
    expect(s.progress).toBeNull()
    api.fast.mockResolvedValue({ started: true })
    await s.fastScan()
    api.done!({ ok: false, cancelled: true, mft: true })
    expect(s.fastError).toBeNull()
    api.scan.mockResolvedValue(true)
    s.fastError = 'failed'
    await s.scan()
    expect(s.fastError).toBeNull()
  })
})

describe('fast scan button', () => {
  const i18n = () => createI18n({ legacy: false, locale: 'vi', messages: { en: diskV101.en, vi: diskV101.vi } })

  it('shows the button and the admin note only for a selected NTFS drive', async () => {
    const s = useDiskMapStore()
    await s.init()
    const w = mount(DiskScanBar, { global: { plugins: [i18n()] } })
    expect(w.find('[data-disk-fastscan]').exists()).toBe(false)
    s.toggleRoot('C:\\')
    await w.vm.$nextTick()
    expect(w.find('[data-disk-fastscan]').text()).toContain('Quét siêu nhanh')
    expect(w.find('[data-disk-fasthint]').text()).toContain('quyền admin')
    expect(w.find('[data-disk-fasthint]').text()).toContain('C:')
    s.toggleRoot('C:\\')
    s.toggleRoot('D:\\')
    await w.vm.$nextTick()
    expect(w.find('[data-disk-fastscan]').exists()).toBe(false)
    w.unmount()
  })

  it('hides pause while reading the file table and shows a declined note after', async () => {
    const s = useDiskMapStore()
    await s.init()
    s.toggleRoot('C:\\')
    api.fast.mockResolvedValue({ started: true })
    api.status.mockResolvedValue(idle(true))
    const w = mount(DiskScanBar, { global: { plugins: [i18n()] } })
    await w.find('[data-disk-fastscan]').trigger('click')
    await vi.waitFor(() => expect(s.fastRunning).toBe(true))
    s.progress = { files: 1234, bytes: 0, current: 'C:\\', elapsedMs: 2000 }
    await w.vm.$nextTick()
    expect(w.find('[data-disk-pause]').exists()).toBe(false)
    expect(w.find('[data-disk-cancel]').exists()).toBe(true)
    expect(w.find('[data-disk-progress]').text()).toContain('bản ghi')
    api.status.mockResolvedValue(idle())
    api.done!({ ok: false, error: 'elevation-declined', mft: true })
    await vi.waitFor(() => expect(s.scanning).toBe(false))
    await w.vm.$nextTick()
    expect(w.find('[data-disk-fasterror]').text()).toContain('quyền admin')
    w.unmount()
  })
})
