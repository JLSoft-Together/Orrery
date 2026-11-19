import { describe, expect, it } from 'vitest'
import { RemoteCommandError } from '../remote/types'
import { canCompact, cleanTabName, closePane, compactPane, deniedAfterHello, errorCode, movePane, moveTargets, renameTab, reopenPane, viewOnlyFrom } from './paneActions'
import { closedCount, parseWorkspace } from './workspaceModel'

const doc = {
  closedCount: 2,
  tabs: [
    { key: 'a', name: 'Mine', project: 'D:/code/api', panes: [{ key: 'p1', kind: 'term', preset: 'claude', title: 'One', rail: 'idle', sessionId: 's1', terminalId: 't1' }, { key: 'p2', kind: 'term', preset: 'shell', title: 'Sh', rail: 'idle', sessionId: 's2', terminalId: 't2' }, { key: 'p3', kind: 'file', title: 'f.ts', rail: 'idle' }] },
    { key: 'b', project: 'D:/code/web', panes: [] },
  ],
}

function recorder(reply: Record<string, unknown> = { ok: true }) {
  const calls: Array<[string, Record<string, unknown> | undefined]> = []
  return { calls, run: async (type: string, fields?: Record<string, unknown>) => (calls.push([type, fields]), reply) as never }
}

describe('pane actions model', () => {
  const tabs = parseWorkspace(doc, [])

  it('keeps the custom tab name apart from the derived one and counts closed panes', () => {
    expect(tabs.map((t) => [t.name, t.customName])).toEqual([['Mine', 'Mine'], ['web', null]])
    expect(closedCount(doc)).toBe(2)
    expect(closedCount({ tabs: [] })).toBe(0)
    expect(closedCount(null)).toBe(0)
  })

  it('offers compact only for agent terminals and moves only to other tabs', () => {
    const [p1, p2, p3] = tabs[0]!.panes
    expect([canCompact(p1!), canCompact(p2!), canCompact(p3!)]).toEqual([true, false, false])
    expect(moveTargets(tabs, 'a').map((t) => t.key)).toEqual(['b'])
  })

  it('trims tab names and turns an empty one into null (default name)', () => {
    expect(cleanTabName('  Ads  ')).toBe('Ads')
    expect(cleanTabName('   ')).toBeNull()
    expect(cleanTabName('x'.repeat(150))).toHaveLength(100)
  })

  it('sends the same fields as the Android app', async () => {
    const r = recorder()
    await renameTab(r.run, 'a', ' New ')
    await renameTab(r.run, 'a', '')
    await movePane(r.run, 'p1', 'b')
    await compactPane(r.run, 'p1')
    await reopenPane(r.run)
    expect(r.calls).toEqual([
      ['ws.rename', { tabKey: 'a', name: 'New' }],
      ['ws.rename', { tabKey: 'a', name: null }],
      ['ws.move', { paneKey: 'p1', tabKey: 'b' }],
      ['ws.compact', { paneKey: 'p1' }],
      ['ws.reopen', undefined],
    ])
  })

  it('tells closed, asked (confirm on the phone) and cancelled apart', async () => {
    expect(await closePane(recorder({ ok: true }).run, 'p1')).toEqual({ closed: true, asked: false })
    expect(await closePane(recorder({ ok: true, confirm: 'c1' }).run, 'p1')).toEqual({ closed: false, asked: true })
    expect(await closePane(recorder({ ok: true, cancelled: true }).run, 'p1')).toEqual({ closed: false, asked: false })
  })

  it('maps errors to codes', () => {
    expect(errorCode(new RemoteCommandError('denied'))).toBe('denied')
    expect(errorCode(new Error('x'))).toBe('internal')
  })
})

describe('view only from hello', () => {
  it('is view only when hello says control is off or a command was denied', () => {
    expect(viewOnlyFrom({ control: false }, false)).toBe(true)
    expect(viewOnlyFrom({ control: true }, false)).toBe(false)
    expect(viewOnlyFrom({}, false)).toBe(false)
    expect(viewOnlyFrom(null, false)).toBe(false)
    expect(viewOnlyFrom({ control: true }, true)).toBe(true)
  })

  it('forgets an old denial only when a new hello says control is on', () => {
    const old = { control: false }
    expect(deniedAfterHello(old, old, true)).toBe(true)
    expect(deniedAfterHello(old, { control: true }, true)).toBe(false)
    expect(deniedAfterHello(old, { control: false }, true)).toBe(true)
    expect(deniedAfterHello(old, { name: 'x' }, true)).toBe(false)
    expect(deniedAfterHello(null, { control: true }, false)).toBe(false)
  })
})
