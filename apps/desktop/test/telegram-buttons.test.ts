import { mkdtempSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it } from 'vitest'
import { parseInbound, parseInboundResult, parseMcpDeskCall, parseTelegramButtons, type TelegramInbound, type TelegramInboundResult } from '@ccm/shared'
import { parseMcpDeskResult, type McpDeskResult } from '../../../packages/shared/src/mcpDesk'
import { handleMcp, type ToolHost } from '../src/pty/mcp'
import { TelegramNotifier } from '../src/telegram/notifier'
import { ReplyButtons, REPLY_GROUPS_MAX, REPLY_TTL_MS } from '../src/telegram/replyButtons'
import type { TgFetch } from '../src/telegram/api'

const crypt = { available: () => true, encrypt: (s: string) => Buffer.from(`sealed:${s}`), decrypt: (b: Buffer) => b.toString().replace(/^sealed:/, '') }
const TOKEN = '123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsawQ'
const CHAT = '-1001234567890'
const OWNER = 555
const MEMBER = 556
const PANE = 'pane-1'
const project = path.resolve(os.tmpdir(), 'tg-buttons', 'A')
const YES_NO = [
  { label: 'Yes', value: 'yes, go on' },
  { label: 'No', value: 'no, stop' },
]

type Sent = { method: string; body: Record<string, unknown> }

function setup(answer: (req: TelegramInbound) => TelegramInboundResult, opts: { owner?: boolean; now?: () => number } = {}) {
  const sent: Sent[] = []
  const asked: TelegramInbound[] = []
  let polls = 0
  let nextId = 100
  const fetch: TgFetch = async (url, init) => {
    const method = url.split('/').pop()!
    if (method === 'getUpdates') {
      polls++
      return new Promise(() => undefined)
    }
    const body = typeof init.body === 'string' ? (JSON.parse(init.body) as Record<string, unknown>) : {}
    sent.push({ method, body })
    const result = method === 'getMe' ? { id: 42, username: 'orrery_bot' } : method === 'sendMessage' ? { message_id: nextId++ } : true
    return { ok: true, status: 200, json: async () => ({ ok: true, result }) }
  }
  const dir = mkdtempSync(path.join(os.tmpdir(), 'tg-buttons-'))
  const file = path.join(dir, 'telegram.json')
  writeFileSync(
    file,
    JSON.stringify({
      enabled: true,
      token: crypt.encrypt(TOKEN).toString('base64'),
      routes: [{ project, label: 'A', chatId: CHAT, threadId: 7 }],
      members: [{ id: MEMBER, name: 'Mem', level: 'full' }],
      ...(opts.owner === false ? {} : { owner: { id: OWNER, name: 'Chủ' } }),
    }),
  )
  const tg = new TelegramNotifier({
    file,
    crypt,
    fetch,
    locale: () => 'en',
    home: '',
    platform: 'win32',
    sleep: async () => undefined,
    ...(opts.now ? { now: opts.now } : {}),
    askPage: async (req) => {
      asked.push(req)
      return answer(req)
    },
  })
  return { tg, sent, asked, polls: () => polls }
}

const tap = (data: string, from = OWNER, id = 'cb') => ({ update_id: 2, callback_query: { id, from: { id: from }, data, message: { message_id: 100 } } })
const keysOf = (sent: Sent[]) => ((sent.find((s) => s.method === 'sendMessage')!.body.reply_markup as { inline_keyboard: { text: string; callback_data: string }[][] }).inline_keyboard.flat())
const answers = (sent: Sent[]) => sent.filter((s) => s.method === 'answerCallbackQuery').map((s) => String(s.body.text ?? ''))
const edits = (sent: Sent[]) => sent.filter((s) => s.method === 'editMessageReplyMarkup').map((s) => (s.body.reply_markup as { inline_keyboard: { text: string }[][] }).inline_keyboard.flat().map((b) => b.text))
const idle = (submitted: boolean) => (req: TelegramInbound): TelegramInboundResult => (req.action === 'answer' ? { ok: true, action: 'answer', name: 'A', submitted } : { ok: false, error: 'unavailable' })

