import { describe, expect, it, vi } from 'vitest'
import { REMOTE_CHAT_TOOL_CATS } from '../../../../packages/shared/src/remote'
import { RemoteCommandError, type JsonMap } from '../remote/types'
import { MachineData } from '../remote/machineData'
import { ChatFeed, type FeedState } from './chatFeed'
import { cachedLog, cacheLog, forgetLogs } from './chatCache'
import { chatRows, filterRows, liveOf, shortAgo, sortRows } from './chatInbox'
import { applyItems, applyPage, chatSupported, chatView, emptyLog, fileTotals, parseChatItem, parseChatLast, parseChatPage, parseMenu, toolCat, toolParts } from './chatModel'
import { settleOutbox } from './chatOutbox'
import { parseInline, parseMarkdown, safeHref } from './markdown'
import { parseWorkspace } from './workspaceModel'

const me = (id: string, seq: number, text = 'hi', rev = 0) => ({ id, k: 'me', seq, rev, text })
const ai = (id: string, seq: number, text = 'ok', rev = 0) => ({ id, k: 'ai', seq, rev, text })

describe('chat items', () => {
  it('parses every kind, keeps unknown tool groups as other and drops broken rows', () => {
    expect(parseChatItem(me('u1', 1))).toMatchObject({ k: 'me', text: 'hi' })
    expect(parseChatItem({ id: 't1', k: 'tools', seq: 2, rev: 1, sum: { edit: 2, weird: 3, read: 0 }, items: [{ tool: 'Edit', cat: 'edit', target: 'a.ts', err: true }, { cat: 'run' }] })).toEqual({
      id: 't1',
      k: 'tools',
      seq: 2,
      rev: 1,
      sum: { edit: 2, other: 3 },
      items: [{ tool: 'Edit', cat: 'edit', target: 'a.ts', err: true }],
    })
    expect(parseChatItem({ id: 'f', k: 'files', seq: 3, rev: 0, root: 'C:\\api', files: [{ path: 'a.ts', add: 2, del: 1 }, { path: '~/x', out: true }], more: 2 })).toMatchObject({ files: [{ path: 'a.ts', add: 2, del: 1 }, { path: '~/x', out: true }], more: 2 })
    expect(parseChatItem({ id: 's', k: 'sys', seq: 4, rev: 0, kind: 'nope' })).toMatchObject({ kind: 'error' })
    expect(parseChatItem({ id: 'x', k: 'thinking', seq: 1, rev: 0 })).toBeNull()
    expect(parseChatItem({ k: 'me', seq: 1, text: 'a' })).toBeNull()
    expect(parseChatItem({ id: 'a', k: 'ai', seq: -1, text: 'a' })).toBeNull()
    expect(toolCat('search')).toBe('search')
    expect(toolCat('mcp')).toBe('other')
  })

  it('upserts by id and rev, resets on a new gen, keeps the older cursor of loaded pages', () => {
    let log = applyPage(emptyLog(), parseChatPage({ gen: 'g1', agent: 'claude', items: [me('u2', 20), ai('a2', 30, 'part', 1)], older: 20 })!, false)
    expect(log.gen).toBe('g1')
    expect(log.older).toBe(20)
    log = applyPage(log, parseChatPage({ gen: 'g1', items: [me('u1', 5), ai('a1', 10)], older: null })!, true)
    expect(log.items.size).toBe(4)
    expect(log.older).toBeNull()
    log = applyPage(log, parseChatPage({ gen: 'g1', items: [ai('a2', 30, 'full', 2)], older: 20 })!, false)
    expect(log.older).toBeNull()
    let r = applyItems(log, 'g1', [parseChatItem(ai('a2', 30, 'old', 1))!])
    expect((r.log.items.get('a2') as { text: string }).text).toBe('full')
    r = applyItems(log, 'g1', [parseChatItem(ai('a2', 30, 'newer', 3))!])
    expect((r.log.items.get('a2') as { text: string }).text).toBe('newer')
    expect(applyItems(log, 'g2', []).stale).toBe(true)
    expect(applyItems(emptyLog(), 'g1', []).stale).toBe(true)
    const fresh = applyPage(log, parseChatPage({ gen: 'g2', items: [me('n', 1)], older: null })!, false)
    expect([...fresh.items.keys()]).toEqual(['n'])
  })

  it('orders by seq and draws the files card at the end of its turn', () => {
    const log = applyPage(
      emptyLog(),
      parseChatPage({
        gen: 'g',
        items: [me('u1', 100), { id: 'f1', k: 'files', seq: 101, rev: 0, root: 'r', files: [{ path: 'a' }] }, { id: 't1', k: 'tools', seq: 150, rev: 0, sum: { read: 1 }, items: [] }, ai('a1', 200), me('u2', 300), ai('a2', 400)],
        older: null,
      })!,
      false,
    )
    expect(chatView(log).map((x) => x.id)).toEqual(['u1', 't1', 'a1', 'f1', 'u2', 'a2'])
  })

  it('builds the tool sentence parts in a fixed order and sums file changes', () => {
    expect(toolParts({ run: 2, edit: 3, other: 0 })).toEqual([
      { cat: 'edit', n: 3 },
      { cat: 'run', n: 2 },
    ])
    expect(REMOTE_CHAT_TOOL_CATS[0]).toBe('edit')
    expect(fileTotals([{ path: 'a', add: 2, del: 1 }, { path: 'b', add: 14 }])).toEqual({ add: 16, del: 1 })
  })

  it('reads the menu and chat.last, full replaces, partial upserts and null removes', () => {
    expect(parseMenu({ sig: 'ab', kind: 'permission', agent: 'claude', question: 'Go?', items: [{ index: 1, label: 'Yes', selected: true }, { index: 3, label: 'Other', other: true }, { label: 'x' }], numbered: true, confident: true, since: 5 })).toEqual({
      sig: 'ab',
      kind: 'permission',
      agent: 'claude',
      question: 'Go?',
      items: [{ index: 1, label: 'Yes', selected: true }, { index: 3, label: 'Other', other: true }],
      numbered: true,
      confident: true,
      since: 5,
    })
    expect(parseMenu({ sig: 'ab', items: [] })).toBeNull()
    expect(parseMenu({ items: [{ index: 1, label: 'a' }] })).toBeNull()
    let last = parseChatLast({ full: true, items: [{ paneKey: 'p1', text: 'Done.', at: 9 }, { paneKey: 'p2', text: null }] }, { old: { text: 'x', at: null } })
    expect(last).toEqual({ p1: { text: 'Done.', at: 9 } })
    last = parseChatLast({ items: [{ paneKey: 'p1', text: null }, { paneKey: 'p3', text: 'Hi' }] }, last)
    expect(last).toEqual({ p3: { text: 'Hi', at: null } })
    expect(chatSupported({ chat: 1 })).toBe(true)
    expect(chatSupported({})).toBe(false)
    expect(chatSupported(null)).toBe(false)
  })
})

