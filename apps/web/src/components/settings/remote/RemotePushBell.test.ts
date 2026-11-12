import { afterEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import type { RemoteDevice } from '@ccm/shared'
import { i18n } from '../../../i18n'
import RemotePushBell from './RemotePushBell.vue'

let w: VueWrapper | null = null
afterEach(() => {
  w?.unmount()
  w = null
})

const device = (id: string, extra: Partial<RemoteDevice> = {}): RemoteDevice => ({ deviceId: id, name: `Phone ${id}`, platform: 'web', pairedAt: 1, lastSeen: 1, state: 'idle', ...extra })

const devices = [device('a', { push: 'web', pushOn: true, pushContent: true }), device('b', { push: 'fcm', pushOn: false, platform: 'android' }), device('c', { push: 'none', pushOn: true })]

const mountBell = (list: RemoteDevice[], locale: 'en' | 'vi' = 'en'): VueWrapper => {
  i18n.global.locale.value = locale
  w = mount(RemotePushBell, { props: { devices: list }, attachTo: document.body, global: { plugins: [i18n] } })
  return w
}

describe('RemotePushBell', () => {
  it('shows how many phones get push and opens the list on click', async () => {
    const bell = mountBell(devices)
    expect(bell.get('[data-remote-push-count]').text()).toBe('1 of 3 on')
    expect(bell.find('[data-remote-push-list]').exists()).toBe(false)
    await bell.get('[data-remote-push-toggle]').trigger('click')
    expect(bell.get('[data-remote-push-toggle]').attributes('aria-expanded')).toBe('true')
    expect(bell.findAll('[data-remote-push-device]')).toHaveLength(3)
  })

  it('gives every phone a state with icon, text and colour', async () => {
    const bell = mountBell(devices)
    await bell.get('[data-remote-push-toggle]').trigger('click')
    const row = (id: string) => bell.get(`[data-remote-push-device="${id}"]`)
    expect(row('a').attributes('data-push-state')).toBe('on')
    expect(row('a').get('[data-remote-push-state]').text()).toBe('Push on · Web Push · with content')
    expect(row('a').get('[data-remote-push-state]').classes()).toContain('text-success')
    expect(row('a').find('[data-remote-push-state] svg').exists()).toBe(true)
    expect(row('b').get('[data-remote-push-state]').text()).toBe('Off on this computer')
    expect(row('b').get('[data-remote-push-state]').classes()).toContain('text-ink-muted')
    expect(row('c').get('[data-remote-push-state]').text()).toBe('Not turned on in the phone')
    expect(row('c').get('[data-remote-push-state]').classes()).toContain('text-warning')
  })

  it('flips one phone and names it for screen readers', async () => {
    const bell = mountBell(devices)
    await bell.get('[data-remote-push-toggle]').trigger('click')
    const a = bell.get('[data-remote-push-device="a"] [data-remote-push-switch]')
    expect(a.attributes('aria-checked')).toBe('true')
    expect(a.attributes('aria-label')).toBe('Push to Phone a')
    await a.trigger('click')
    await bell.get('[data-remote-push-device="b"] [data-remote-push-switch]').trigger('click')
    expect(bell.emitted('toggle')).toEqual([
      ['a', false],
      ['b', true],
    ])
  })

  it('closes on Escape and returns focus to the bell, in Vietnamese too', async () => {
    const bell = mountBell(devices, 'vi')
    expect(bell.get('[data-remote-push-count]').text()).toBe('Bật 1 / 3')
    await bell.get('[data-remote-push-toggle]').trigger('click')
    await bell.get('[data-remote-push-list]').trigger('keydown', { key: 'Escape' })
    expect(bell.find('[data-remote-push-list]').exists()).toBe(false)
    expect(document.activeElement).toBe(bell.get('[data-remote-push-toggle]').element)
    i18n.global.locale.value = 'en'
  })
})
