import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import vi_ from '../../i18n/vi'
import { parseTermMenu } from '../../lib/termMenu'

const bridge = vi.hoisted(() => ({ sent: [] as unknown[] }))

vi.mock('../../lib/desktop', async (orig) => ({
  ...(await orig<typeof import('../../lib/desktop')>()),
  desktop: { menuToastSync: (p: unknown) => bridge.sent.push(p), onMenuToast: () => () => undefined },
  hasTerminals: true,
}))

const { default: InboxChip } = await import('./InboxChip.vue')
const { default: InboxPanel } = await import('./InboxPanel.vue')
const { useInboxGlobal } = await import('../../composables/useInboxGlobal')
const { useMenuQueueStore } = await import('../../stores/menuQueue')
const { useInboxStore } = await import('../../stores/inbox')
const { useWorkspaceStore } = await import('../../stores/workspace')
const { useExtrasStore } = await import('../../stores/extras')

const DIR = join(__dirname, '..', '..', 'lib', '__fixtures__', 'term-menu')
const menu = parseTermMenu(readFileSync(join(DIR, 'claude-ask-single.txt'), 'utf8').replace(/\n$/, '').split('\n'))!
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: vi_ } })
const anchor = { left: 0, right: 400, top: 0, bottom: 20, width: 400, height: 20, x: 0, y: 0, toJSON: () => ({}) } as DOMRect

describe('InboxChip', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    bridge.sent = []
  })
  afterEach(() => (document.body.innerHTML = ''))

  it('is absent with nothing to do and counts menu panes', async () => {
    const q = useMenuQueueStore()
    const w = mount(InboxChip, { global: { plugins: [i18n] }, attachTo: document.body })
    expect(w.find('[data-inbox-chip]').exists()).toBe(false)
    q.report('a', { menu, question: 'First?' }, 1)
    q.report('b', { menu, question: 'Second?' }, 2)
    await nextTick()
    expect(w.find('[data-inbox-chip]').text()).toContain('Inbox 2')
    expect(w.find('[data-inbox-chip]').attributes('data-tone')).toBe('waiting')
    q.report('a', null)
    await nextTick()
    expect(w.find('[data-inbox-chip]').text()).toContain('Inbox 1')
    w.unmount()
  })

  it('opens the Inbox anchored to itself', async () => {
    useMenuQueueStore().report('a', { menu, question: 'First?' }, 1)
    const inbox = useInboxStore()
    const w = mount(InboxChip, { global: { plugins: [i18n] }, attachTo: document.body })
    await nextTick()
    await w.find('[data-inbox-chip]').trigger('click')
    expect(inbox.open).toBe(true)
    expect(inbox.trigger).toBe(w.find('[data-inbox-chip]').element)
    await w.find('[data-inbox-chip]').trigger('click')
    expect(inbox.open).toBe(false)
    w.unmount()
  })

  it('lists menus oldest first in their group and only jumps to the pane on click', async () => {
    const q = useMenuQueueStore()
    const ws = useWorkspaceStore()
    q.report('b', { menu, question: 'Second?' }, 2)
    q.report('a', { menu, question: 'First?' }, 1)
    const answered: string[] = []
    q.register('a', async () => void answered.push('a'))
    ws.panes.push({ key: 'a', preset: 'shell', state: 'running' }, { key: 'b', preset: 'shell', state: 'running' })
    const activated: string[] = []
    ws.activate = ((k: string) => activated.push(k)) as typeof ws.activate
    const w = mount(InboxPanel, { props: { anchor }, global: { plugins: [i18n] }, attachTo: document.body })
    await nextTick()
    expect([...document.querySelectorAll('[data-inbox-group]')].map((g) => g.getAttribute('data-inbox-group'))).toEqual(['menu'])
    const rows = [...document.querySelectorAll<HTMLElement>('[data-inbox-row][data-reason=menu]')]
    expect(rows.map((r) => r.textContent)).toEqual([expect.stringContaining('First?'), expect.stringContaining('Second?')])
    rows[1]!.click()
    expect(activated).toEqual(['b'])
    expect(answered).toEqual([])
    w.unmount()
  })

  it('keeps the OS menu toast running without the chip mounted', async () => {
    useExtrasStore().setNotify({ toast: 'on', kinds: { waiting: true }, quiet: { enabled: false, from: '22:00', to: '07:00' }, stuckMinutes: 10, idleMinutes: 30 } as never)
    const Probe = defineComponent({
      setup() {
        useInboxGlobal()
        return () => h('div')
      },
    })
    const w = mount(Probe, { global: { plugins: [i18n] } })
    useMenuQueueStore().report('a', { menu, question: 'Q?' }, 1)
    await nextTick()
    const last = bridge.sent.at(-1) as { entries: { paneKey: string }[] }
    expect(last.entries.map((e) => e.paneKey)).toEqual(['a'])
    w.unmount()
  })

  it('opens and steps with Ctrl+Shift+A and Ctrl+Alt+Q from anywhere', async () => {
    const Probe = defineComponent({
      setup() {
        useInboxGlobal()
        return () => h('div')
      },
    })
    const ws = useWorkspaceStore()
    ws.panes.push({ key: 'a', preset: 'shell', state: 'running', attention: 'waiting' }, { key: 'b', preset: 'shell', state: 'error' })
    const activated: string[] = []
    ws.activate = ((k: string) => activated.push(k)) as typeof ws.activate
    const inbox = useInboxStore()
    const w = mount(Probe, { global: { plugins: [i18n] } })
    window.dispatchEvent(new KeyboardEvent('keydown', { code: 'KeyA', key: 'A', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))
    expect(inbox.open).toBe(true)
    const q = new KeyboardEvent('keydown', { code: 'KeyQ', key: 'q', ctrlKey: true, altKey: true, bubbles: true, cancelable: true })
    Object.defineProperty(q, 'getModifierState', { value: () => false })
    window.dispatchEvent(q)
    expect(activated).toEqual(['a'])
    expect(inbox.count).toBe(2)
    w.unmount()
  })
})