describe('markdown', () => {
  it('parses blocks: paragraphs, headings, lists, fences, quotes, tables, rules', () => {
    const blocks = parseMarkdown('# Title\n\nLine one\nline two\n\n- a\n- b\n\n2. x\n3. y\n\n```ts\nconst a = 1\n```\n\n> quoted\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n---')
    expect(blocks.map((b) => b.k)).toEqual(['h', 'p', 'list', 'list', 'code', 'quote', 'table', 'hr'])
    expect(blocks[2]).toMatchObject({ ordered: false, items: [[{ k: 'text', v: 'a' }], [{ k: 'text', v: 'b' }]] })
    expect(blocks[3]).toMatchObject({ ordered: true, start: 2 })
    expect(blocks[4]).toEqual({ k: 'code', lang: 'ts', text: 'const a = 1' })
    expect(blocks[6]).toMatchObject({ k: 'table', rows: [[[{ k: 'text', v: '1' }], [{ k: 'text', v: '2' }]]] })
  })

  it('keeps an unclosed fence as code and never yields raw html', () => {
    expect(parseMarkdown('```\n<b>x</b>')).toEqual([{ k: 'code', lang: '', text: '<b>x</b>' }])
    expect(parseMarkdown('<script>alert(1)</script>')).toEqual([{ k: 'p', c: [{ k: 'text', v: '<script>alert(1)</script>' }] }])
  })

  it('parses inline code, bold, italic, safe links only', () => {
    expect(parseInline('a `b*c` **d** _e_ snake_case')).toEqual([
      { k: 'text', v: 'a ' },
      { k: 'code', v: 'b*c' },
      { k: 'text', v: ' ' },
      { k: 'strong', c: [{ k: 'text', v: 'd' }] },
      { k: 'text', v: ' ' },
      { k: 'em', c: [{ k: 'text', v: 'e' }] },
      { k: 'text', v: ' snake_case' },
    ])
    const links = parseInline('[ok](https://x.dev) [bad](javascript:alert(1))')
    expect(links[0]).toEqual({ k: 'link', href: 'https://x.dev', c: [{ k: 'text', v: 'ok' }] })
    expect(links.filter((x) => x.k === 'link')).toHaveLength(1)
    expect(JSON.stringify(links)).not.toContain('javascript')
    expect(safeHref('data:text/html,x')).toBeNull()
  })
})

