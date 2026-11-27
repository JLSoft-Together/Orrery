import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { reactive } from 'vue'
import { i18n } from '../../../i18n'
import runV113 from '../i18n/run-v113'

const hoisted = vi.hoisted(() => ({
  startMany: vi.fn(),
  cancelMany: vi.fn(),
  attach: vi.fn(),
  save: vi.fn(),
}))

const devices = [
  { id: 'a33', name: 'Galaxy A33', platform: 'android', emulator: false, state: 'ready', source: 'adb' },
  { id: 'px8', name: 'Pixel 8', platform: 'android', emulator: false, state: 'ready', source: 'adb' },
  { id: 'tab', name: 'Tab S9', platform: 'android', emulator: false, state: 'ready', source: 'adb' },
  { id: 'off', name: 'Old phone', platform: 'android', emulator: false, state: 'offline', source: 'adb' },
]
const multiState = reactive<{ value: unknown }>({ value: null })

vi.mock('../stores/run', () => ({
  useRunStore: () => ({
    devices,
    selectedConfig: () => ({ id: 'cfg', type: 'android', name: 'app', cwd: '/p', module: ':app', variant: 'debug' }),
    devicesFor: () => devices,
    refreshDevices: () => Promise.resolve(),
  }),
}))
vi.mock('../stores/multiRun', () => ({
  useMultiRunStore: () => ({ stateFor: () => multiState.value, startMany: hoisted.startMany, cancelMany: hoisted.cancelMany }),
}))
vi.mock('../composables/useShotsHandoff', () => ({ useShotsHandoff: () => ({ attach: hoisted.attach }) }))
vi.mock('../lib/multiRunUi', async (orig) => ({ ...(await orig<typeof import('../lib/multiRunUi')>()), saveShot: hoisted.save }))

import RunShotsDialog from './RunShotsDialog.vue'

const t = (k: string, p?: Record<string, unknown>): string => i18n.global.t(k, p ?? {})
let w: VueWrapper | null = null

const outcome = (): unknown => ({
  shots: [
    { deviceId: 'a33', variant: 'dark', stage: 'done', png: 'data:image/png;base64,AA', width: 100, height: 200 },
    { deviceId: 'px8', variant: 'dark', stage: 'failed', error: 'signature' },
  ],
  restoreFailed: [],
})

beforeEach(() => {
  i18n.global.mergeLocaleMessage('en', runV113.en)
  i18n.global.locale.value = 'en'
  multiState.value = null
  hoisted.startMany.mockReset().mockResolvedValue(null)
  hoisted.cancelMany.mockReset()
  hoisted.attach.mockReset().mockResolvedValue('ok')
  hoisted.save.mockReset()
})
afterEach(() => {
  w?.unmount()
  w = null
})

const mountIt = async (): Promise<VueWrapper> => {
  w = mount(RunShotsDialog, { props: { open: true, root: '/p' }, global: { plugins: [i18n], stubs: { teleport: true } } })
  await flushPromises()
  return w
}

describe('RunShotsDialog', () => {
  it('lists only ready devices, picks the first two and caps at 3', async () => {
    const el = await mountIt()
    const ids = el.findAll('[data-multi-device]').map((b) => b.attributes('data-multi-device'))
    expect(ids).toEqual(['a33', 'px8', 'tab'])
    expect(el.get('[data-multi-device="a33"]').attributes('aria-pressed')).toBe('true')
    expect(el.get('[data-multi-device="tab"]').attributes('aria-pressed')).toBe('false')
    await el.get('[data-multi-device="tab"]').trigger('click')
    expect(el.get('[data-multi-device="tab"]').attributes('aria-pressed')).toBe('true')
    await el.get('[data-multi-device="a33"]').trigger('click')
    await el.get('[data-multi-device="a33"]').trigger('click')
    expect(el.get('[data-multi-device="a33"]').attributes('aria-pressed')).toBe('true')
  })

  it('starts the run with the picked devices and variant', async () => {
    const el = await mountIt()
    const dark = el.findAll('[role="radio"]').find((b) => b.text() === t('runMulti.variants.dark'))!
    await dark.trigger('click')
    await el.get('[data-multi-run]').trigger('click')
    expect(hoisted.startMany).toHaveBeenCalledWith('/p', 'cfg', ['a33', 'px8'], 'dark')
  })

  it('shows the shot grid and a status row per device with the failure reason', async () => {
    multiState.value = { variant: 'dark', running: false, cancelling: false, stages: { a33: { stage: 'done' }, px8: { stage: 'failed', error: 'signature' } }, outcome: outcome() }
    const el = await mountIt()
    expect(el.findAll('[data-multi-shot]').map((s) => s.attributes('data-multi-shot'))).toEqual(['a33'])
    expect(el.get('[data-multi-shot="a33"] img').attributes('style')).toContain('100 / 200')
    expect(el.get('[data-multi-row="px8"]').text()).toContain(t('runMulti.err.signature'))
    expect(el.get('[data-multi-row="px8"]').attributes('data-multi-stage')).toBe('failed')
    expect(el.find('[data-multi-back]').exists()).toBe(true)
  })

  it('sends the image to the agent without submitting and says so', async () => {
    multiState.value = { variant: 'dark', running: false, cancelling: false, stages: { a33: { stage: 'done' } }, outcome: outcome() }
    const el = await mountIt()
    await el.get('[data-multi-send]').trigger('click')
    await flushPromises()
    expect(hoisted.attach).toHaveBeenCalledWith([{ png: 'data:image/png;base64,AA', device: 'Galaxy A33', variant: 'dark' }])
    expect(el.get('[data-multi-note]').text()).toBe(t('runMulti.sent'))
  })

  it('warns when there is no agent pane and saves a file on request', async () => {
    hoisted.attach.mockResolvedValue('no-agent')
    multiState.value = { variant: 'dark', running: false, cancelling: false, stages: { a33: { stage: 'done' } }, outcome: outcome() }
    const el = await mountIt()
    await el.get('[data-multi-send-all]').trigger('click')
    await flushPromises()
    expect(el.get('[data-multi-note]').text()).toBe(t('runMulti.noAgent'))
    await el.get('[data-multi-save]').trigger('click')
    expect(hoisted.save).toHaveBeenCalledWith('data:image/png;base64,AA', 'Galaxy-A33-dark.png')
  })

  it('cancels the run when the dialog is closed while it runs, so settings get restored', async () => {
    multiState.value = { variant: 'dark', running: true, cancelling: false, stages: { a33: { stage: 'capturing' } }, outcome: null }
    const el = await mountIt()
    expect(el.find('[data-multi-run]').exists()).toBe(false)
    await el.get('[data-multi-close]').trigger('click')
    expect(hoisted.cancelMany).toHaveBeenCalledWith('/p')
    expect(el.emitted('close')).toHaveLength(1)
  })

  it('closing an idle dialog does not cancel anything', async () => {
    const el = await mountIt()
    await el.get('[data-multi-close]').trigger('click')
    expect(hoisted.cancelMany).not.toHaveBeenCalled()
  })

  it('warns when the settings could not be put back', async () => {
    multiState.value = {
      variant: 'dark',
      running: false,
      cancelling: false,
      stages: { a33: { stage: 'done' } },
      outcome: { shots: [], restoreFailed: ['a33'] },
    }
    const el = await mountIt()
    expect(el.find('[data-multi-back]').exists()).toBe(false)
    expect(el.get('[data-multi-restore-failed]').text()).toContain('Galaxy A33')
  })
})
