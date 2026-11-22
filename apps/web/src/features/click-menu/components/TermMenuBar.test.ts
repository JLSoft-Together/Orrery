import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../../i18n/en'
import vi from '../../../i18n/vi'
import { parseTermMenu } from '../lib/termMenu'
import { useSettingsStore } from '../../../core/stores/settings'
import TermMenuBar from './TermMenuBar.vue'

const DIR = join(__dirname, '..', 'lib', '__fixtures__', 'term-menu')
const menuOf = (name: string) => {
  const lines = readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
  return parseTermMenu(lines)!
}
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi } })
const mountBar = (name: string, busy = false) => mount(TermMenuBar, { props: { menu: menuOf(name), busy }, global: { plugins: [i18n] } })

describe('TermMenuBar', () => {
  it('renders one button per option with its number and full label as title', () => {
    const w = mountBar('claude-ask-single')
    const buttons = w.findAll('[data-term-menu-item]')
    expect(buttons.map((b) => b.attributes('title'))).toContain('Blue')
    expect(buttons[2]!.text()).toContain('3')
    expect(buttons[2]!.text()).toContain('Blue')
  })

  it('marks the highlighted option without relying on colour', () => {
    const w = mountBar('claude-ask-single')
    const on = w.findAll('[data-term-menu-item]').filter((b) => b.attributes('aria-current') === 'true')
    expect(on).toHaveLength(1)
    expect(on[0]!.find('svg').exists()).toBe(true)
  })

  it('exposes tick state and the Submit button in a multiSelect', () => {
    const w = mountBar('claude-ask-multi-ticked')
    const pressed = w.findAll('[data-term-menu-item]').filter((b) => b.attributes('aria-pressed') === 'true')
    expect(pressed.length).toBeGreaterThan(0)
    expect(w.text()).toContain('Submit')
  })

  it('emits the item on click and disables the buttons while busy', async () => {
    const w = mountBar('claude-ask-single')
    await w.findAll('[data-term-menu-item]')[1]!.trigger('click')
    expect(w.emitted('pick')![0]![0]).toMatchObject({ label: 'Green' })
    const busy = mountBar('claude-ask-single', true)
    expect(busy.findAll('[data-term-menu-item]').every((b) => b.attributes('disabled') !== undefined)).toBe(true)
  })

  it('has en and vi text for every clickMenu key', () => {
    const keys = (o: Record<string, unknown>) => Object.keys((o as { clickMenu: object }).clickMenu).sort()
    expect(keys(vi)).toEqual(keys(en))
  })
})

describe('click menu setting', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('is on by default', () => {
    expect(useSettingsStore().clickMenu).toBe(true)
  })

  it('persists as ccm.clickMenu', async () => {
    const s = useSettingsStore()
    s.clickMenu = false
    await Promise.resolve()
    expect(localStorage.getItem('ccm.clickMenu')).toBe('0')
    setActivePinia(createPinia())
    expect(useSettingsStore().clickMenu).toBe(false)
  })
})