describe('ChatFeed', () => {
  function setup(reply: (type: string, fields: JsonMap) => Promise<JsonMap>) {
    const calls: Array<{ type: string; fields: JsonMap }> = []
    const states: FeedState[] = []
    const feed = new ChatFeed('p1', {
      command: (type, fields) => {
        calls.push({ type, fields })
        return reply(type, fields)
      },
      onChange: (s) => states.push(s),
    })
    return { feed, calls, states }
  }

  it('watches, applies live items, re-watches on a gen change or reset, unwatches on stop', async () => {
    const { feed, calls } = setup(async () => ({ ok: true, paneKey: 'p1', agent: 'claude', gen: 'g1', items: [me('u1', 1)], older: null }))
    await feed.watch()
    expect(feed.current.status).toBe('ready')
    feed.handle({ t: 'chat.items', paneKey: 'p1', gen: 'g1', items: [ai('a1', 2)] })
    feed.handle({ t: 'chat.items', paneKey: 'p9', gen: 'g1', items: [ai('x', 3)] })
    expect(feed.current.log.items.size).toBe(2)
    feed.handle({ t: 'chat.items', paneKey: 'p1', gen: 'g2', items: [] })
    await vi.waitFor(() => expect(calls.filter((c) => c.type === 'chat.watch').length).toBe(2))
    feed.handle({ t: 'chat.reset', paneKey: 'p1', gen: 'g3' })
    await vi.waitFor(() => expect(calls.filter((c) => c.type === 'chat.watch').length).toBe(3))
    feed.stop()
    expect(calls.at(-1)).toEqual({ type: 'chat.unwatch', fields: { paneKey: 'p1' } })
    feed.handle({ t: 'chat.items', paneKey: 'p1', gen: 'g1', items: [ai('late', 9)] })
    expect(feed.current.log.items.has('late')).toBe(false)
  })

  it('loads older pages with gen and before, and stops at the top', async () => {
    const { feed, calls } = setup(async (type) =>
      type === 'chat.watch' ? { gen: 'g', items: [me('u2', 50)], older: 50 } : { gen: 'g', items: [me('u1', 10)], older: null },
    )
    await feed.watch()
    await feed.loadOlder()
    expect(calls[1]).toEqual({ type: 'chat.page', fields: { paneKey: 'p1', gen: 'g', before: 50 } })
    expect(feed.current.log.older).toBeNull()
    await feed.loadOlder()
    expect(calls).toHaveLength(2)
  })

  it('treats denied as an old desktop, pauses with unwatch and resumes with watch', async () => {
    const denied = setup(async () => {
      throw new RemoteCommandError('denied')
    })
    await denied.feed.watch()
    expect(denied.feed.current.status).toBe('unsupported')
    const { feed, calls } = setup(async () => ({ gen: 'g', items: [], older: null }))
    await feed.watch()
    feed.pause()
    expect(calls.at(-1)?.type).toBe('chat.unwatch')
    feed.handle({ t: 'chat.items', paneKey: 'p1', gen: 'g', items: [ai('a', 1)] })
    expect(feed.current.log.items.size).toBe(0)
    await feed.resume()
    expect(calls.at(-1)?.type).toBe('chat.watch')
    feed.stop()
    expect(calls.filter((c) => c.type === 'chat.unwatch')).toHaveLength(2)
  })

  it('starts from a cached page at once', () => {
    forgetLogs()
    const log = applyPage(emptyLog(), parseChatPage({ gen: 'g', items: [me('u', 1)], older: null })!, false)
    cacheLog('m', 'p1', log)
    expect(cachedLog('m', 'p1')).toBe(log)
    const feed = new ChatFeed('p1', { command: async () => ({}), onChange: () => {} }, cachedLog('m', 'p1'))
    expect(feed.current.status).toBe('ready')
    expect(feed.current.log.items.size).toBe(1)
    forgetLogs('m')
    expect(cachedLog('m', 'p1')).toBeNull()
  })
})

