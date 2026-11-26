import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { reactive } from 'vue'
import type { AdbDevice, RemoteDevice } from '@ccm/shared'
import devicesV113 from '../i18n/devices-v113'

const m = vi.hoisted(() => ({
  remoteStore: null as unknown as Record<string, unknown>,
  adbStore: null as unknown as Record<string, unknown>,
  emuStore: null as unknown as Record<string, unknown>,
  remoteCheck: vi.fn(),
  remoteRevoke: vi.fn(),
  remoteRevokeUndo: vi.fn(),
  remoteStatus: vi.fn(),
  installGet: vi.fn(),
  installUseSuggested: vi.fn(),
  devInstall: vi.fn(),
  devPkgVersion: vi.fn(),
  devLaunch: vi.fn(),
}))

vi.mock('@features/remote', () => ({
  useRemoteStore: () => m.remoteStore,
  remoteCheck: (id: string) => m.remoteCheck(id),
  remoteRevoke: (id: string, g: number) => m.remoteRevoke(id, g),
  remoteRevokeUndo: (id: string) => m.remoteRevokeUndo(id),
  remoteStatus: () => m.remoteStatus(),
  installGet: () => m.installGet(),
  installUseSuggested: () => m.installUseSuggested(),
  hashSerial: async (id: string) => `hash-${id}`.toLowerCase(),
  expMs: (n: number) => (n < 1e12 ? n * 1000 : n),
}))
vi.mock('@features/settings', () => ({ SettingEmbed: { props: ['id'], template: '<div :data-setting-row="id"><slot /></div>' } }))
vi.mock('../stores/devices', () => ({ useDevicesStore: () => m.adbStore }))
vi.mock('../stores/emulators', () => ({ useEmulatorsStore: () => m.emuStore }))
vi.mock('../lib/devices', () => ({
  hasDevices: true,
  hasDeviceInstall: true,
  devInstall: (id: string, apk: string) => m.devInstall(id, apk),
  devPkgVersion: (id: string, pkg: string) => m.devPkgVersion(id, pkg),
  devLaunch: (id: string, t: unknown) => m.devLaunch(id, t),
}))
vi.mock('./DeviceCaptureDialog.vue', () => ({ default: { props: ['open', 'deviceId'], template: '<div data-capture :data-open="open" :data-id="deviceId" />' } }))

import { useToastsStore } from '../../../core/stores/toasts'
import { useUiStore } from '../../../core/stores/ui'
import SettingsDevices from './SettingsDevices.vue'

const phone = (over: Partial<RemoteDevice> = {}): RemoteDevice => ({ deviceId: 'r1', name: 'Pixel 8', platform: 'android', pairedAt: 1, lastSeen: 2, state: 'connected', via: 'lan', push: 'fcm', pushOn: true, appVersion: '4.0.10', model: 'Pixel 8', ...over })
const adbDev = (over: Partial<AdbDevice> = {}): AdbDevice => ({ id: 'R58M', state: 'ready', transport: 'usb', model: 'Redmi', product: null, device: null, transportId: null, alias: null, name: 'Redmi Note 12', online: true, lastSeen: null, ...over })

function setup(remote: RemoteDevice[], adb: AdbDevice[]): void {
  m.remoteStore = reactive({ devices: remote, status: null, start: vi.fn(), setDevicePush: vi.fn(async () => true), renameDevice: vi.fn(async () => true) })
  m.adbStore = reactive({ devices: adb, adbFound: true, open: vi.fn(async () => undefined), close: vi.fn(async () => undefined), setAlias: vi.fn(async () => undefined) })
  m.emuStore = reactive({ avds: [], load: vi.fn(async () => undefined), start: vi.fn(async () => undefined), lastError: null })
}

async function mountPage(): Promise<VueWrapper> {
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en: devicesV113.en }, missingWarn: false, fallbackWarn: false })
  const w = mount(SettingsDevices, { global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

const rows = (w: VueWrapper) => w.findAll('[data-device-row]')

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  Object.values(m).forEach((f) => typeof f === 'function' && (f as ReturnType<typeof vi.fn>).mockReset())
  m.installGet.mockResolvedValue({ ok: true, value: { apk: { path: 'C:\\apk\\orrery.apk', name: 'orrery.apk', size: 1 }, suggested: null, version: '4.0.10', source: 'bundled' } })
  m.devPkgVersion.mockResolvedValue({ ok: true, installed: false })
  m.remoteStatus.mockResolvedValue(null)
  m.remoteCheck.mockResolvedValue({ lan: { ok: true, ms: 18 }, relay: { state: 'off' }, push: { ok: true } })
  m.remoteRevoke.mockResolvedValue(true)
  m.remoteRevokeUndo.mockResolvedValue(true)
  m.devInstall.mockResolvedValue({ ok: true })
  m.devLaunch.mockResolvedValue({ ok: true })
})

