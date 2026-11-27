import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h } from 'vue'
import { i18n } from '../../../../i18n'
import { useMenuQueueStore } from '@features/click-menu'
import { now } from '../../../../core/lib/format'
import { useToolSpansStore } from '../../../../core/stores/toolSpans'
import { useUiStore } from '../../../../core/stores/ui'
import { useWorkspaceStore } from '../../../workspace/stores/workspace'
import { PEEK_DELAY_MS, PEEK_GRACE_MS } from '../../lib/railPeek'
import type { LiveRow } from '../../lib/railLive'
import RailPeek from './RailPeek.vue'
import RailRowItem from './RailRowItem.vue'

type Over = { buttons?: number; pointerType?: string; relatedTarget?: EventTarget | null }

function fire(el: Element | Document | Window, type: string, init: Over = {}): void {
  const e = new Event(type, { bubbles: true, cancelable: true })
  Object.assign(e, { pointerType: 'mouse', buttons: 0, relatedTarget: null, ...init })
  el.dispatchEvent(e)
}

function row(key: string, over: Partial<LiveRow> & { agent?: string } = {}, kind: 'agent' | 'shell' | 'other' = 'agent'): LiveRow {
  return {
    key,
    paneKey: key,
    tabKey: `t-${key}`,
    clusterKey: 'c',
    clusterName: 'proj',
    tone: 0,
    agent: kind === 'agent' ? 'Claude' : 'PowerShell',
    title: `title-${key}`,
    state: 'working',
    attn: null,
    since: Date.now() - 65_000,
    subagents: 0,
    model: null,
    ctxPct: null,
    tool: 'Bash',
    waitText: null,
    stateText: null,
    shell: kind === 'shell',
    focused: false,
    src: { kind, pane: { key, kind: 'term', terminalId: `term-${key}`, cwd: 'D:\\work\\web\\app' } } as unknown as LiveRow['src'],
    ...over,
  } as LiveRow
}

const probes: VueWrapper[] = []

function mountRail(rows: Record<string, LiveRow>) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const onOpen = vi.fn()
  useWorkspaceStore()
  useUiStore()
  useToolSpansStore()
  useMenuQueueStore()
  const probe = mount(defineComponent({ render: () => h('i') }), { global: { plugins: [pinia, i18n] } })
  const base = vi.getTimerCount()
  probes.push(probe)
  const Harness = defineComponent({
    setup() {
      return () =>
        h('nav', { 'data-live-rail': '' }, [
          h('div', { 'data-rail-list': '' }, Object.values(rows).map((r) => h('div', { 'data-row-key': r.key, key: r.key }, [h('button', { 'data-rail-open': '', title: `open ${r.key}` })]))),
          h(RailPeek, { rowOf: (k: string) => rows[k], onOpen }),
        ])
    },
  })
  const w = mount(Harness, { attachTo: document.body, global: { plugins: [pinia, i18n] } })
  return { w, onOpen, base }
}

const rowEl = (key: string): HTMLElement => document.querySelector<HTMLElement>(`[data-row-key="${key}"]`)!
const card = (): HTMLElement | null => document.querySelector<HTMLElement>('[data-rail-peek]')

async function hover(key: string, ms = PEEK_DELAY_MS): Promise<void> {
  fire(rowEl(key), 'pointerover')
  await vi.advanceTimersByTimeAsync(ms)
  await flushPromises()
}