describe('chat inbox', () => {
  const doc = {
    tabs: [
      {
        key: 'tab1',
        project: 'D:/Code/api',
        groups: [{ project: 'D:/Code/web', name: 'web', panes: ['p2'] }],
        panes: [
          { key: 'p1', kind: 'term', preset: 'claude', title: 'auth fix', rail: 'idle', sessionId: 's1', terminalId: 't1', cwd: 'D:/Code/api' },
          { key: 'p2', kind: 'term', preset: 'codex', title: 'web', rail: 'asked', sessionId: 's2', terminalId: 't2', cwd: 'D:/Code/web' },
          { key: 'p3', kind: 'term', preset: 'shell', title: 'npm run dev', rail: 'working', sessionId: 's3', terminalId: 't3', cwd: 'D:/Code/api' },
          { key: 'f1', kind: 'file', title: 'a.ts' },
        ],
      },
    ],
  }
  const snapshot = {
    terminals: [
      { id: 't1', lastActivityAt: '2026-11-20T08:00:00Z' },
      { id: 't3', lastActivityAt: '2026-11-20T09:00:00Z' },
    ],
    agents: [{ id: 'a1', terminalId: 't1', role: 'main', contextPct: 61.6, model: 'opus', currentTool: 'Edit', currentToolSummary: 'a.ts', currentToolStartedAt: '2026-11-20T08:00:00Z' }],
  }

  it('makes one row per terminal pane with project, last sentence and time, waiting first then newest', () => {
    const tabs = parseWorkspace(doc, [])
    const rows = chatRows(tabs, snapshot, { p1: { text: 'Fixed.', at: Date.parse('2026-11-20T08:30:00Z') } })
    expect(rows.map((r) => r.key)).toEqual(['p1', 'p2', 'p3'])
    expect(rows[0]).toMatchObject({ project: 'api', last: 'Fixed.', canChat: true, at: Date.parse('2026-11-20T08:30:00Z') })
    expect(rows[1]).toMatchObject({ project: 'web', canChat: true, rail: 'asked' })
    expect(rows[2]).toMatchObject({ canChat: false, provider: 'shell' })
    const sorted = sortRows(rows)
    expect(sorted.waiting.map((r) => r.key)).toEqual(['p2'])
    expect(sorted.rest.map((r) => r.key)).toEqual(['p3', 'p1'])
    expect(filterRows(rows, 'FIXED').map((r) => r.key)).toEqual(['p1'])
    expect(liveOf(snapshot, 't1')).toMatchObject({ contextPct: 62, model: 'opus', tool: 'Edit' })
    expect(liveOf(null, 't1').contextPct).toBeNull()
  })

  it('formats short relative times', () => {
    const t = (k: string, v?: Record<string, unknown>) => `${k}:${v?.n ?? ''}`
    expect(shortAgo(30_000, t)).toBe('duration.lessThanMinute:')
    expect(shortAgo(5 * 60_000, t)).toBe('duration.minutes:5')
    expect(shortAgo(3 * 3_600_000, t)).toBe('duration.hours:3')
    expect(shortAgo(50 * 3_600_000, t)).toBe('duration.days:2')
  })

  it('drops a sent message once the transcript shows it, keeps unsent ones', () => {
    const out = [
      { id: 'a', text: 'Run  the tests', state: 'sent' as const, at: 1000 },
      { id: 'b', text: 'later', state: 'unsent' as const, at: 1000 },
      { id: 'c', text: 'gone', state: 'sent' as const, at: 0 },
    ]
    const items = [parseChatItem({ ...me('u', 1, 'Run the tests'), at: 2000 })!]
    expect(settleOutbox(out, items, 5000).map((m) => m.id)).toEqual(['b', 'c'])
    expect(settleOutbox(out, items, 70_000).map((m) => m.id)).toEqual(['b'])
  })
})

describe('MachineData chat.last', () => {
  it('keeps the last agent sentence per pane', () => {
    const data = new MachineData({ machineId: 'm', commander: async () => ({}) })
    data.handleEvent({ t: 'chat.last', full: true, items: [{ paneKey: 'p1', text: 'Done', at: 5 }] })
    expect(data.view.chatLast).toEqual({ p1: { text: 'Done', at: 5 } })
    data.handleEvent({ t: 'chat.last', items: [{ paneKey: 'p1', text: null }] })
    expect(data.view.chatLast).toEqual({})
  })
})