describe('SettingsDevices', () => {
  it('lists remote and adb devices in one list and filters by role', async () => {
    setup([phone()], [adbDev()])
    const w = await mountPage()
    expect(rows(w)).toHaveLength(2)
    expect(w.find('[data-devices-count]').text()).toContain('2 devices')
    await w.find('[data-devices-filter-value="remote"]').trigger('click')
    expect(rows(w)).toHaveLength(1)
    expect(rows(w)[0]?.text()).toContain('Pixel 8')
    await w.find('[data-devices-filter-value="adb"]').trigger('click')
    expect(rows(w)[0]?.text()).toContain('Redmi Note 12')
    await w.find('[data-devices-filter-value="emulator"]').trigger('click')
    expect(rows(w)).toHaveLength(0)
    expect(w.find('[data-devices-none]').exists()).toBe(true)
    w.unmount()
  })

  it('merges a phone with the adb device that has the same serial hash into one row with two tags', async () => {
    setup([phone({ adbSerialHash: 'hash-r58m' })], [adbDev()])
    const w = await mountPage()
    expect(rows(w)).toHaveLength(1)
    expect(rows(w)[0]?.find('[data-device-tag="remote"]').exists()).toBe(true)
    expect(rows(w)[0]?.find('[data-device-tag="adb"]').text()).toBe('USB adb')
    w.unmount()
  })

  it('opens the detail on click and closes it again', async () => {
    setup([phone()], [])
    const w = await mountPage()
    expect(w.find('[data-device-detail]').exists()).toBe(false)
    await w.find('[data-device-toggle]').trigger('click')
    expect(w.find('[data-device-detail]').exists()).toBe(true)
    expect(w.find('[data-device-toggle]').attributes('aria-expanded')).toBe('true')
    await w.find('[data-device-toggle]').trigger('click')
    await flushPromises()
    expect(w.find('[data-device-toggle]').attributes('aria-expanded')).toBe('false')
    w.unmount()
  })

  it('sends a test notification and shows the measured LAN time', async () => {
    setup([phone()], [])
    const w = await mountPage()
    await w.find('[data-device-toggle]').trigger('click')
    await w.find('[data-device-test]').trigger('click')
    await flushPromises()
    expect(m.remoteCheck).toHaveBeenCalledWith('r1')
    expect(w.find('[data-device-state="ok"]').text()).toContain('LAN 18 ms')
    expect(useToastsStore().list.at(-1)?.text).toContain('Test notification sent to Pixel 8')
    w.unmount()
  })

  it('revokes with a 5 second grace and Undo brings the device back', async () => {
    setup([phone()], [])
    const w = await mountPage()
    await w.find('[data-device-toggle]').trigger('click')
    await w.find('[data-device-revoke]').trigger('click')
    expect(w.find('[data-device-revoke-ask]').text()).toContain('Revoke Pixel 8?')
    expect(m.remoteRevoke).not.toHaveBeenCalled()
    await w.find('[data-device-revoke-yes]').trigger('click')
    await flushPromises()
    expect(m.remoteRevoke).toHaveBeenCalledWith('r1', 5)
    const toast = useToastsStore().list.find((x) => x.id === 'devices-revoke-r1')
    expect(toast?.kind).toBe('undo')
    expect(toast?.timeoutMs).toBe(5000)
    toast?.actions?.[0]?.run()
    await flushPromises()
    expect(m.remoteRevokeUndo).toHaveBeenCalledWith('r1')
    expect(useToastsStore().list.some((x) => x.id === 'devices-revoke-r1')).toBe(false)
    w.unmount()
  })

  it('cancelling the revoke question does not call the core', async () => {
    setup([phone()], [])
    const w = await mountPage()
    await w.find('[data-device-toggle]').trigger('click')
    await w.find('[data-device-revoke]').trigger('click')
    await w.find('[data-device-revoke-no]').trigger('click')
    await flushPromises()
    expect(m.remoteRevoke).not.toHaveBeenCalled()
    expect(w.find('[data-device-revoke-ask]').exists()).toBe(false)
    w.unmount()
  })

  it('a phone that is revoking shows Undo on the row and no other action', async () => {
    setup([phone({ revoking: true })], [])
    const w = await mountPage()
    expect(w.find('[data-device-row]').attributes('data-device-revoking')).toBe('true')
    expect(w.find('[data-device-state]').text()).toContain('Revoking')
    await w.find('[data-device-undo]').trigger('click')
    await flushPromises()
    expect(m.remoteRevokeUndo).toHaveBeenCalledWith('r1')
    w.unmount()
  })

  it('shows Update on an adb phone running an older Remote and installs the bundled app', async () => {
    m.devPkgVersion.mockResolvedValue({ ok: true, installed: true, versionName: '4.0.8', versionCode: 8 })
    setup([phone({ adbSerialHash: 'hash-r58m', appVersion: '4.0.8' })], [adbDev()])
    const w = await mountPage()
    expect(w.find('[data-device-tag="remote"]').text()).toContain('older than desktop')
    const button = w.find('[data-device-action="update"]')
    expect(button.text()).toBe('Update to 4.0.10')
    await button.trigger('click')
    await flushPromises()
    expect(m.devInstall).toHaveBeenCalledWith('R58M', 'C:\\apk\\orrery.apk')
    expect(m.devPkgVersion).toHaveBeenCalledTimes(2)
    w.unmount()
  })

  it('shows Install Remote for an adb phone without the app', async () => {
    setup([], [adbDev()])
    const w = await mountPage()
    expect(w.find('[data-device-action="install"]').text()).toBe('Install Remote')
    expect(w.find('[data-device-state]').text()).toContain('Remote not installed')
    w.unmount()
  })

  it('explains why an x86_64 emulator cannot get Remote', async () => {
    setup([], [adbDev({ id: 'emulator-5554', transport: 'emulator', name: 'sdk' })])
    m.emuStore = reactive({ avds: [{ name: 'Emu', displayName: 'Emu', api: 35, abi: 'x86_64', device: null, tag: null, serial: 'emulator-5554' }], load: vi.fn(), start: vi.fn(), lastError: null })
    const w = await mountPage()
    expect(w.find('[data-device-action]').exists()).toBe(false)
    expect(w.find('[data-device-blocked]').text()).toContain('x86_64')
    w.unmount()
  })

  it('suggests merging by model and name and remembers the answer', async () => {
    setup([phone({ name: 'Redmi Note 12', model: 'Redmi Note 12' })], [adbDev({ model: 'Redmi_Note_12' })])
    const w = await mountPage()
    expect(rows(w)).toHaveLength(2)
    expect(w.find('[data-device-suggest]').text()).toContain('Same device?')
    await w.find('[data-device-merge]').trigger('click')
    await flushPromises()
    expect(rows(w)).toHaveLength(1)
    expect(JSON.parse(localStorage.getItem('ccm.deviceMerge') ?? '{}').merged).toEqual(['r1|R58M'])
    w.unmount()
  })

  it('shows an invitation with two actions when there are no devices', async () => {
    setup([], [])
    const w = await mountPage()
    expect(w.find('[data-devices-empty]').text()).toContain('No devices yet')
    expect(w.find('[data-devices-filter]').exists()).toBe(false)
    await w.find('[data-devices-empty-connect]').trigger('click')
    expect(useUiStore().settingsPage).toBe('remote')
    await w.find('[data-devices-pair-adb]').trigger('click')
    expect(useUiStore().settingsOpen).toBe(false)
    expect(useUiStore().view).toBe('devices')
    w.unmount()
  })

  it('works with a desktop that reports no version, no hash and no revoking flag', async () => {
    m.installGet.mockResolvedValue({ ok: false, error: 'unavailable' })
    const old = phone()
    delete old.appVersion
    delete old.pairedVia
    setup([old], [])
    const w = await mountPage()
    expect(w.find('[data-device-tag="remote"]').text()).toBe('Remote')
    expect(w.find('[data-device-action]').exists()).toBe(false)
    w.unmount()
  })
})