describe('RailPeek', () => {
  let w: VueWrapper | null = null

  beforeEach(() => {
    vi.useFakeTimers()
    localStorage.clear()
  })
  afterEach(() => {
    w?.unmount()
    w = null
    for (const p of probes.splice(0)) p.unmount()
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('mounts no popover and no timer until a row is hovered', () => {
    const m = mountRail({ a: row('a') })
    w = m.w
    expect(card()).toBeNull()
    expect(vi.getTimerCount()).toBe(m.base)
  })

  it('opens one popover after 400 ms, drops the native title and links aria-describedby', async () => {
    w = mountRail({ a: row('a'), b: row('b') }).w
    const btn = rowEl('a').querySelector('button')!
    await hover('a', PEEK_DELAY_MS - 1)
    expect(card()).toBeNull()
    await vi.advanceTimersByTimeAsync(1)
    await flushPromises()
    expect(document.querySelectorAll('[data-rail-peek]')).toHaveLength(1)
    expect(btn.hasAttribute('title')).toBe(false)
    expect(btn.getAttribute('aria-describedby')).toBe(card()!.id)
    expect(rowEl('b').querySelector('button')!.getAttribute('title')).toBe('open b')
  })

  it('restores the title and closes when the pointer leaves', async () => {
    w = mountRail({ a: row('a') }).w
    const btn = rowEl('a').querySelector('button')!
    await hover('a')
    fire(rowEl('a'), 'pointerout', { relatedTarget: document.body })
    await vi.advanceTimersByTimeAsync(PEEK_GRACE_MS + 200)
    await flushPromises()
    expect(card()).toBeNull()
    expect(btn.getAttribute('title')).toBe('open a')
    expect(btn.hasAttribute('aria-describedby')).toBe(false)
  })

  it('stays open while the pointer crosses onto the popover', async () => {
    w = mountRail({ a: row('a') }).w
    await hover('a')
    fire(rowEl('a'), 'pointerout', { relatedTarget: document.body })
    card()!.dispatchEvent(new Event('pointerenter'))
    await vi.advanceTimersByTimeAsync(PEEK_GRACE_MS * 3)
    expect(card()).not.toBeNull()
  })

  it('closes on Escape, on scroll, on pointerdown (drag) and on resize', async () => {
    w = mountRail({ a: row('a') }).w
    await hover('a')
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
    await vi.advanceTimersByTimeAsync(300)
    await flushPromises()
    expect(card()).toBeNull()

    await hover('a')
    fire(document.querySelector('[data-rail-list]')!, 'scroll')
    await vi.advanceTimersByTimeAsync(300)
    expect(card()).toBeNull()

    await hover('a')
    fire(rowEl('a'), 'pointerdown')
    await vi.advanceTimersByTimeAsync(300)
    expect(card()).toBeNull()

    await hover('a')
    window.dispatchEvent(new Event('resize'))
    await vi.advanceTimersByTimeAsync(300)
    expect(card()).toBeNull()
  })

  it('does not open while a button is held (dragging)', async () => {
    const m = mountRail({ a: row('a') })
    w = m.w
    fire(rowEl('a'), 'pointerover', { buttons: 1 })
    await vi.advanceTimersByTimeAsync(PEEK_DELAY_MS * 2)
    expect(card()).toBeNull()
    expect(vi.getTimerCount()).toBe(m.base)
  })

  it('shows tool, its running time, the status word and an open button for an agent', async () => {
    w = mountRail({ a: row('a') }).w
    const spans = useToolSpansStore()
    spans.upsert({ id: 's1', terminalId: 'term-a', agentId: 'main', tool: 'Bash', start: new Date(Date.now() - 12_000).toISOString() })
    now.value = Date.now()
    await hover('a')
    const text = card()!.textContent ?? ''
    expect(card()!.querySelector('[data-peek-tool]')!.textContent).toContain('Bash')
    expect(card()!.querySelector('[data-peek-tool]')!.textContent).toMatch(/running \d+s/)
    expect(card()!.querySelector('[data-peek-status]')!.textContent).toContain('Working')
    expect(text).toContain('title-a')
    expect(card()!.querySelector('[data-peek-open]')).not.toBeNull()
  })

  it('shows the masked menu question while waiting and no reply control', async () => {
    const waiting = row('a', { state: 'asked', attn: 'question', tool: null })
    w = mountRail({ a: waiting }).w
    const queue = useMenuQueueStore()
    queue.report('a', { menu: { kind: 'ask-single', items: [], cursor: null } as never, question: 'Use token=abcd1234efgh for deploy?' })
    await hover('a')
    const q = card()!.querySelector('[data-peek-question]')!.textContent ?? ''
    expect(q).toContain('Use')
    expect(q).not.toContain('abcd1234efgh')
    expect(card()!.querySelectorAll('input, textarea')).toHaveLength(0)
    expect(card()!.querySelectorAll('button')).toHaveLength(1)
  })

  it('says there is no activity for an idle agent', async () => {
    w = mountRail({ a: row('a', { state: 'idle', tool: null, since: 0 }) }).w
    await hover('a')
    expect(card()!.querySelector('[data-peek-empty]')!.textContent).toBe('No activity yet')
  })

  it('shows name, kind and short cwd for a shell pane, with no tool line', async () => {
    w = mountRail({ a: row('a', {}, 'shell') }).w
    await hover('a')
    const kind = card()!.querySelector('[data-peek-kind]')!.textContent ?? ''
    expect(kind).toContain('PowerShell')
    expect(kind).toContain('…/web/app')
    expect(card()!.querySelector('[data-peek-tool]')).toBeNull()
  })

  it('open button emits open with the row and closes', async () => {
    const m = mountRail({ a: row('a') })
    w = m.w
    await hover('a')
    card()!.querySelector<HTMLButtonElement>('[data-peek-open]')!.click()
    await vi.advanceTimersByTimeAsync(300)
    expect(m.onOpen).toHaveBeenCalledTimes(1)
    expect(m.onOpen.mock.calls[0]![0].key).toBe('a')
    expect(card()).toBeNull()
  })

})

describe('RailRowItem i key', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('toggles the peek with i and keeps other keys flowing to the rail', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const rows = { a: row('a') }
    const Harness = defineComponent({
      setup: () => () =>
        h('nav', { 'data-live-rail': '' }, [h(RailRowItem, { row: rows.a, tabStop: true }), h(RailPeek, { rowOf: (k: string) => (rows as Record<string, LiveRow>)[k] })]),
    })
    const w = mount(Harness, { attachTo: document.body, global: { plugins: [pinia, i18n] } })
    const btn = document.querySelector<HTMLButtonElement>('[data-rail-open]')!
    btn.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', bubbles: true, cancelable: true }))
    await flushPromises()
    expect(card()).not.toBeNull()
    expect(btn.getAttribute('aria-describedby')).toBe(card()!.id)
    btn.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', bubbles: true, cancelable: true }))
    await flushPromises()
    await new Promise((r) => setTimeout(r, 250))
    expect(card()).toBeNull()
    const row0 = w.findComponent(RailRowItem)
    btn.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }))
    expect(row0.emitted('key')).toHaveLength(1)
    w.unmount()
  })

  it('ignores the auto repeat of a held i key so the card does not flicker', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const rows = { a: row('a') }
    const Harness = defineComponent({
      setup: () => () =>
        h('nav', { 'data-live-rail': '' }, [h(RailRowItem, { row: rows.a, tabStop: true }), h(RailPeek, { rowOf: (k: string) => (rows as Record<string, LiveRow>)[k] })]),
    })
    const w = mount(Harness, { attachTo: document.body, global: { plugins: [pinia, i18n] } })
    const btn = document.querySelector<HTMLButtonElement>('[data-rail-open]')!
    btn.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', bubbles: true, cancelable: true }))
    await flushPromises()
    expect(card()).not.toBeNull()
    const held = new KeyboardEvent('keydown', { key: 'i', repeat: true, bubbles: true, cancelable: true })
    btn.dispatchEvent(held)
    await flushPromises()
    await new Promise((r) => setTimeout(r, 250))
    expect(held.defaultPrevented).toBe(true)
    expect(card()).not.toBeNull()
    expect(w.findComponent(RailRowItem).emitted('key')).toBeUndefined()
    w.unmount()
  })
})
