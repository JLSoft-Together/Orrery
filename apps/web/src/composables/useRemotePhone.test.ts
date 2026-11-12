import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { PANE_LIVE_MAX, type PtyPreset } from '@ccm/shared'

describe('phone commands (ws.* with source phone)', () => {
  let ws: Awaited<ReturnType<typeof boot>>['ws']
  let call: Awaited<ReturnType<typeof boot>>['call']
  let ui: Awaited<ReturnType<typeof boot>>['ui']
  let remote: Awaited<ReturnType<typeof boot>>['remote']
  let busyKeys = new Set<string>()
  const compactStart = vi.fn((_key: string) => true)

  let orreryApi: typeof import('./useOrreryApi')

  async function boot() {
    vi.resetModules()
    setActivePinia(createPinia())
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { platform: 'win32', onOrrery: () => undefined }
    vi.doMock('./useCompact', () => ({ useCompact: () => ({ start: compactStart }) }))
    vi.doMock('./usePaneInfo', () => ({ usePaneInfo: () => ({ isBusy: (p: { key: string }) => busyKeys.has(p.key) }) }))
    const phone = await import('./useRemotePhone')
    const orrery = await import('./useOrreryApi')
    orreryApi = orrery
    const store = await import('../stores/workspace')
    const uiStore = await import('../stores/ui')
    const remoteStore = await import('../stores/remote')
    phone.installRemotePhone()
    const call = (action: string, cmd: Record<string, unknown>, extra: Record<string, unknown> = {}) =>
      orrery.runOrrery(action, { device: 'Pixel', cmd: { t: action, ...cmd }, ...extra }, { source: 'phone' }) as Promise<Record<string, unknown>>
    return { ws: store.useWorkspaceStore(), call, ui: uiStore.useUiStore(), remote: remoteStore.useRemoteStore() }
  }

  function pane(preset: PtyPreset, cwd: string, id: string) {
    const p = ws.open(preset, cwd)
    ws.update(p.key, { sessionId: `pty-${id}`, terminalId: `term-${id}`, state: 'running' })
    return ws.panes.find((x) => x.key === p.key)!
  }

  beforeEach(async () => {
    busyKeys = new Set()
    compactStart.mockReset().mockReturnValue(true)
    localStorage.clear()
    sessionStorage.clear()
    ;({ ws, call, ui, remote } = await boot())
  })

  it('a command typed from the phone marks a shell pane as busy like desktop Enter does', async () => {
    const sh = pane('shell', 'C:\\work\\B', 'sh')
    const ag = pane('claude', 'C:\\work\\A', 'ag')
    const run = (sessionId: string, source = 'phone') => orreryApi.runOrrery('remote.typed', { device: 'Pixel', sessionId }, { source }) as Promise<Record<string, unknown>>
    expect(sh.ranCommand).toBeUndefined()
    expect(await run('pty-sh')).toMatchObject({ ok: true })
    expect(ws.panes.find((p) => p.key === sh.key)?.ranCommand).toBe(true)
    expect(await run('pty-ag')).toMatchObject({ ok: true })
    expect(ws.panes.find((p) => p.key === ag.key)?.ranCommand).toBeUndefined()
    expect(await run('pty-none')).toMatchObject({ ok: false })
    expect(await run('pty-sh', 'mcp')).toMatchObject({ ok: false })
  })

  it('a session rename sent from the phone lands in the remote log', async () => {
    const run = (args: unknown, source = 'phone') => orreryApi.runOrrery('remote.logged', args, { source }) as Promise<Record<string, unknown>>
    expect(await run({ kind: 'alias', device: 'Pixel', alias: 'API work' })).toMatchObject({ ok: true })
    expect(remote.log[0]?.text).toContain('API work')
    expect(await run({ kind: 'alias', device: 'Pixel', alias: '' })).toMatchObject({ ok: true })
    expect(remote.log).toHaveLength(2)
    expect(await run({ kind: 'alias', device: 'Pixel', alias: 'x' }, 'mcp')).toMatchObject({ ok: false })
    expect(await run({ kind: 'other', device: 'Pixel' })).toMatchObject({ ok: false })
  })

  it('open docks into the project tab, keeps the desktop focus and the view, and only submits on request', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a')
    const b = pane('shell', 'C:\\work\\B', 'b')
    ui.setView('monitor')
    expect(ws.activeKey).toBe(b.key)
    const typed = await call('ws.open', { cwd: 'C:\\work\\A', preset: 'codex', prompt: 'hello' })
    expect(typed).toMatchObject({ ok: true })
    const typedPane = ws.panes.find((p) => p.key === typed.paneKey)!
    expect(typedPane).toMatchObject({ preset: 'codex', prefill: 'hello' })
    expect(typedPane.prefillSubmit).toBeUndefined()
    expect(ws.tabOf(typedPane.key)).toBe(ws.tabOf(a.key))
    expect(ws.activeKey).toBe(b.key)
    expect(ui.view).toBe('monitor')
    const sent = await call('ws.open', { cwd: 'C:\\work\\A', preset: 'claude', prompt: 'go', submit: true })
    expect(ws.panes.find((p) => p.key === sent.paneKey)).toMatchObject({ prefill: 'go', prefillSubmit: true })
    expect(ws.activeKey).toBe(b.key)
  })

  it('open refuses past the live-pane limit and a bad session id', async () => {
    for (let i = 0; i < PANE_LIVE_MAX; i++) pane('shell', `C:\\work\\p${i}`, String(i))
    expect(await call('ws.open', { cwd: 'C:\\work\\A', preset: 'claude' })).toEqual({ ok: false, error: 'limit' })
    expect(await call('ws.open', { cwd: 'C:\\work\\A', preset: 'claude', resume: 'nope' })).toEqual({ ok: false, error: 'args' })
  })

  it('close of a busy pane asks the phone and never opens the desktop sheet; the confirmed call closes it', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a')
    busyKeys.add(a.key)
    const ask = await call('ws.close', { paneKey: a.key })
    expect(ask).toMatchObject({ needsConfirm: { ok: expect.any(String), cancel: expect.any(String), title: expect.any(String) } })
    expect(ws.pendingClose).toBeNull()
    expect(ws.panes.some((p) => p.key === a.key)).toBe(true)
    expect(await call('ws.close', { paneKey: a.key }, { confirmed: true })).toEqual({ ok: true })
    expect(ws.pendingClose).toBeNull()
    expect(ws.panes.some((p) => p.key === a.key)).toBe(false)
    expect(ws.closedStack).toHaveLength(1)
  })

  it('close of an idle pane goes straight through, and a missing pane is notFound', async () => {
    const a = pane('shell', 'C:\\work\\A', 'a')
    expect(await call('ws.close', { paneKey: a.key })).toEqual({ ok: true })
    expect(ws.pendingClose).toBeNull()
    expect(await call('ws.close', { paneKey: a.key })).toEqual({ ok: false, error: 'notFound' })
  })

  it('close drops the pane from a desktop sheet that was already open', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a')
    const b = pane('claude', 'C:\\work\\B', 'b')
    busyKeys.add(a.key).add(b.key)
    ws.requestClose([a.key, b.key], (p) => busyKeys.has(p.key))
    expect(ws.pendingClose).toEqual([a.key, b.key])
    await call('ws.close', { paneKey: a.key }, { confirmed: true })
    expect(ws.pendingClose).toEqual([b.key])
    await call('ws.close', { paneKey: b.key }, { confirmed: true })
    expect(ws.pendingClose).toBeNull()
  })

  it('a repeated gateway rid runs once and returns the first result', async () => {
    const { useTerminalsStore } = await import('../stores/terminals')
    const a = pane('claude', 'C:\\work\\A', 'a')
    useTerminalsStore().byId['term-a'] = { id: 'term-a', status: 'idle' } as never
    const first = await call('ws.send', { paneKey: a.key, text: 'go' }, { rid: 'n1.61' })
    expect(first).toEqual({ ok: true, submitted: true })
    ws.update(a.key, { prefill: undefined, prefillSubmit: undefined })
    expect(await call('ws.send', { paneKey: a.key, text: 'go' }, { rid: 'n1.61' })).toEqual(first)
    expect(a.prefill).toBeUndefined()
    expect(await call('ws.send', { paneKey: a.key, text: 'go' }, { rid: 'n1.62' })).toEqual({ ok: true, submitted: true })
  })

  it('keeps at most the newest rids and forgets them after a minute', async () => {
    const { onceByRid } = await import('./useRemotePhone')
    const { REMOTE_RID_KEEP, REMOTE_RID_TTL_MS } = await import('@ccm/shared')
    const recent = new Map()
    let runs = 0
    const run = () => ({ ok: true as const, n: ++runs })
    for (let i = 0; i <= REMOTE_RID_KEEP; i++) await onceByRid(recent, `r${i}`, run, 1000)
    expect(recent.size).toBe(REMOTE_RID_KEEP)
    await onceByRid(recent, 'r0', run, 1000)
    expect(runs).toBe(REMOTE_RID_KEEP + 2)
    await onceByRid(recent, 'r5', run, 1000)
    expect(runs).toBe(REMOTE_RID_KEEP + 2)
    await onceByRid(recent, 'r5', run, 1000 + REMOTE_RID_TTL_MS)
    expect(runs).toBe(REMOTE_RID_KEEP + 3)
  })

  it('rename and move change state only', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a')
    const b = pane('claude', 'C:\\work\\B', 'b')
    const tabA = ws.tabOf(a.key)!
    const tabB = ws.tabOf(b.key)!
    expect(await call('ws.rename', { tabKey: tabA.key, name: 'Work' })).toEqual({ ok: true })
    expect(tabA.name).toBe('Work')
    expect(await call('ws.rename', { tabKey: 'nope', name: 'x' })).toEqual({ ok: false, error: 'notFound' })
    expect(await call('ws.move', { paneKey: b.key, tabKey: tabA.key })).toEqual({ ok: true })
    expect(ws.tabOf(b.key)).toBe(tabA)
    expect(ws.tabs.includes(tabB)).toBe(false)
    expect(await call('ws.move', { paneKey: 'nope', tabKey: tabA.key })).toEqual({ ok: false, error: 'notFound' })
  })

  it('send submits into an idle agent and only types into a busy one; text over the limit is refused', async () => {
    const { useTerminalsStore } = await import('../stores/terminals')
    const a = pane('claude', 'C:\\work\\A', 'a')
    const b = pane('codex', 'C:\\work\\B', 'b')
    useTerminalsStore().byId['term-a'] = { id: 'term-a', status: 'idle' } as never
    useTerminalsStore().byId['term-b'] = { id: 'term-b', status: 'working' } as never
    expect(await call('ws.send', { paneKey: a.key, text: 'go' })).toEqual({ ok: true, submitted: true })
    expect(a.prefill).toBe('go')
    expect(a.prefillSubmit).toBe(true)
    expect(await call('ws.send', { paneKey: b.key, text: 'wait' })).toEqual({ ok: true, submitted: false })
    expect(b.prefillSubmit).toBeUndefined()
    expect(await call('ws.send', { paneKey: a.key, text: 'again' })).toEqual({ ok: false, error: 'busy' })
    expect(await call('ws.send', { paneKey: b.key, text: 'x'.repeat(2001) })).toEqual({ ok: false, error: 'args' })
    expect(await call('ws.send', { paneKey: 'gone', text: 'x' })).toEqual({ ok: false, error: 'notFound' })
  })

  it('keys refuses a press whose prompt moved on', async () => {
    const { useTerminalsStore } = await import('../stores/terminals')
    const a = pane('claude', 'C:\\work\\A', 'a')
    const at = '2026-11-04T10:00:00.000Z'
    useTerminalsStore().byId['term-a'] = { id: 'term-a', status: 'waiting', statusSince: at, waitingFor: 'permission prompt' } as never
    expect(await call('ws.keys', { paneKey: a.key, keys: 'approve', since: Date.parse(at) + 1 })).toEqual({ ok: false, error: 'busy' })
    expect(a.typeKeys).toBeUndefined()
    expect(await call('ws.keys', { paneKey: a.key, keys: 'rm -rf', since: 1 })).toEqual({ ok: false, error: 'args' })
  })

  it('reopen brings the last closed pane back without moving the desktop focus', async () => {
    const a = pane('shell', 'C:\\work\\A', 'a')
    pane('shell', 'C:\\work\\B', 'b')
    ws.requestClose([a.key], () => false)
    const before = ws.activeKey
    const out = await call('ws.reopen', {})
    expect(out).toMatchObject({ ok: true })
    expect(ws.activeKey).toBe(before)
    expect(await call('ws.reopen', {})).toEqual({ ok: false, error: 'notFound' })
  })

  it('compact starts the flow on an agent pane, and refuses a shell or a pane with a pending prompt', async () => {
    const a = pane('claude', 'C:\\work\\A', 'a')
    const s = pane('shell', 'C:\\work\\S', 's')
    expect(await call('ws.compact', { paneKey: a.key })).toEqual({ ok: true })
    expect(compactStart).toHaveBeenCalledWith(a.key)
    expect(await call('ws.compact', { paneKey: s.key })).toEqual({ ok: false, error: 'notFound' })
    ws.update(a.key, { prefill: 'pending' })
    expect(await call('ws.compact', { paneKey: a.key })).toEqual({ ok: false, error: 'busy' })
  })

  it('every command writes one line to the phone log, and only phone callers may send them', async () => {
    const a = pane('shell', 'C:\\work\\A', 'a')
    await call('ws.close', { paneKey: a.key })
    expect(remote.log).toHaveLength(1)
    expect(remote.log[0]).toMatchObject({ device: 'Pixel', ok: true })
    const orrery = await import('./useOrreryApi')
    expect(await orrery.runOrrery('ws.reopen', { device: 'x', cmd: { t: 'ws.reopen' } }, { source: 'telegram' })).toEqual({ ok: false, error: 'source' })
    expect(await orrery.runOrrery('ws.reopen', { device: 'x', cmd: { t: 'ws.reopen' } }, { source: 'mcp' })).toEqual({ ok: false, error: 'source' })
  })
})
