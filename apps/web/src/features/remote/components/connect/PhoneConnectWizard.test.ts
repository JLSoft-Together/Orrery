import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import type { AdbDevice, RemoteDevice, RemoteStatus } from '@ccm/shared'
import { now } from '../../../../core/lib/format'
import messages from '../../i18n/v113-connect'
import type { ConnectApi } from '../../lib/connectApi'
import { useRemoteStore } from '../../stores/remote'
import PhoneConnectWizard from './PhoneConnectWizard.vue'
import { useConnectWizard, type ConnectWizard } from './useConnectWizard'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en: messages.en, vi: messages.vi } })
const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, Math.min(ms, 5)))
const adb = (over: Partial<AdbDevice> = {}): AdbDevice => ({ id: 'SER1', state: 'ready', transport: 'usb', model: 'SM-A336E', product: 'a33x', device: 'a33x', transportId: '1', alias: null, name: 'Galaxy A33', online: true, lastSeen: 1, ...over })
const status = (over: Partial<RemoteStatus> = {}): RemoteStatus => ({ enabled: true, listening: true, addresses: ['192.168.1.2'], port: 4330, relay: 'online', keepAwake: false, devices: [], ...over })
const phone: RemoteDevice = { deviceId: 'd9', name: 'Galaxy A33', platform: 'android', pairedAt: 900, lastSeen: 900, state: 'connected' }
const goodCheck = { lan: { ok: true, ms: 18 }, relay: { state: 'online', ms: 140 }, push: { ok: true } }

let w: VueWrapper | null = null
let fetched: (() => void) | null = null

function fakeApi(devices: AdbDevice[], over: Partial<ConnectApi> = {}): ConnectApi {
  return {
    hasUsb: true,
    remoteStatus: vi.fn(async () => null),
    remoteEnable: vi.fn(async () => null),
    pairStart: vi.fn(async () => ({ qr: 'QR', exp: Math.floor(Date.now() / 1000) + 120 })),
    pairCancel: vi.fn(async () => true),
    check: vi.fn(async () => goodCheck),
    installState: vi.fn(async () => ({ ok: true as const, value: { apk: { path: 'C:/o.apk', name: 'o.apk', size: 1 }, missing: false, suggested: null, url: '', running: false, source: null, version: '' } })),
    installUseSuggested: vi.fn(async () => ({ ok: false as const, error: 'apk' as const })),
    installStart: vi.fn(async () => ({ ok: true as const, value: { path: '/install/tok/orrery.apk', exp: Math.floor(Date.now() / 1000) + 600, port: 4330, addresses: ['192.168.1.2'] } })),
    installStop: vi.fn(async () => undefined),
    installArm: vi.fn(async () => true),
    onInstallFetched: (h) => {
      fetched = () => h('192.168.1.40')
      return () => undefined
    },
    devices: vi.fn(async () => devices),
    onDevices: () => () => undefined,
    usbInstall: vi.fn(async () => ({ ok: true as const })),
    usbReverse: vi.fn(async () => true),
    usbOpenPair: vi.fn(async () => true),
    serialHash: vi.fn(async () => 'abcdef0123456789'),
    qrSvg: vi.fn(async () => '<svg data-fake></svg>'),
    ...over,
  }
}

async function open(devices: AdbDevice[], st: Partial<RemoteStatus> = {}, over: Partial<ConnectApi> = {}): Promise<{ w: VueWrapper; wizard: ConnectWizard; api: ConnectApi }> {
  const remote = useRemoteStore()
  remote.status = status(st)
  const api = fakeApi(devices, over)
  let wizard!: ConnectWizard
  const Host = {
    components: { PhoneConnectWizard },
    setup() {
      wizard = useConnectWizard({ api, sleep })
      return { wizard }
    },
    template: '<PhoneConnectWizard :wizard="wizard" />',
  }
  w = mount(Host, { attachTo: document.body, global: { plugins: [i18n] } })
  wizard.start()
  await flushPromises()
  return { w, wizard, api }
}

const state = (): string | undefined => document.querySelector('[data-connect-wizard]')?.getAttribute('data-connect-state') ?? undefined
const q = (sel: string): HTMLElement | null => document.querySelector<HTMLElement>(sel)
const click = async (sel: string): Promise<void> => {
  q(sel)?.click()
  await flushPromises()
}

