import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { i18n } from '../i18n'
import type { LayoutInstallStatus } from '../lib/layoutPreview'

const api = vi.hoisted(() => ({ status: vi.fn(), download: vi.fn(), uninstall: vi.fn(), handler: null as null | ((e: unknown) => void) }))
vi.mock('../lib/layoutPreview', async (orig) => ({
  ...(await orig<typeof import('../lib/layoutPreview')>()),
  layoutInstallStatus: api.status,
  layoutDownload: api.download,
  layoutUninstall: api.uninstall,
  onLayoutEvent: (h: (e: unknown) => void) => {
    api.handler = h
    return () => undefined
  },
}))

import XmlPreviewSettings from './XmlPreviewSettings.vue'

let w: VueWrapper | null = null
const t = (k: string, p?: Record<string, unknown>): string => i18n.global.t(k, p ?? {})
const MB = 1024 * 1024
const missing: LayoutInstallStatus = { available: true, installed: false, version: null, pinnedVersion: '1', sizeBytes: 0, downloadBytes: 190 * MB, bundle: { state: 'missing' } }
const ready: LayoutInstallStatus = { available: true, installed: true, version: '1', pinnedVersion: '1', sizeBytes: 250 * MB, downloadBytes: 0, bundle: { state: 'ready', dir: '/x' } }

beforeEach(() => {
  i18n.global.locale.value = 'en'
  api.status.mockReset()
  api.download.mockReset()
  api.uninstall.mockReset()
})
afterEach(() => {
  w?.unmount()
  w = null
})

const mountIt = async (): Promise<VueWrapper> => {
  w = mount(XmlPreviewSettings, { props: { open: true }, global: { plugins: [i18n] } })
  await flushPromises()
  return w
}

describe('XmlPreviewSettings', () => {
  it('not installed: shows the state and sizes, installs only on the button, remove is disabled', async () => {
    api.status.mockResolvedValueOnce(missing).mockResolvedValueOnce(ready)
    api.download.mockResolvedValue({ state: 'ready', dir: '/x' })
    const el = await mountIt()
    expect(el.get('[data-xml-state]').text()).toBe(t('xmlPreview.notInstalled'))
    expect(el.get('[data-xml-remove]').attributes('disabled')).toBeDefined()
    expect(api.download).not.toHaveBeenCalled()
    await el.get('[data-xml-install]').trigger('click')
    await flushPromises()
    expect(api.download).toHaveBeenCalledTimes(1)
    expect(el.get('[data-xml-state]').text()).toContain('1')
    expect(el.get('[data-xml-size]').text()).toBe(t('xmlPreview.sizeValue', { mb: 250 }))
    expect(el.find('[data-xml-install]').exists()).toBe(false)
  })

  it('shows progress from layout events while downloading', async () => {
    api.status.mockResolvedValue({ ...missing, bundle: { state: 'downloading', pct: 12 } })
    const el = await mountIt()
    api.handler?.({ type: 'bundle', status: { state: 'downloading', pct: 40 } })
    await flushPromises()
    expect(el.get('[data-xml-progress]').attributes('aria-valuenow')).toBe('40')
    expect(el.get('[data-xml-install]').attributes('disabled')).toBeDefined()
  })

  it('removes and shows the status the main process sends back', async () => {
    api.status.mockResolvedValue(ready)
    api.uninstall.mockResolvedValue({ ok: true, status: { ...missing, downloadBytes: 0 } })
    const el = await mountIt()
    await el.get('[data-xml-remove]').trigger('click')
    await flushPromises()
    expect(api.uninstall).toHaveBeenCalledTimes(1)
    expect(el.get('[data-xml-state]').text()).toBe(t('xmlPreview.notInstalled'))
  })

  it('a kit that is not published says so and offers no install', async () => {
    api.status.mockResolvedValue({ ...missing, available: false, pinnedVersion: null })
    const el = await mountIt()
    expect(el.get('[data-xml-state]').text()).toBe(t('xmlPreview.unavailable'))
    expect(el.find('[data-xml-install]').exists()).toBe(false)
  })

  it('an old kit offers an update and a failed remove says why', async () => {
    api.status.mockResolvedValue({ ...missing, version: '3.5.0', sizeBytes: 5 * MB })
    api.uninstall.mockResolvedValue({ ok: false, code: 'io', status: { ...missing, version: '3.5.0', sizeBytes: 5 * MB } })
    const el = await mountIt()
    expect(el.get('[data-xml-state]').text()).toBe(t('xmlPreview.updateAvailable', { version: '3.5.0', pinned: '1' }))
    expect(el.get('[data-xml-install]').text()).toBe(t('xmlPreview.update'))
    await el.get('[data-xml-remove]').trigger('click')
    await flushPromises()
    expect(el.text()).toContain(t('xmlPreview.removeFailed', { code: 'io' }))
  })

  it('renders nothing without a desktop answer', async () => {
    api.status.mockResolvedValue(null)
    const el = await mountIt()
    expect(el.find('[data-xml-preview-settings]').exists()).toBe(false)
  })
})
