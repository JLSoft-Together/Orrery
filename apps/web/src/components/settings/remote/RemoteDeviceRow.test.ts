import { afterEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import type { RemoteDevice } from '@ccm/shared'
import { i18n } from '../../../i18n'
import RemoteDeviceRow from './RemoteDeviceRow.vue'

let w: VueWrapper | null = null
afterEach(() => {
  w?.unmount()
  w = null
})

const device = (extra: Partial<RemoteDevice> = {}): RemoteDevice => ({ deviceId: 'p1', name: 'Pixel 7', originalName: 'Pixel 7', alias: '', platform: 'android', pairedAt: 1_700_000_000, lastSeen: 1_700_000_000, state: 'idle', ...extra })

const mountRow = (d: RemoteDevice): VueWrapper => {
  i18n.global.locale.value = 'en'
  w = mount(RemoteDeviceRow, { props: { device: d }, attachTo: document.body, global: { plugins: [i18n] } })
  return w
}

const open = async (row: VueWrapper): Promise<HTMLInputElement> => {
  await row.get('[data-remote-rename]').trigger('click')
  return row.get<HTMLInputElement>('[data-remote-rename-input]').element
}

describe('RemoteDeviceRow rename', () => {
  it('edits in place and saves on Enter with the trimmed name', async () => {
    const row = mountRow(device())
    const input = await open(row)
    expect(input.maxLength).toBe(40)
    await row.get('[data-remote-rename-input]').setValue('  Work phone  ')
    await row.get('[data-remote-rename-input]').trigger('keydown', { key: 'Enter' })
    expect(row.emitted('rename')).toEqual([['p1', 'Work phone']])
    expect(row.find('[data-remote-rename-input]').exists()).toBe(false)
  })

  it('cancels on Esc without emitting', async () => {
    const row = mountRow(device())
    await open(row)
    await row.get('[data-remote-rename-input]').setValue('Nope')
    await row.get('[data-remote-rename-input]').trigger('keydown', { key: 'Escape' })
    expect(row.emitted('rename')).toBeUndefined()
    expect(row.find('[data-remote-rename-input]').exists()).toBe(false)
  })

  it('an empty value clears an existing alias and shows the original name', async () => {
    const row = mountRow(device({ name: 'Desk', alias: 'Desk' }))
    expect(row.get('[data-remote-device-original]').text()).toContain('Pixel 7')
    const input = await open(row)
    expect(input.value).toBe('Desk')
    await row.get('[data-remote-rename-input]').setValue('   ')
    await row.get('[data-remote-rename-input]').trigger('keydown', { key: 'Enter' })
    expect(row.emitted('rename')).toEqual([['p1', '']])
  })

  it('does not emit when nothing changed, strips control characters and caps emoji names at 40', async () => {
    const row = mountRow(device())
    await open(row)
    await row.get('[data-remote-rename-input]').setValue('   ')
    await row.get('[data-remote-rename-input]').trigger('keydown', { key: 'Enter' })
    expect(row.emitted('rename')).toBeUndefined()
    await open(row)
    await row.get('[data-remote-rename-input]').setValue(`a\u0007b${'📱'.repeat(60)}`)
    await row.get('[data-remote-rename-input]').trigger('keydown', { key: 'Enter' })
    const [, name] = row.emitted('rename')![0] as [string, string]
    expect(Array.from(name)).toHaveLength(40)
    expect(name.startsWith('ab')).toBe(true)
  })

  it('shows no original line without an alias', () => {
    const row = mountRow(device())
    expect(row.find('[data-remote-device-original]').exists()).toBe(false)
  })
})