beforeEach(() => {
  now.value = Date.now()
  setActivePinia(createPinia())
  fetched = null
})
afterEach(() => {
  w?.unmount()
  w = null
  document.body.innerHTML = ''
})

describe('PhoneConnectWizard', () => {
  it('asks for consent when remote control is off and goes on to install once listening', async () => {
    const { wizard } = await open([adb()], { enabled: false, listening: false })
    expect(state()).toBe('consent')
    expect(q('[data-connect-consent-yes]')).not.toBeNull()
    const remote = useRemoteStore()
    remote.status = status()
    await click('[data-connect-consent-yes]')
    await vi.waitFor(() => expect(state()).toBe('install'))
    expect(wizard.name.value).toBe('install')
    expect(q('[data-connect-stepper]')?.getAttribute('data-step')).toBe('1')
  })

  it('install step: a ready cable phone gets an install button and the best badge', async () => {
    await open([adb()])
    expect(state()).toBe('install')
    expect(q('[data-connect-usb-install]')?.textContent).toContain('Galaxy A33')
    expect(q('[data-connect-usb-tile]')?.getAttribute('data-ready')).toBe('true')
    expect(q('[data-connect-usb-tile]')?.textContent).toContain('Fastest')
    expect(document.body.textContent).not.toContain('Choose APK')
  })

  it('install step: no phone on a cable puts QR first and disables the cable button', async () => {
    await open([])
    expect(q('[data-connect-usb-wait]')?.hasAttribute('disabled')).toBe(true)
    expect(q('[data-connect-qr-tile]')?.textContent).toContain('Suggested')
    expect(q('[data-connect-usb-desc]')?.textContent).toContain('No Android phone')
  })

  it('install step: x86 emulator is explained and not offered', async () => {
    await open([adb({ id: 'emu', transport: 'emulator', product: 'sdk_gphone64_x86_64', name: 'Emu' })])
    expect(q('[data-connect-usb-install]')).toBeNull()
    expect(q('[data-connect-usb-desc]')?.textContent).toContain('x86')
  })

  it('usb path runs to the pair step, then test, then done', async () => {
    const { api } = await open([adb()])
    await click('[data-connect-usb-install]')
    await vi.waitFor(() => expect(state()).toBe('pairConfirm'))
    expect(api.usbInstall).toHaveBeenCalledWith('SER1', 'C:/o.apk')
    expect(api.usbReverse).toHaveBeenCalledWith('SER1', 4330, true)
    expect(api.usbOpenPair).toHaveBeenCalledWith('SER1', 'QR')
    expect(q('[data-connect-pair-confirm]')).not.toBeNull()
    const remote = useRemoteStore()
    remote.status = status({ devices: [phone] })
    await vi.waitFor(() => expect(state()).toBe('paired'))
    expect(q('[data-connect-link]')?.getAttribute('data-linked')).toBe('true')
    await vi.waitFor(() => expect(state()).toBe('testDone'), { timeout: 4000 })
    expect(q('[data-connect-checks]')?.textContent).toContain('18 ms')
    expect(api.usbReverse).toHaveBeenLastCalledWith('SER1', 4330, false)
    await click('[data-connect-push-yes]')
    expect(state()).toBe('done')
  })

  it('usb failure shows the cause, the fix, and a QR exit', async () => {
    await open([adb()], {}, { usbInstall: vi.fn(async () => ({ ok: false as const, error: 'restricted' })) })
    await click('[data-connect-usb-install]')
    await vi.waitFor(() => expect(state()).toBe('usbFail'))
    expect(q('[data-connect-usb-fail]')?.getAttribute('data-error')).toBe('restricted')
    expect(q('[data-connect-usb-fail]')?.textContent).toContain('Install via USB')
    await click('[data-connect-usb-qr]')
    await vi.waitFor(() => expect(state()).toBe('qrInstall'))
  })

  it('signature failure explains the migration', async () => {
    await open([adb()], {}, { usbInstall: vi.fn(async () => ({ ok: false as const, error: 'signature' })) })
    await click('[data-connect-usb-install]')
    await vi.waitFor(() => expect(state()).toBe('usbFail'))
    expect(q('[data-connect-usb-fail]')?.textContent).toContain('different signature')
  })

  it('qr path: shows the page link, reacts to the APK fetch, then the pair confirm', async () => {
    const { api } = await open([])
    await click('[data-connect-qr-show]')
    await vi.waitFor(() => expect(q('[data-connect-qr-url]')).not.toBeNull())
    expect(q('[data-connect-qr-url]')?.textContent).toBe('http://192.168.1.2:4330/install/tok/')
    expect(api.installArm).toHaveBeenCalledWith(true)
    fetched?.()
    await vi.waitFor(() => expect(state()).toBe('qrInstalled'))
    expect(q('[data-connect-qr-fetched]')).not.toBeNull()
    const remote = useRemoteStore()
    remote.status = status({ pairing: { exp: 1, relay: false } })
    await flushPromises()
    remote.status = status({ pairing: null })
    await vi.waitFor(() => expect(state()).toBe('pairConfirm'))
    expect(api.installArm).toHaveBeenLastCalledWith(false)
  })

  it('qr path prefers the page path from the ticket and listens from the ticket start', async () => {
    const onInstallFetched = vi.fn((h: (ip: string) => void, since?: number) => {
      void h
      void since
      return () => undefined
    })
    const installStart = vi.fn(async () => ({ ok: true as const, value: { path: '/install/tok/orrery.apk', page: '/install/pg/', exp: Math.floor(Date.now() / 1000) + 600, port: 4330, addresses: ['192.168.1.2'] } }))
    const before = Math.floor(Date.now() / 1000)
    await open([], {}, { installStart, onInstallFetched })
    await click('[data-connect-qr-show]')
    await vi.waitFor(() => expect(q('[data-connect-qr-url]')).not.toBeNull())
    expect(q('[data-connect-qr-url]')?.textContent).toBe('http://192.168.1.2:4330/install/pg/')
    await vi.waitFor(() => expect(onInstallFetched).toHaveBeenCalled())
    const since = onInstallFetched.mock.calls[0]?.[1] ?? 0
    expect(since).toBeGreaterThanOrEqual(before - 1)
    expect(since).toBeLessThanOrEqual(Math.floor(Date.now() / 1000))
  })

  it('qr path without an APK reports it', async () => {
    await open([], {}, { installState: vi.fn(async () => ({ ok: true as const, value: { apk: null, missing: true, suggested: null, url: '', running: false, source: null, version: '' } })) })
    await click('[data-connect-qr-show]')
    await vi.waitFor(() => expect(q('[data-connect-qr-error]')).not.toBeNull())
    expect(q('[data-connect-qr-error]')?.getAttribute('data-error')).toBe('apk')
  })

  it('already installed goes to the pairing QR with the old selectors', async () => {
    await open([])
    await click('[data-connect-installed]')
    await vi.waitFor(() => expect(q('[data-remote-pair]')).not.toBeNull())
    expect(q('[data-remote-qr]')).not.toBeNull()
    expect(q('[data-remote-countdown]')?.textContent?.trim()).toMatch(/^[12]:[0-9]{2}$/)
    expect(q('[data-connect-stepper]')?.getAttribute('data-step')).toBe('2')
  })

  it('re-pairing an old phone is recognised by a newer pairedAt', async () => {
    const remote = useRemoteStore()
    await open([], { devices: [{ ...phone, pairedAt: 100 }] })
    await click('[data-connect-installed]')
    await vi.waitFor(() => expect(q('[data-remote-pair]')).not.toBeNull())
    remote.status = status({ devices: [{ ...phone, pairedAt: 100 }], pairing: { exp: 1, relay: false } })
    await flushPromises()
    remote.status = status({ devices: [{ ...phone, pairedAt: 100 }], pairing: null })
    await flushPromises()
    expect(state()).toBe('pairConfirm')
    remote.status = status({ devices: [{ ...phone, pairedAt: 900 }] })
    await vi.waitFor(() => expect(state()).toBe('paired'))
  })

  it('denied on the computer shows the denied panel with a new code button', async () => {
    await open([])
    await click('[data-connect-installed]')
    await vi.waitFor(() => expect(q('[data-remote-pair]')).not.toBeNull())
    useRemoteStore().status = status({ pairEnded: 'denied' })
    await vi.waitFor(() => expect(state()).toBe('denied'))
    expect(q('[data-connect-denied-renew]')).not.toBeNull()
  })

  it('remote turned off elsewhere closes the wizard', async () => {
    await open([])
    useRemoteStore().status = status({ enabled: false, listening: false })
    await vi.waitFor(() => expect(state()).toBe('idle'))
  })

  it('iPhone without a relay is blocked at install with a relay button', async () => {
    const { wizard } = await open([])
    wizard.choosePlatform('ios')
    await flushPromises()
    expect(state()).toBe('iosNoRelay')
    expect(q('[data-connect-ios-no-relay]')).not.toBeNull()
    expect(q('[data-connect-to-relay]')).not.toBeNull()
  })

  it('iPhone with a relay shows the web app QR and the three Safari steps', async () => {
    const { wizard } = await open([], { relayUrl: 'https://relay.example.dev' })
    wizard.choosePlatform('ios')
    await vi.waitFor(() => expect(q('[data-connect-ios]')).not.toBeNull())
    expect(q('[data-connect-ios]')?.querySelectorAll('li')).toHaveLength(3)
    expect(q('[data-connect-ios]')?.textContent).toContain('https://relay.example.dev/app/')
  })

  it('a check that cannot run falls back to a warning with a later button', async () => {
    const { wizard } = await open([], {}, { check: vi.fn(async () => null) })
    useRemoteStore().status = status({ devices: [phone] })
    wizard.dispatch({ type: 'alreadyInstalled' })
    await flushPromises()
    wizard.dispatch({ type: 'paired' })
    await vi.waitFor(() => expect(state()).toBe('testWarn'), { timeout: 8000 })
    expect(q('[data-connect-test-unknown]')).not.toBeNull()
    expect(q('[data-connect-test-later]')).not.toBeNull()
  }, 12000)

  it('a pairing ticket that ends with no phone does not leave pairConfirm stuck', async () => {
    await open([])
    await click('[data-connect-installed]')
    await vi.waitFor(() => expect(q('[data-remote-pair]')).not.toBeNull())
    const remote = useRemoteStore()
    remote.status = status({ pairing: { exp: 1, relay: false } })
    await flushPromises()
    remote.status = status({ pairing: null })
    await flushPromises()
    expect(state()).toBe('pairConfirm')
    now.value += 60_000
    await flushPromises()
    expect(state()).toBe('pairExpired')
  })

  it('re-check after a wizard run tests the connected phone, not a revoked old one', async () => {
    const check = vi.fn(async (_id: string) => goodCheck)
    const { wizard } = await open([], {}, { check })
    useRemoteStore().status = status({ devices: [phone] })
    wizard.dispatch({ type: 'alreadyInstalled' })
    await flushPromises()
    wizard.dispatch({ type: 'paired' })
    await vi.waitFor(() => expect(state()).toBe('testDone'), { timeout: 8000 })
    wizard.dispatch({ type: 'skip' })
    await flushPromises()
    useRemoteStore().status = status({ devices: [{ ...phone, state: 'revoked' }, { ...phone, deviceId: 'd10', name: 'Pixel' }] })
    wizard.dispatch({ type: 'recheck' })
    await vi.waitFor(() => expect(check).toHaveBeenCalled())
    expect(check.mock.calls[check.mock.calls.length - 1]?.[0]).toBe('d10')
  }, 12000)

  it('starting the wizard again forgets the previously paired phone', async () => {
    const { wizard } = await open([], {}, {})
    useRemoteStore().status = status({ devices: [phone] })
    wizard.dispatch({ type: 'alreadyInstalled' })
    await flushPromises()
    wizard.dispatch({ type: 'paired' })
    await vi.waitFor(() => expect(state()).toBe('testDone'), { timeout: 8000 })
    wizard.dispatch({ type: 'skip' })
    await flushPromises()
    expect(wizard.paired.value?.deviceId).toBe('d9')
    wizard.start()
    expect(wizard.paired.value).toBeNull()
  }, 12000)
})
