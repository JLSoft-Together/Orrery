import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../../i18n/en'
import vi18n from '../../../i18n/vi'
import { parseTermMenu } from '../../../lib/termMenu'
import { useTermMenu, type MenuTerm } from '../../../composables/useTermMenu'
import { useMenuQueueStore } from '../../../stores/menuQueue'
import MissionCard from './MissionCard.vue'

const DIR = join(__dirname, '..', '..', '..', 'lib', '__fixtures__', 'term-menu')
const lines = (name: string): string[] => readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: vi18n } })

const card = (paneKeys: string[]) => ({
  tabKey: 'tab-1',
  title: 'Project',
  named: false,
  current: false,
  focus: paneKeys[0]!,
  state: 'asked' as const,
  subagents: 0,
  needs: 1,
  failed: 0,
  tiles: paneKeys.map((paneKey, i) => ({ paneKey, title: paneKey, state: 'asked' as const, run: null, subagents: 0, box: { x: i / paneKeys.length, y: 0, w: 1 / paneKeys.length, h: 1 } })),
})

const props = (c: ReturnType<typeof card>) => ({
  card: c,
  section: 'home',
  roving: true,
  draggable: false,
  renaming: null,
  highlight: 0,
  snaps: {},
  flashes: {},
  drop: false,
  dragging: null,
  index: 1,
  bodyH: 160,
  rowCap: 6,
  motion: { enter: (_: Element, done: () => void) => done(), leave: (_: Element, done: () => void) => done() },
})

function screen(name: string): MenuTerm {
  const rows = lines(name)
  return {
    rows: rows.length,
    buffer: { active: { baseY: 0, viewportY: 0, cursorX: 0, cursorY: rows.length - 1, getLine: (y) => (rows[y] === undefined ? undefined : { translateToString: () => rows[y]! }) } },
    onWriteParsed: () => ({ dispose: () => undefined }),
    onScroll: () => ({ dispose: () => undefined }),
    onResize: () => ({ dispose: () => undefined }),
  }
}

const mountCard = (c: ReturnType<typeof card>) => mount(MissionCard, { props: props(c), global: { plugins: [i18n] } })

describe('MissionCard answer strip', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('shows no strip when no pane of the card has a menu', () => {
    const w = mountCard(card(['p1']))
    expect(w.find('[data-mission-ask]').exists()).toBe(false)
  })

  it('shows the question and one button per option, and the buttons send the key through the pane', async () => {
    const queue = useMenuQueueStore()
    const sent: string[] = []
    const term = screen('claude-ask-single')
    const api = useTermMenu({ term: () => term, enabled: () => false, observe: () => true, send: (k) => sent.push(k), focus: vi.fn() })
    api.attach(term)
    api.refresh()
    queue.register('p1', api.pick)
    queue.report('p1', api.seen.value)

    const w = mountCard(card(['p1']))
    expect(w.find('[data-mission-ask-text]').text()).toMatch(/\?$/)
    const buttons = w.findAll('[data-mission-ask] [data-term-menu-item]')
    expect(buttons.length).toBe(api.seen.value!.menu.items.length)
    await buttons[2]!.trigger('click')
    await vi.waitFor(() => expect(sent).toEqual(['3']))
  })

  it('clicking the strip does not open the card', async () => {
    const queue = useMenuQueueStore()
    const menu = parseTermMenu(lines('claude-ask-single'))!
    queue.register('p1', vi.fn().mockResolvedValue(undefined))
    queue.report('p1', { menu, question: 'Pick one?' })
    const w = mountCard(card(['p1']))
    await w.find('[data-mission-ask]').trigger('click')
    await w.find('[data-term-menu-item]').trigger('click')
    expect(w.emitted('open')).toBeUndefined()
  })

  it('with two asking panes shows the oldest and counts the rest', () => {
    const queue = useMenuQueueStore()
    const menu = parseTermMenu(lines('claude-ask-single'))!
    queue.report('p2', { menu, question: 'Second?' }, 20)
    queue.report('p1', { menu, question: 'First?' }, 10)
    const w = mountCard(card(['p1', 'p2']))
    expect(w.find('[data-mission-ask-text]').text()).toBe('First?')
    expect(w.find('[data-mission-ask-more]').text()).toBe('+1 more')
    expect(w.find('[data-mission-ask]').attributes('data-pane')).toBe('p1')
  })

  it('falls back to the menu kind when the question is unknown and drops the strip once answered', async () => {
    const queue = useMenuQueueStore()
    const menu = parseTermMenu(lines('claude-ask-single'))!
    queue.report('p1', { menu, question: '' })
    const w = mountCard(card(['p1']))
    expect(w.find('[data-mission-ask-text]').text()).toBe('Question')
    queue.report('p1', null)
    await w.vm.$nextTick()
    expect(w.find('[data-mission-ask]').exists()).toBe(false)
  })
})
