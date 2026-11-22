import { describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import viMessages from '../../i18n/vi'
import { parseMarkdown, safeHref } from './markdown'
import { anchorTop, applyItems, applyPage, CHAT_WINDOW, chatView, emptyLog, joinRoot, nearBottom, parseChatItem, parseChatPage, parseChatPoll, topAction, windowOf } from './model'
import { ChatFeed, sameTarget, type ChatTarget } from './feed'
import ChatMarkdown from './ChatMarkdown.vue'
import ChatView from './ChatView.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: viMessages } })
const target: ChatTarget = { key: 'p1', agent: 'claude', session: 'sid-1', cwd: 'C:\\work\\api' }

const me = (id: string, seq: number, text: string) => ({ id, seq, rev: 0, k: 'me' as const, text })
const ai = (id: string, seq: number, text: string, rev = 0) => ({ id, seq, rev, k: 'ai' as const, text })

describe('chat model', () => {
  it('parses items and drops unknown kinds or missing ids', () => {
    expect(parseChatItem(me('a', 1, 'hi'))).toMatchObject({ k: 'me', text: 'hi' })
    expect(parseChatItem({ id: 'x', seq: 1, k: 'html', text: '<b>' })).toBeNull()
    expect(parseChatItem({ seq: 1, k: 'me', text: 'x' })).toBeNull()
    expect(parseChatItem({ id: 't', seq: 2, k: 'tools', sum: { edit: 2, bogus: 1 }, items: [{ tool: 'Edit', cat: 'edit', target: 'a.ts' }, { nope: 1 }] })).toMatchObject({
      k: 'tools',
      sum: { edit: 2, other: 1 },
      items: [{ tool: 'Edit', cat: 'edit', target: 'a.ts' }],
    })
  })

  it('parses page and poll replies, refusing failed ones', () => {
    expect(parseChatPage({ ok: false, code: 'internal' })).toBeNull()
    expect(parseChatPage({ ok: true, gen: 'g:0', agent: 'claude', items: [me('a', 1, 'q')], older: 40 })).toMatchObject({ gen: 'g:0', older: 40 })
    expect(parseChatPoll({ ok: true, reset: true })).toEqual({ reset: true })
    expect(parseChatPoll({ ok: true, gen: 'g:0', items: [ai('b', 2, 'x')] })).toMatchObject({ reset: false, gen: 'g:0' })
    expect(parseChatPoll({ ok: true })).toBeNull()
  })

  it('upserts by rev, resets on a new gen and keeps files after their turn', () => {
    let log = applyPage(emptyLog(), { gen: 'g', agent: 'claude', items: [me('m1', 1, 'q'), ai('a1', 2, 'old')], older: 10 }, false)
    log = applyItems(log, 'g', [ai('a1', 2, 'new', 1), { id: 'f1', seq: 3, rev: 0, k: 'files', root: '', files: [] }, me('m2', 4, 'next')]).log
    expect(chatView(log).map((i) => i.id)).toEqual(['m1', 'a1', 'f1', 'm2'])
    expect(chatView(log).find((i) => i.id === 'a1')).toMatchObject({ text: 'new' })
    expect(applyItems(log, 'other', []).stale).toBe(true)
    const older = applyPage(log, { gen: 'g', agent: null, items: [me('m0', 0, 'first')], older: null }, true)
    expect(older.older).toBeNull()
    expect(chatView(older)[0]!.id).toBe('m0')
    expect(applyPage(log, { gen: 'h', agent: null, items: [], older: null }, false).items.size).toBe(0)
  })

  it('windows the list and decides what scrolling up does', () => {
    const all = Array.from({ length: 100 }, (_, i) => i)
    expect(windowOf(all, CHAT_WINDOW)).toHaveLength(60)
    expect(windowOf(all, CHAT_WINDOW)[0]).toBe(40)
    expect(windowOf(all, 500)).toHaveLength(100)
    expect(topAction(100, 60, null, false)).toBe('grow')
    expect(topAction(60, 60, 500, false)).toBe('fetch')
    expect(topAction(60, 60, 500, true)).toBe('none')
    expect(topAction(60, 60, null, false)).toBe('none')
    expect(anchorTop(10, 1000, 1600)).toBe(610)
    expect(nearBottom(560, 1000, 400)).toBe(true)
    expect(nearBottom(100, 1000, 400)).toBe(false)
  })

  it('joins a project root with a relative path', () => {
    expect(joinRoot('C:\\work\\api', 'src/a.ts')).toBe('C:\\work\\api\\src\\a.ts')
    expect(joinRoot('/home/x/api/', 'src/a.ts')).toBe('/home/x/api/src/a.ts')
  })
})

describe('chat markdown', () => {
  it('never turns raw HTML into markup and only keeps safe links', () => {
    const blocks = parseMarkdown('<img src=x onerror=alert(1)> **bold** [x](javascript:alert(1)) [ok](https://a.b)')
    const html = mount(ChatMarkdown, { props: { text: '<script>alert(1)</script>\n\n<b>hi</b> [x](javascript:alert(1))' }, global: { plugins: [i18n] } }).html()
    expect(blocks[0]!.k).toBe('p')
    expect(html).not.toContain('<script')
    expect(html).not.toContain('<b>')
    expect(html).toContain('&lt;script&gt;')
    expect(html).not.toContain('javascript:')
    expect(safeHref('https://a.b')).toBe('https://a.b')
    expect(safeHref('file:///etc/passwd')).toBeNull()
  })

  it('copies a code block', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    const w = mount(ChatMarkdown, { props: { text: '```ts\nconst a = 1\n```' }, global: { plugins: [i18n] } })
    await w.get('[data-chat-copy]').trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledWith('const a = 1')
    expect(w.get('[data-chat-copy]').text()).toBe('Copied')
  })
})

