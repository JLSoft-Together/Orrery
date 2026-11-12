import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h, nextTick } from 'vue'
import type { RemoteGuardBlock, RemoteGuardLogLine, RemoteGuardState } from '@ccm/shared'
import { i18n } from '../../../i18n'
import { looksLikeRule, splitRules } from '../../../lib/remoteGuard'
import { useRemoteStore } from '../../../stores/remote'

const api = vi.hoisted(() => ({ remoteGuard: vi.fn() }))
vi.mock('../../../lib/remoteGuard', async (orig) => ({ ...(await orig<typeof import('../../../lib/remoteGuard')>()), ...api }))

import { useRemoteGuard } from '../../../composables/useRemoteGuard'
import RemoteAccessLog from './RemoteAccessLog.vue'
import RemoteGuardBlocked from './RemoteGuardBlocked.vue'
import RemoteGuardTrust from './RemoteGuardTrust.vue'

let w: VueWrapper | null = null

const rows: RemoteGuardBlock[] = [
  { key: '198.51.100.0/24', source: 'manual', why: 'manual', count: 0, at: 1_900_000_000, until: null, level: 0 },
  { key: '203.0.113.9', source: 'relay', why: 'ticket', count: 20, at: 1_900_000_000, until: 1_900_003_600, level: 1 },
  { key: '2001:db8:1:2::/64', source: 'lan', why: 'code', count: 21, at: 1_900_000_000, until: 1_900_086_400, level: 2 },
]

beforeEach(() => {
  setActivePinia(createPinia())
  i18n.global.locale.value = 'en'
})
afterEach(() => {
  w?.unmount()
  w = null
  vi.clearAllMocks()
})

describe('rule parsing', () => {
  it('accepts v4 / v6 addresses and ranges with a sane prefix only', () => {
    expect(['203.0.113.9', '198.51.100.0/24', '2001:db8::/32', '::1'].every(looksLikeRule)).toBe(true)
    expect(['0.0.0.0/0', '10.0.0.0/7', '2001::/8', 'example.com', '1.2.3', '1.2.3.4/33', ''].some(looksLikeRule)).toBe(false)
    expect(splitRules(' 10.0.0.1, 10.0.0.1\n192.168.0.0/16 ')).toEqual(['10.0.0.1', '192.168.0.0/16'])
  })
})

describe('RemoteGuardBlocked', () => {
  it('lists every block with where, reason, count and expiry, and unblocks one', async () => {
    w = mount(RemoteGuardBlocked, { props: { blocked: rows, busy: false, error: null }, global: { plugins: [i18n] } })
    const list = w.findAll('[data-remote-guard-row]')
    expect(list).toHaveLength(3)
    expect(list[0]!.get('[data-remote-guard-source]').text()).toBe('Manual')
    expect(list[0]!.get('[data-remote-guard-until]').text()).toBe('Permanent')
    expect(list[1]!.text()).toContain('Relay')
    expect(list[1]!.text()).toContain('Bad pairing ticket')
    expect(list[1]!.text()).toContain('Failures20')
    expect(list[2]!.text()).toContain('Step 2 of 3')
    await list[1]!.get('[data-remote-guard-unblock]').trigger('click')
    expect(w.emitted('run')).toEqual([[{ op: 'unblock', key: '203.0.113.9' }]])
  })

  it('asks before unblocking everything', async () => {
    w = mount(RemoteGuardBlocked, { props: { blocked: rows, busy: false, error: null }, global: { plugins: [i18n] } })
    await w.get('[data-remote-guard-unblock-all]').trigger('click')
    expect(w.emitted('run')).toBeUndefined()
    await w.get('[data-remote-guard-unblock-all-yes]').trigger('click')
    expect(w.emitted('run')).toEqual([[{ op: 'unblock-all' }]])
  })

  it('blocks a valid range for the chosen time and refuses a bad one in place', async () => {
    w = mount(RemoteGuardBlocked, { props: { blocked: [], busy: false, error: null }, global: { plugins: [i18n] } })
    expect(w.find('[data-remote-guard-empty]').exists()).toBe(true)
    await w.get('[data-remote-guard-cidr]').setValue('0.0.0.0/0')
    await w.get('[data-remote-guard-add]').trigger('submit')
    expect(w.emitted('run')).toBeUndefined()
    expect(w.get('[data-remote-guard-hint]').attributes('role')).toBe('alert')
    await w.get('[data-remote-guard-cidr]').setValue(' 198.51.100.0/24 ')
    const select = w.get('[data-remote-guard-ttl]').element as HTMLSelectElement
    select.selectedIndex = 4
    await w.get('[data-remote-guard-ttl]').trigger('change')
    await w.get('[data-remote-guard-add]').trigger('submit')
    expect(w.emitted('run')).toEqual([[{ op: 'block', cidr: '198.51.100.0/24', ttl: null }]])
    await w.setProps({ error: 'live' })
    expect(w.get('[data-remote-guard-hint]').text()).toContain('A paired phone is connected')
  })
})

