import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import type { RemoteStatus } from '@ccm/shared'
import { i18n } from '../../../i18n'
import { useRemoteStore } from '../../../stores/remote'

const api = vi.hoisted(() => ({ remotePairStart: vi.fn(), remotePairCancel: vi.fn() }))

vi.mock('../../../lib/remote', async (orig) => ({ ...(await orig<typeof import('../../../lib/remote')>()), ...api }))

import RemotePairPanel from './RemotePairPanel.vue'

let w: VueWrapper | null = null

const status = (extra: Partial<RemoteStatus>): RemoteStatus => ({ enabled: true, listening: true, addresses: ['192.168.1.2'], port: 4320, relay: 'online', keepAwake: false, devices: [], ...extra })

async function mountPanel(relay: boolean): Promise<VueWrapper> {
  i18n.global.locale.value = 'en'
  api.remotePairStart.mockResolvedValue({ qr: 'eyJ2IjoyfQ', exp: Math.floor(Date.now() / 1000) + 120, relay, lan: 1 })
  api.remotePairCancel.mockResolvedValue(true)
  w = mount(RemotePairPanel, { attachTo: document.body, global: { plugins: [i18n] } })
  await flushPromises()
  await vi.dynamicImportSettled()
  await flushPromises()
  return w
}

beforeEach(() => setActivePinia(createPinia()))
afterEach(() => {
  w?.unmount()
  w = null
  vi.clearAllMocks()
})

describe('RemotePairPanel QR v2', () => {
  it('says the QR includes the relay and counts down from 2 minutes', async () => {
    const panel = await mountPanel(true)
    const route = panel.get('[data-remote-pair-route]')
    expect(route.attributes('data-relay')).toBe('true')
    expect(route.text()).toBe('Includes the relay: the phone can pair from another network')
    expect(panel.get('[data-remote-countdown]').text()).toMatch(/^(2:00|1:59)$/)
    expect(panel.get('[data-remote-pair-status]').text()).toContain('Waiting for the phone')
  })

  it('says LAN only when the relay is not in the QR', async () => {
    const panel = await mountPanel(false)
    expect(panel.get('[data-remote-pair-route]').attributes('data-relay')).toBe('false')
    expect(panel.get('[data-remote-pair-route]').text()).toBe('LAN only: the phone must be on the same Wi-Fi')
  })

  it('shows the approve step when the code is used and the cancel after 5 wrong tries', async () => {
    const panel = await mountPanel(true)
    const remote = useRemoteStore()
    remote.status = status({ pairing: { exp: 1, relay: true }, pairEnded: null })
    await flushPromises()
    expect(panel.get('[data-remote-pair-status]').text()).toContain('Waiting for the phone')
    remote.status = status({ pairing: null, pairEnded: null })
    await flushPromises()
    expect(panel.get('[data-remote-pair-status]').text()).toContain('Approve the phone in the dialog on this computer')
    panel.unmount()
    w = null
    const again = await mountPanel(true)
    useRemoteStore().status = status({ pairing: null, pairEnded: 'tries' })
    await flushPromises()
    expect(again.get('[data-remote-pair-status]').text()).toContain('Code cancelled after 5 wrong tries')
    expect(again.find('[data-remote-pair-retry]').exists()).toBe(true)
  })

  it('a code that ran out shows expired, not the approve step, when pairing goes null', async () => {
    const panel = await mountPanel(true)
    const later = Date.now() + 121_000
    const spy = vi.spyOn(Date, 'now').mockReturnValue(later)
    useRemoteStore().status = status({ pairing: null, pairEnded: null })
    await flushPromises()
    spy.mockRestore()
    expect(panel.get('[data-remote-pair-status]').text()).toContain('The code expired')
  })
})