describe('chat feed', () => {
  it('pages, polls new items, re-pages on reset and unwatches on stop', async () => {
    vi.useFakeTimers()
    const calls: Array<Record<string, unknown>> = []
    const replies: unknown[] = [
      { ok: true, gen: 'g', agent: 'claude', items: [me('m1', 1, 'q')], older: null },
      { ok: true, gen: 'g', items: [ai('a1', 2, 'answer')] },
      { ok: true, reset: true },
      { ok: true, gen: 'h', agent: 'claude', items: [me('m9', 1, 'fresh')], older: null },
    ]
    const call = vi.fn(async (req: Record<string, unknown>) => {
      calls.push(req)
      return req.op === 'unwatch' ? { ok: true } : replies.shift()
    })
    const seen: string[][] = []
    const feed = new ChatFeed(target, call, (s) => seen.push(chatView(s.log).map((i) => i.id)), 100)
    feed.start()
    await vi.advanceTimersByTimeAsync(0)
    expect(feed.current.status).toBe('ready')
    await vi.advanceTimersByTimeAsync(100)
    expect(chatView(feed.current.log).map((i) => i.id)).toEqual(['m1', 'a1'])
    expect(calls[1]).toMatchObject({ op: 'poll', gen: 'g', session: 'sid-1' })
    await vi.advanceTimersByTimeAsync(100)
    expect(chatView(feed.current.log).map((i) => i.id)).toEqual(['m9'])
    feed.stop()
    expect(calls.at(-1)).toEqual({ op: 'unwatch', key: 'p1' })
    const n = calls.length
    await vi.advanceTimersByTimeAsync(500)
    expect(calls.length).toBe(n)
    vi.useRealTimers()
    expect(sameTarget(target, { ...target })).toBe(true)
    expect(sameTarget(target, { ...target, session: 'x' })).toBe(false)
  })
})