describe('RemoteGuardTrust', () => {
  it('adds and removes trusted ranges as a whole list', async () => {
    w = mount(RemoteGuardTrust, { props: { trusted: ['10.0.0.0/8'], busy: false }, global: { plugins: [i18n] } })
    await w.get('[data-remote-guard-trust-input]').setValue('192.168.1.0/24 nope')
    expect(w.get('[data-remote-guard-trust-add]').attributes('disabled')).toBeDefined()
    await w.get('[data-remote-guard-trust-input]').setValue('192.168.1.0/24, 2001:db8::/32')
    await w.get('form').trigger('submit')
    expect(w.emitted('run')![0]).toEqual([{ op: 'trust', list: ['10.0.0.0/8', '192.168.1.0/24', '2001:db8::/32'] }])
    await w.get('[data-remote-guard-trusted="10.0.0.0/8"] button').trigger('click')
    expect(w.emitted('run')![1]).toEqual([{ op: 'trust', list: [] }])
  })
})

describe('RemoteAccessLog', () => {
  it('shows result as icon + text, ip, country, route and the device or reason', async () => {
    const log: RemoteGuardLogLine[] = [
      { at: 1_900_000_100, ip: '203.0.113.9', via: 'relay', country: 'VN', result: 'blocked', why: 'ticket' },
      { at: 1_900_000_000, ip: '192.168.1.5', via: 'lan', device: 'Pixel 9', result: 'ok', why: 'session' },
    ]
    w = mount(RemoteAccessLog, { props: { log, busy: false }, global: { plugins: [i18n] } })
    const lines = w.findAll('[data-result]')
    expect(lines[0]!.text()).toContain('Blocked')
    expect(lines[0]!.text()).toContain('203.0.113.9 [VN]')
    expect(lines[0]!.find('svg').exists()).toBe(true)
    expect(lines[1]!.text()).toContain('Pixel 9 · Session opened')
    await w.get('[data-remote-access-refresh]').trigger('click')
    expect(w.emitted('refresh')).toHaveLength(1)
  })
})

describe('useRemoteGuard', () => {
  it('reloads the state when the status carries a new guard revision', async () => {
    const state = (rev: number): RemoteGuardState => ({ blocked: [], trusted: [], log: [], rev })
    api.remoteGuard.mockResolvedValueOnce({ ok: true, state: state(2) }).mockResolvedValueOnce({ ok: false, error: 'live', state: state(3) })
    let guard: ReturnType<typeof useRemoteGuard> | null = null
    w = mount(defineComponent({ setup: () => ((guard = useRemoteGuard()), () => h('div')) }), { global: { plugins: [i18n] } })
    const remote = useRemoteStore()
    remote.status = { enabled: true, listening: true, addresses: [], port: 4320, relay: 'off', keepAwake: true, devices: [], guardRev: 2 }
    await nextTick()
    await flushPromises()
    expect(api.remoteGuard).toHaveBeenCalledWith({ op: 'get' })
    expect(guard!.state.value.rev).toBe(2)
    expect(await guard!.run({ op: 'block', cidr: '10.0.0.0/24', ttl: null })).toBe(false)
    expect(guard!.error.value).toBe('live')
    remote.status = { ...remote.status, guardRev: 3 }
    await nextTick()
    expect(api.remoteGuard).toHaveBeenCalledTimes(2)
  })
})
