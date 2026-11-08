import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { TELEGRAM_INBOUND_ACTION, type PtyPreset, type TelegramInbound, type TelegramInboundResult } from '@ccm/shared'

type Handler = (raw: unknown) => Promise<TelegramInboundResult>

describe('Telegram reply button (session.answer)', () => {
  let handler: Handler
  let ws: Awaited<ReturnType<typeof boot>>['ws']
  let terminals: Awaited<ReturnType<typeof boot>>['terminals']

  async function boot() {
    vi.resetModules()
    setActivePinia(createPinia())
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { platform: 'win32', onOrrery: () => undefined, telegramPanes: async () => true }
    const mod = await import('./useTelegramInbound')
    const orrery = await import('./useOrreryApi')
    const captured: Handler = (raw) => orrery.runOrrery(TELEGRAM_INBOUND_ACTION[(raw as TelegramInbound).action], raw, { source: 'telegram' }) as Promise<TelegramInboundResult>
    const store = await import('../stores/workspace')
    const terms = await import('../stores/terminals')
    mod.installTelegramInbound()
    return { handler: captured, ws: store.useWorkspaceStore(), terminals: terms.useTerminalsStore() }
  }

  function pane(preset: PtyPreset, cwd: string, id: string, status?: string) {
    const p = ws.open(preset, cwd)
    ws.update(p.key, { sessionId: `pty-${id}`, terminalId: `term-${id}`, state: 'running' })
    if (status) terminals.byId[`term-${id}`] = { id: `term-${id}`, status, title: '' } as never
    return ws.panes.find((x) => x.key === p.key)!
  }

  const req = (sessionId: string, text = 'yes, go on') => ({ id: '1', action: 'answer', project: 'C:\\work\\A', text, from: 'Owner', target: { kind: 'pane', sessionId } })

  beforeEach(async () => {
    localStorage.clear()
    sessionStorage.clear()
    ;({ handler, ws, terminals } = await boot())
  })

  it('idle pane: typed and submitted', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a', 'idle')
    expect(await handler(req('pty-a'))).toEqual({ ok: true, action: 'answer', name: expect.any(String), submitted: true })
    expect(a.prefill).toBe('yes, go on')
    expect(a.prefillSubmit).toBe(true)
  })

  it('working, waiting or unknown pane: only typed, never submitted', async () => {
    const w = pane('claude', 'C:\\work\\A', 'w', 'working')
    const q = pane('codex', 'C:\\work\\A', 'q', 'waiting')
    const u = pane('claude', 'C:\\work\\A', 'u')
    for (const [p, id] of [[w, 'w'], [q, 'q'], [u, 'u']] as const) {
      expect(await handler(req(`pty-${id}`))).toMatchObject({ ok: true, action: 'answer', submitted: false })
      expect(p.prefill).toBe('yes, go on')
      expect(p.prefillSubmit).toBeUndefined()
    }
  })

  it('cleans the value and refuses a pane with a prompt already waiting', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a', 'idle')
    await handler(req('pty-a', ' line\x07 one\r\nline two '))
    expect(a.prefill).toBe('line one\nline two')
    expect(await handler(req('pty-a', 'again'))).toMatchObject({ ok: false, error: 'busy' })
    expect(a.prefill).toBe('line one\nline two')
  })

  it('gone: closed pane, shell pane or unknown session, nothing typed', async () => {
    const s = pane('shell', 'C:\\work\\A', 's', 'idle')
    const x = pane('claude', 'C:\\work\\A', 'x', 'idle')
    ws.update(x.key, { state: 'exited' })
    expect(await handler(req('pty-s'))).toEqual({ ok: false, error: 'gone' })
    expect(await handler(req('pty-x'))).toEqual({ ok: false, error: 'gone' })
    expect(await handler(req('pty-zz'))).toEqual({ ok: false, error: 'gone' })
    expect(s.prefill).toBeUndefined()
    expect(x.prefill).toBeUndefined()
  })

  it('works for a pane that moved out of the project folder (bound by session id)', async () => {
    const a = pane('claude', 'C:\\elsewhere', 'a', 'idle')
    expect(await handler(req('pty-a'))).toMatchObject({ ok: true, submitted: true })
    expect(a.prefill).toBe('yes, go on')
  })
})