describe('ChatView', () => {
  function pageOf(n: number, older: number | null) {
    const items = Array.from({ length: n }, (_, i) => (i % 2 ? ai(`a${i}`, i + 100, `answer ${i}`) : me(`m${i}`, i + 100, `question ${i}`)))
    return { ok: true, gen: 'g', agent: 'claude', items, older }
  }

  it('renders a window of 60, grows on scroll up and then fetches older pages', async () => {
    const call = vi.fn(async (req: Record<string, unknown>) => {
      if (req.op === 'page' && req.before === undefined) return pageOf(80, 7)
      if (req.op === 'page') return { ok: true, gen: 'g', agent: 'claude', items: [me('old', 1, 'oldest question')], older: null }
      return { ok: true, gen: 'g', items: [] }
    })
    const w = mount(ChatView, { props: { target, active: true, agentLabel: 'Claude', call }, global: { plugins: [i18n] }, attachTo: document.body })
    await flushPromises()
    expect(w.findAll('[data-chat-item]')).toHaveLength(60)
    const el = w.get('[data-chat-scroll]').element as HTMLElement
    Object.defineProperty(el, 'scrollHeight', { configurable: true, get: () => w.findAll('[data-chat-item]').length * 50 })
    Object.defineProperty(el, 'clientHeight', { configurable: true, get: () => 400 })
    el.scrollTop = 0
    await w.get('[data-chat-scroll]').trigger('scroll')
    await flushPromises()
    expect(w.findAll('[data-chat-item]')).toHaveLength(80)
    expect(el.scrollTop).toBe(1000)
    el.scrollTop = 0
    await w.get('[data-chat-scroll]').trigger('scroll')
    await flushPromises()
    expect(call).toHaveBeenCalledWith(expect.objectContaining({ op: 'page', before: 7 }))
    expect(w.findAll('[data-chat-item]')).toHaveLength(81)
    expect(el.scrollTop).toBe(50)
    expect(w.find('[data-chat-top]').exists()).toBe(true)
    w.unmount()
  })

  it('folds tools into one line, opens files and unwatches when hidden', async () => {
    const call = vi.fn(async (req: Record<string, unknown>) => {
      if (req.op === 'page')
        return {
          ok: true,
          gen: 'g',
          agent: 'claude',
          older: null,
          items: [
            me('m1', 1, 'Fix retry'),
            { id: 't1', seq: 2, rev: 0, k: 'tools', sum: { edit: 2, run: 1 }, items: [{ tool: 'Edit', cat: 'edit', target: 'src/api.ts' }] },
            ai('a1', 3, 'Retry is now **500ms**.'),
            { id: 'f1', seq: 4, rev: 0, k: 'files', root: '/w', files: [{ path: 'src/api.ts', add: 3, del: 1 }] },
          ],
        }
      return { ok: true }
    })
    const w = mount(ChatView, { props: { target, active: true, agentLabel: 'Claude', call }, global: { plugins: [i18n] } })
    await flushPromises()
    expect(w.get('[data-chat-me]').text()).toContain('Fix retry')
    expect(w.get('[data-chat-ai] strong').text()).toBe('500ms')
    expect(w.get('[data-chat-tools-toggle]').text()).toContain('Edited 2 files, Ran 1 command')
    expect(w.find('[data-chat-tools-list]').exists()).toBe(false)
    await w.get('[data-chat-tools-toggle]').trigger('click')
    expect(w.get('[data-chat-tools-list]').text()).toContain('src/api.ts')
    await w.get('[data-chat-file]').trigger('click')
    expect(w.emitted('openFile')?.[0]).toEqual(['/w/src/api.ts'])
    expect(w.find('textarea, input').exists()).toBe(false)
    await w.setProps({ active: false })
    expect(call).toHaveBeenLastCalledWith({ op: 'unwatch', key: 'p1' })
    w.unmount()
  })

  it('waits for a session before reading', () => {
    const call = vi.fn()
    const w = mount(ChatView, { props: { target: { ...target, session: '' }, active: true, agentLabel: 'Claude', call }, global: { plugins: [i18n] } })
    expect(w.find('[data-chat-nosession]').exists()).toBe(true)
    expect(call).not.toHaveBeenCalled()
  })
})