describe('ReplyButtons store', () => {
  it('one tap per message, then already answered', () => {
    const store = new ReplyButtons(() => 0)
    const token = store.add({ pane: PANE, project, chatId: CHAT, threadId: 7, buttons: YES_NO })
    expect(Buffer.byteLength(`rb:${token}5`)).toBeLessThanOrEqual(64)
    const first = store.claim(`rb:${token}1`)
    expect(first).toMatchObject({ kind: 'ok', index: 1, button: { label: 'No', value: 'no, stop' } })
    expect(store.claim(`rb:${token}0`)).toEqual({ kind: 'busy' })
    store.finish(token, 'No')
    expect(store.claim(`rb:${token}0`)).toEqual({ kind: 'answered', label: 'No' })
    expect(store.pending()).toBe(false)
  })

  it('release lets the next tap try again; unknown index or token is expired', () => {
    const store = new ReplyButtons(() => 0)
    const token = store.add({ pane: PANE, project, chatId: CHAT, threadId: 7, buttons: YES_NO })
    store.claim(`rb:${token}0`)
    store.release(token)
    expect(store.claim(`rb:${token}0`).kind).toBe('ok')
    expect(store.claim(`rb:${token}5`)).toEqual({ kind: 'expired' })
    expect(store.claim('rb:AAAAAAAAAAAA0')).toEqual({ kind: 'expired' })
    expect(store.claim('ap:abcdefabcdef')).toEqual({ kind: 'expired' })
  })

  it('drops groups after the TTL and keeps at most REPLY_GROUPS_MAX', () => {
    let now = 0
    const store = new ReplyButtons(() => now)
    const old = store.add({ pane: PANE, project, chatId: CHAT, threadId: 7, buttons: YES_NO })
    expect(store.pending()).toBe(true)
    now = REPLY_TTL_MS + 1
    expect(store.claim(`rb:${old}0`)).toEqual({ kind: 'expired' })
    expect(store.pending()).toBe(false)
    const tokens = Array.from({ length: REPLY_GROUPS_MAX + 5 }, () => store.add({ pane: PANE, project, chatId: CHAT, threadId: 7, buttons: YES_NO }))
    expect(store.claim(`rb:${tokens[0]}0`)).toEqual({ kind: 'expired' })
    expect(store.claim(`rb:${tokens.at(-1)}0`).kind).toBe('ok')
  })
})

describe('telegram_send buttons: arguments', () => {
  it('parses count and lengths', () => {
    expect(parseTelegramButtons(undefined)).toBeUndefined()
    expect(parseTelegramButtons(YES_NO)).toEqual(YES_NO)
    expect(parseTelegramButtons([])).toBeNull()
    expect(parseTelegramButtons(Array.from({ length: 7 }, () => ({ label: 'a', value: 'b' })))).toBeNull()
    expect(parseTelegramButtons([{ label: 'x'.repeat(33), value: 'b' }])).toBeNull()
    expect(parseTelegramButtons([{ label: 'x'.repeat(32), value: 'v'.repeat(200) }])).toHaveLength(1)
    expect(parseTelegramButtons([{ label: 'a', value: 'v'.repeat(201) }])).toBeNull()
    expect(parseTelegramButtons([{ label: '  ', value: 'b' }])).toBeNull()
    expect(parseTelegramButtons([{ label: 'a', value: '' }])).toBeNull()
    expect(parseTelegramButtons([{ label: 'a' }])).toBeNull()
    expect(parseTelegramButtons('yes')).toBeNull()
  })

  it('main re-checks them and the host reads back the count', () => {
    const base = { tool: 'telegram_send', id: '1', from: PANE, cwd: project, pid: null, name: 'A', text: 'pick one' }
    expect(parseMcpDeskCall({ ...base, buttons: YES_NO }, 'telegram_send')).toMatchObject({ buttons: YES_NO })
    expect(parseMcpDeskCall({ ...base, buttons: [] }, 'telegram_send')).toBeNull()
    expect(parseMcpDeskResult({ ok: true, tool: 'telegram_send', topic: 'A', buttons: 2 }, 'telegram_send')).toEqual({ ok: true, tool: 'telegram_send', topic: 'A', buttons: 2 })
    expect(parseMcpDeskResult({ ok: true, tool: 'telegram_send', topic: 'A', buttons: 9 }, 'telegram_send')).toEqual({ ok: true, tool: 'telegram_send', topic: 'A' })
  })

  it('the answer action needs a pane target', () => {
    const req = { id: '1', action: 'answer', project, text: 'yes', from: 'Chủ', target: { kind: 'pane', sessionId: PANE } }
    expect(parseInbound(req)).toEqual(req)
    expect(parseInbound({ ...req, target: { kind: 'terminal', terminalId: 't' } })).toBeNull()
    expect(parseInbound({ ...req, text: ' ' })).toBeNull()
    expect(parseInboundResult({ ok: true, action: 'answer', name: 'A', submitted: true })).toEqual({ ok: true, action: 'answer', name: 'A', submitted: true })
  })

  const fail = (): Promise<never> => Promise.reject(new Error('not expected'))
  const run = async (args: unknown, desk: (c: unknown) => Promise<McpDeskResult>) => {
    const host: ToolHost = { root: project, preset: 'claude', take: () => true, openSession: fail, notify: fail, page: fail, checkoutRoots: () => Promise.resolve([]), desk: desk as ToolHost['desk'] }
    const res = await handleMcp({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'telegram_send', arguments: args } }, host, '0')
    return (res.json as { result: { content: { text: string }[]; structuredContent: Record<string, unknown>; isError?: boolean } }).result
  }

  it('the tool refuses bad buttons and buttons with a file, and reports the count', async () => {
    const calls: unknown[] = []
    const desk = async (c: unknown): Promise<McpDeskResult> => {
      calls.push(c)
      return { ok: true, tool: 'telegram_send', topic: 'A', buttons: 2 }
    }
    expect((await run({ text: 'x', buttons: [{ label: '', value: 'v' }] }, desk)).structuredContent).toEqual({ ok: false, error: 'buttons' })
    expect((await run({ text: 'x', file: 'a.txt', buttons: YES_NO }, desk)).isError).toBe(true)
    expect(calls).toHaveLength(0)
    const ok = await run({ text: 'Ship it?', buttons: YES_NO }, desk)
    expect(calls[0]).toEqual({ tool: 'telegram_send', text: 'Ship it?', buttons: YES_NO })
    expect(ok.structuredContent).toEqual({ ok: true, tool: 'telegram_send', topic: 'A', buttons: 2 })
    expect(ok.content[0]!.text).toContain('2 buttons')
  })
})

