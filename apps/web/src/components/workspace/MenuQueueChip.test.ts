import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import vi from '../../i18n/vi'
import { parseTermMenu } from '../../lib/termMenu'
import { useMenuQueueStore } from '../../stores/menuQueue'
import { useWorkspaceStore } from '../../stores/workspace'
import MenuQueueChip from './MenuQueueChip.vue'

const DIR = join(__dirname, '..', '..', 'lib', '__fixtures__', 'term-menu')
const menu = parseTermMenu(readFileSync(join(DIR, 'claude-ask-single.txt'), 'utf8').replace(/\n$/, '').split('\n'))!
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi } })

describe('MenuQueueChip', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => (document.body.innerHTML = ''))

  it('is absent with an empty queue and counts waiting panes', async () => {
    const q = useMenuQueueStore()
    const w = mount(MenuQueueChip, { global: { plugins: [i18n] }, attachTo: document.body })
    expect(w.find('[data-menu-queue-chip]').exists()).toBe(false)
    q.report('a', { menu, question: 'First?' }, 1)
    q.report('b', { menu, question: 'Second?' }, 2)
    await w.vm.$nextTick()
    expect(w.find('[data-menu-queue-chip]').text()).toContain('2')
    q.report('a', null)
    await w.vm.$nextTick()
    expect(w.find('[data-menu-queue-chip]').text()).toContain('1')
    w.unmount()
  })

  it('opens a list with the question of each pane, oldest first, and a click jumps to the pane', async () => {
    const q = useMenuQueueStore()
    const ws = useWorkspaceStore()
    q.report('b', { menu, question: 'Second?' }, 2)
    q.report('a', { menu, question: 'First?' }, 1)
    const activated: string[] = []
    ws.activate = ((k: string) => activated.push(k)) as typeof ws.activate
    const w = mount(MenuQueueChip, { global: { plugins: [i18n] }, attachTo: document.body })
    await w.find('[data-menu-queue-chip]').trigger('click')
    await w.vm.$nextTick()
    const rows = [...document.querySelectorAll<HTMLElement>('[data-asked]')]
    expect(rows.map((r) => r.dataset.asked)).toEqual(['a', 'b'])
    expect(rows[0]!.textContent).toContain('First?')
    rows[1]!.click()
    expect(activated).toEqual(['b'])
    w.unmount()
  })
})
