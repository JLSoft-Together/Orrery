import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../../i18n/en'
import vi_ from '../../../i18n/vi'
import { menuSig } from '../lib/menuToast'
import { parseTermMenu } from '../lib/termMenu'

const bridge = vi.hoisted(() => {
  const state: { sync: ((p: unknown) => void) | undefined; handler: ((a: unknown) => void) | null; off: number } = { sync: undefined, handler: null, off: 0 }
  return state
})

vi.mock('../../../core/lib/desktop', () => ({
  desktop: {
    menuToastSync: (p: unknown) => bridge.sync?.(p),
    onMenuToast: (h: (a: unknown) => void) => {
      bridge.handler = h
      return () => void bridge.off++
    },
  },
  hasTerminals: true,
}))

const { useMenuToast } = await import('./useMenuToast')
const { useMenuQueueStore } = await import('../stores/menuQueue')
const { useSettingsStore } = await import('../../../core/stores/settings')
const { useExtrasStore } = await import('../../../core/stores/extras')
const { useWorkspaceStore } = await import('../../workspace/stores/workspace')
const { useUiStore } = await import('../../../core/stores/ui')

const DIR = join(__dirname, '..', 'lib', '__fixtures__', 'term-menu')
const menu = parseTermMenu(readFileSync(join(DIR, 'claude-permission.txt'), 'utf8').replace(/\n$/, '').split('\n'))!
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: vi_ } })

const notify = (toast: 'on' | 'off' = 'on', waiting = true) =>
  ({ toast, kinds: { waiting }, quiet: { enabled: false, from: '22:00', to: '07:00' }, stuckMinutes: 10, idleMinutes: 30 }) as never

const Probe = defineComponent({
  setup() {
    const q = useMenuQueueStore()
    useMenuToast(() => q.entries.map((e) => ({ paneKey: e.paneKey, menu: e.menu, question: e.question, title: e.question, place: 'proj' })))
    return () => h('div')
  },
})

interface Payload {
  enabled: boolean
  entries: { paneKey: string; sig: string; options: { index: number; label: string }[] | null }[]
}

describe('useMenuToast', () => {
  let sent: Payload[]
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    sent = []
    bridge.sync = (p) => sent.push(p as Payload)
    bridge.handler = null
    bridge.off = 0
    useExtrasStore().setNotify(notify())
  })

  it('syncs the queue with signature and options when everything is on', async () => {
    const q = useMenuQueueStore()
    q.report('a', { menu, question: 'Q?' }, 1)
    const w = mount(Probe, { global: { plugins: [i18n] } })
    await nextTick()
    const last = sent.at(-1)!
    expect(last.enabled).toBe(true)
    expect(last.entries[0]).toMatchObject({ paneKey: 'a', sig: menuSig(menu, 'Q?') })
    expect(last.entries[0]?.options?.length).toBeGreaterThan(0)
    w.unmount()
  })

  it.each([
    ['toast off', () => useExtrasStore().setNotify(notify('off'))],
    ['waiting kind off', () => useExtrasStore().setNotify(notify('on', false))],
    ['click menu off', () => void (useSettingsStore().clickMenu = false)],
    ['menu toast off', () => void (useSettingsStore().menuToast = false)],
  ])('sends nothing to show when %s', async (_name, off) => {
    const q = useMenuQueueStore()
    q.report('a', { menu, question: 'Q?' }, 1)
    off()
    const w = mount(Probe, { global: { plugins: [i18n] } })
    await nextTick()
    expect(sent.at(-1)).toMatchObject({ enabled: false, entries: [] })
    w.unmount()
  })

  it('re-syncs when the queue changes and turns itself off on unmount', async () => {
    const q = useMenuQueueStore()
    const w = mount(Probe, { global: { plugins: [i18n] } })
    await nextTick()
    const before = sent.length
    q.report('a', { menu, question: 'Q?' }, 1)
    await nextTick()
    expect(sent.length).toBe(before + 1)
    q.report('a', { menu, question: 'Q?' }, 5)
    await nextTick()
    expect(sent.length).toBe(before + 1)
    w.unmount()
    expect(sent.at(-1)).toMatchObject({ enabled: false, entries: [] })
    expect(bridge.off).toBe(1)
  })

  it('answers through the pane picker only when the signature still matches', async () => {
    const q = useMenuQueueStore()
    const picked: string[] = []
    q.register('a', async (item) => void picked.push(item.label))
    q.report('a', { menu, question: 'Q?' }, 1)
    const w = mount(Probe, { global: { plugins: [i18n] } })
    await nextTick()
    const first = menu.items[0]!
    bridge.handler?.({ type: 'answer', paneKey: 'a', sig: 'stale000', index: first.index })
    expect(picked).toEqual([])
    bridge.handler?.({ type: 'answer', paneKey: 'gone', sig: menuSig(menu, 'Q?'), index: first.index })
    expect(picked).toEqual([])
    bridge.handler?.({ type: 'answer', paneKey: 'a', sig: menuSig(menu, 'Q?'), index: 99 })
    expect(picked).toEqual([])
    bridge.handler?.({ type: 'answer', paneKey: 'a', sig: menuSig(menu, 'Q?'), index: first.index })
    await nextTick()
    expect(picked).toEqual([first.label])
    bridge.handler?.({ nonsense: true })
    expect(picked).toHaveLength(1)
    w.unmount()
  })

  it('open jumps to the pane when it still exists', async () => {
    const ws = useWorkspaceStore()
    const ui = useUiStore()
    const activated: string[] = []
    ws.activate = ((k: string) => activated.push(k)) as typeof ws.activate
    const view = vi.spyOn(ui, 'setView')
    ws.panes.push({ key: 'a' } as never)
    const w = mount(Probe, { global: { plugins: [i18n] } })
    bridge.handler?.({ type: 'open', paneKey: 'missing', sig: 's' })
    expect(activated).toEqual([])
    bridge.handler?.({ type: 'open', paneKey: 'a', sig: 's' })
    expect(activated).toEqual(['a'])
    expect(view).toHaveBeenCalledWith('workspace')
    w.unmount()
  })
})