describe('telegram_send buttons: taps', () => {
  it('sends an inline keyboard of tokens only and needs a paired owner', async () => {
    const off = setup(idle(true), { owner: false })
    expect(await off.tg.report(project, 'Ship it?', undefined, PANE, YES_NO)).toEqual({ ok: false, error: 'not-paired' })
    const { tg, sent } = setup(idle(true))
    expect(await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)).toMatchObject({ ok: true, buttons: 2 })
    const keys = keysOf(sent)
    expect(keys.map((k) => k.text)).toEqual(['Yes', 'No'])
    for (const k of keys) {
      expect(k.callback_data).toMatch(/^rb:[A-Za-z0-9_-]{12}[0-5]$/)
      expect(k.callback_data).not.toContain('yes')
    }
  })

  it('keeps reading updates for pending buttons with receive mode off', async () => {
    const { tg, polls } = setup(idle(true))
    expect(tg.state().receiving).toBe(false)
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    await new Promise((r) => setTimeout(r, 10))
    expect(polls()).toBeGreaterThan(0)
  })

  it('owner tap on an idle pane: typed and submitted, keyboard replaced by ✓ label, later taps already answered', async () => {
    const { tg, sent, asked } = setup(idle(true))
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    const [, no] = keysOf(sent)
    await tg.onUpdate(tap(no!.callback_data))
    expect(asked).toEqual([expect.objectContaining({ action: 'answer', project, text: 'no, stop', target: { kind: 'pane', sessionId: PANE }, from: 'Chủ' })])
    expect(answers(sent).at(-1)).toContain('sent')
    expect(edits(sent)).toEqual([['✓ No']])
    await tg.onUpdate(tap(keysOf(sent)[0]!.callback_data, OWNER, 'cb2'))
    expect(asked).toHaveLength(1)
    expect(answers(sent).at(-1)).toBe('Already answered: No')
  })

  it('busy pane: only typed, the toast says so', async () => {
    const { tg, sent } = setup(idle(false))
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    await tg.onUpdate(tap(keysOf(sent)[0]!.callback_data))
    expect(answers(sent).at(-1)).toContain('not submitted')
  })

  it('a pane with a prompt waiting keeps the buttons for another try', async () => {
    let busy = true
    const { tg, sent, asked } = setup(() => (busy ? { ok: false, error: 'busy' } : { ok: true, action: 'answer', name: 'A', submitted: true }))
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    const data = keysOf(sent)[0]!.callback_data
    await tg.onUpdate(tap(data))
    expect(answers(sent).at(-1)).toContain('prompt waiting')
    expect(edits(sent)).toEqual([])
    busy = false
    await tg.onUpdate(tap(data))
    expect(asked).toHaveLength(2)
    expect(edits(sent)).toEqual([['✓ Yes']])
  })

  it('pane gone: nothing typed, session closed, keyboard removed', async () => {
    const { tg, sent } = setup(() => ({ ok: false, error: 'gone' }))
    await tg.onUpdate(tap('rb:AAAAAAAAAAAA0'))
    expect(answers(sent).at(-1)).toBe('This button is no longer valid.')
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    const data = keysOf(sent)[0]!.callback_data
    await tg.onUpdate(tap(data))
    expect(answers(sent).at(-1)).toBe('That session is closed; nothing was typed.')
    expect(edits(sent)).toEqual([[]])
    await tg.onUpdate(tap(data))
    expect(answers(sent).at(-1)).toBe('This button is no longer valid.')
  })

  it('only the paired owner can tap: strangers and members are refused and nothing is typed', async () => {
    const { tg, sent, asked } = setup(idle(true))
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    const data = keysOf(sent)[0]!.callback_data
    await tg.onUpdate(tap(data, 999))
    await tg.onUpdate(tap(data, MEMBER))
    expect(asked).toEqual([])
    expect(answers(sent)).toEqual(['Only the paired account can use these buttons.', 'Only the paired account can use these buttons.'])
    await tg.onUpdate(tap(data))
    expect(asked).toHaveLength(1)
  })

  it('buttons expire after 24 h', async () => {
    let now = 1_000_000
    const { tg, sent, asked } = setup(idle(true), { now: () => now })
    await tg.report(project, 'Ship it?', undefined, PANE, YES_NO)
    now += REPLY_TTL_MS + 1
    await tg.onUpdate(tap(keysOf(sent)[0]!.callback_data))
    expect(asked).toEqual([])
    expect(answers(sent).at(-1)).toBe('This button is no longer valid.')
  })
})
