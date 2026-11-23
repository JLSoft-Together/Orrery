import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { createI18n } from 'vue-i18n'
import { createPinia } from 'pinia'
import en from '../../i18n/en'
import viMessages from '../../i18n/vi'
import type { CommitSessions } from '@ccm/shared'
import { chipTarget, COMMIT_BATCH, COMMIT_WAIT_MS, parseCommitSessions, useCommitSessions } from './commitSessions'
import { ChatFeed, type ChatTarget } from './feed'
import CommitChips from './CommitChips.vue'
import ChatView from './ChatView.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: viMessages } })
const chip = (over: Record<string, unknown> = {}) => ({ agent: 'claude', session: 'sid-1', label: 'api', cwd: 'C:/w/api', turn: 120, score: 1, strong: true, prompt: 'Fix login', at: 5, ...over })

afterEach(() => vi.useRealTimers())

describe('commit sessions', () => {
  it('parses replies and drops bad chips', () => {
    const got = parseCommitSessions({ ok: true, commits: { ABCDEF1: { chips: [chip(), chip({ agent: 'deepseek' }), chip({ session: 'b', strong: false, score: 0.5 })], more: 2 }, bad: { chips: 'x' }, empty: { chips: [] } } })
    expect([...got!.keys()]).toEqual(['abcdef1'])
    expect(got!.get('abcdef1')).toMatchObject({ more: 2, chips: [{ session: 'sid-1', strong: true }, { session: 'b', strong: false }] })
    expect(parseCommitSessions({ ok: false })).toBeNull()
    expect(chipTarget(got!.get('abcdef1')!.chips[0]!)).toEqual({ key: 'commit-chat:claude:sid-1', agent: 'claude', session: 'sid-1', cwd: 'C:/w/api' })
  })

  it('asks once per commit, in batches, only for the commits shown', async () => {
    vi.useFakeTimers()
    const call = vi.fn(async (req: Record<string, unknown>) => {
      const commits = req.commits as Array<{ hash: string }>
      return { ok: true, commits: Object.fromEntries(commits.filter((c) => c.hash.startsWith('a')).map((c) => [c.hash, { chips: [chip()], more: 0 }])) }
    })
    const root = ref<string | null>('C:/w/api')
    const api = useCommitSessions(root, call, true)
    const many = Array.from({ length: COMMIT_BATCH + 5 }, (_, i) => ({ hash: `${i === 0 ? 'a' : 'b'}${String(i).padStart(7, '0')}`, time: 100 + i }))
    api.want(many)
    api.want(many.slice(0, 3))
    await vi.advanceTimersByTimeAsync(COMMIT_WAIT_MS * 3)
    await flushPromises()
    expect(call).toHaveBeenCalledTimes(2)
    expect((call.mock.calls[0]![0].commits as unknown[]).length).toBe(COMMIT_BATCH)
    expect(call.mock.calls[0]![0]).toMatchObject({ op: 'commits', root: 'C:/w/api' })
    expect(api.sessions.value.get('a0000000')?.chips[0]?.session).toBe('sid-1')
    api.reset()
    expect(api.sessions.value.size).toBe(0)
    api.want(many.slice(0, 1))
    await vi.advanceTimersByTimeAsync(COMMIT_WAIT_MS)
    expect(call).toHaveBeenCalledTimes(3)
  })

  it('shows a strong chip, a faint chip with ? and the rest as +N', async () => {
    const sessions: CommitSessions = { chips: [chip() as CommitSessions['chips'][number], chip({ session: 's2', label: 'web', agent: 'codex', strong: false, score: 0.5 }) as CommitSessions['chips'][number]], more: 3 }
    const w = mount(CommitChips, { props: { sessions }, global: { plugins: [i18n] } })
    const chips = w.findAll('[data-commit-chip]')
    expect(chips.map((c) => c.attributes('data-commit-chip'))).toEqual(['strong', 'weak'])
    expect(chips[0]!.text()).toBe('Claude api')
    expect(chips[1]!.text()).toBe('Codex web?')
    expect(chips[1]!.attributes('title')).toContain('maybe made in this session, 50% of the files')
    expect(chips[0]!.attributes('title')).toContain('Prompt: Fix login')
    expect(w.get('[data-commit-chips-more]').text()).toBe('+3')
    await chips[1]!.trigger('click')
    expect(w.emitted('open')?.[0]?.[0]).toMatchObject({ session: 's2' })
  })
})

describe('chat seek', () => {
  const target: ChatTarget = { key: 'c1', agent: 'claude', session: 'sid-1', cwd: 'C:/w/api' }
  const me = (seq: number, text: string) => ({ id: `u:${seq}`, seq, rev: 0, k: 'me', text })
  const ai = (seq: number, text: string) => ({ id: `a:${seq}`, seq, rev: 0, k: 'ai', text })

  it('a pinned page does not poll and going live drops the pinned items', async () => {
    vi.useFakeTimers()
    const call = vi.fn(async (req: Record<string, unknown>) => {
      if (req.op === 'seek') return { ok: true, gen: 'g', agent: 'claude', items: [me(0, 'one'), ai(5, 'first')], older: null, newer: 50, turn: 0 }
      if (req.op === 'page') return { ok: true, gen: 'g', agent: 'claude', items: [me(90, 'three')], older: 90 }
      return { ok: true, gen: 'g', items: [] }
    })
    const states: boolean[] = []
    const feed = new ChatFeed(target, call, (s) => states.push(!!s.pinned), 100)
    feed.start(0)
    await flushPromises()
    expect(feed.current.pinned).toBe(true)
    await vi.advanceTimersByTimeAsync(500)
    expect(call.mock.calls.filter((c) => c[0].op === 'poll')).toHaveLength(0)
    await feed.reload()
    expect(feed.current.pinned).toBe(false)
    expect([...feed.current.log.items.keys()]).toEqual(['u:90'])
    await vi.advanceTimersByTimeAsync(150)
    expect(call.mock.calls.some((c) => c[0].op === 'poll')).toBe(true)
    feed.stop()
  })

  it('opens at the focused turn and marks it', async () => {
    const items = [me(0, 'one'), ai(5, 'first'), me(40, 'Fix login'), ai(45, 'done')]
    const call = vi.fn(async (req: Record<string, unknown>) => (req.op === 'seek' ? { ok: true, gen: 'g', agent: 'claude', items, older: null, newer: 80, turn: req.turn } : { ok: true }))
    const w = mount(ChatView, { props: { target, active: true, agentLabel: 'Claude', call, focusTurn: 40 }, global: { plugins: [i18n, createPinia()] }, attachTo: document.body })
    await flushPromises()
    expect(call).toHaveBeenCalledWith(expect.objectContaining({ op: 'seek', turn: 40, session: 'sid-1' }))
    expect(w.find('[data-chat-pinned]').exists()).toBe(true)
    expect(w.get('[data-chat-focus]').attributes('data-id')).toBe('u:40')
    w.unmount()
  })
})
